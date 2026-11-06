//! Builds the prompts the murloc scribe writes from: a day, or a week as one chapter.
//!
//! The prompt is grounded: the beats are the only events the model may tell, quest text is given
//! for colour, earlier days come as memory only, and the narrator profile sets the voice. Tone
//! rules are explicit constraints, since the voice is the product: never mock the player, and the
//! murloc is seasoning, not the meal. Whether the murloc meddles with a margin note is rolled from
//! the character and the day, so regenerating never changes it.

use std::collections::BTreeMap;

use chrono::{Datelike, NaiveDate};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::beats::{self, DayRecord, Thresholds};
use crate::continuity::ContinuitySheet;
use crate::milestones::{self, Award};
use crate::narrators::Narrator;
use crate::schema::Journal;

/// Words a day entry aims for.
pub const DAY_WORDS: u32 = 120;
/// Words a weekly chapter aims for.
pub const WEEK_WORDS: u32 = 250;
/// Earlier days remembered in a day prompt.
const MEMORY_DAYS: usize = 2;
/// Beats remembered from each earlier day.
const MEMORY_BEATS: usize = 3;
/// Quest text beyond this is cut, to keep small models' context for the day itself.
const QUEST_TEXT_CHARS: usize = 500;

/// How often the murloc asks for a margin note.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase")]
pub enum Meddling {
    Off,
    Rare,
    #[default]
    Sometimes,
    Often,
}

impl Meddling {
    pub fn percent(self) -> u64 {
        match self {
            Meddling::Off => 0,
            Meddling::Rare => 5,
            Meddling::Sometimes => 15,
            Meddling::Often => 35,
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Meddling::Off => "off",
            Meddling::Rare => "rare",
            Meddling::Sometimes => "sometimes",
            Meddling::Often => "often",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        [Self::Off, Self::Rare, Self::Sometimes, Self::Often]
            .into_iter()
            .find(|m| m.key() == key)
    }
}

fn sha256_hex(text: &str) -> String {
    Sha256::digest(text.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// The golden ratio's fractional part: stepping by it spreads points evenly over [0, 1).
const GOLDEN: f64 = 0.618_033_988_749_894_9;

/// Whether the murloc meddles in the entry for `slot` (a day, or `week:` and its Monday).
///
/// Independent random rolls would drift: 15% of 100 days can come out as 22. Instead each
/// character gets a fixed offset, from a hash that does not change between builds, and day `n`
/// meddles when `offset + n × golden ratio` (mod 1) falls below the chance. That sequence is
/// spread evenly, so any run of days lands close to the chosen share, and the answer for a day
/// never changes on regeneration.
pub fn meddles(character_id: i64, slot: &str, meddling: Meddling) -> bool {
    let (salt, day) = match slot.strip_prefix("week:") {
        Some(monday) => ("week", monday),
        None => ("day", slot),
    };
    let digest = Sha256::digest(format!("meddling:{character_id}:{salt}").as_bytes());
    let offset =
        u64::from_be_bytes(digest[..8].try_into().expect("8 bytes")) as f64 / u64::MAX as f64;
    let n = match NaiveDate::parse_from_str(day, "%Y-%m-%d") {
        Ok(d) => f64::from(d.num_days_from_ce()),
        // Not a date: fall back to a fixed point derived from the slot itself.
        Err(_) => f64::from(Sha256::digest(slot.as_bytes())[0]),
    };
    let point = (offset + n * GOLDEN).fract();
    point < meddling.percent() as f64 / 100.0
}

/// The language's English name, for the prompt.
pub fn language_name(code: &str) -> &str {
    match code.split(['-', '_']).next().unwrap_or(code) {
        "en" => "English",
        "fr" => "French",
        "de" => "German",
        "es" => "Spanish",
        "it" => "Italian",
        "pt" => "Portuguese",
        _ => code,
    }
}

/// A companion's real name and the stand-in sent instead.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Alias {
    pub real: String,
    pub alias: String,
}

/// Stand-in names for companions, for endpoints the player would rather not send names to.
const PSEUDONYMS: &[&str] = &[
    "Fennick", "Marlow", "Juniper", "Corwin", "Sable", "Thistle", "Bramble", "Odette", "Harrow",
    "Linnet", "Quill", "Pip", "Ember", "Rook", "Willa", "Moss",
];

fn aliases(names: &[String]) -> Vec<Alias> {
    let mut taken: Vec<&str> = Vec::new();
    names
        .iter()
        .map(|real| {
            let digest = Sha256::digest(real.as_bytes());
            let start = digest[0] as usize % PSEUDONYMS.len();
            let alias = (0..PSEUDONYMS.len())
                .map(|i| PSEUDONYMS[(start + i) % PSEUDONYMS.len()])
                .find(|a| !taken.contains(a) && !names.iter().any(|n| n == a))
                .unwrap_or("Someone");
            taken.push(alias);
            Alias {
                real: real.clone(),
                alias: alias.to_string(),
            }
        })
        .collect()
}

fn replace_word(text: &str, from: &str, to: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find(from) {
        let before = rest[..at].chars().next_back();
        let after = rest[at + from.len()..].chars().next();
        let bounded =
            !before.is_some_and(char::is_alphanumeric) && !after.is_some_and(char::is_alphanumeric);
        out.push_str(&rest[..at]);
        out.push_str(if bounded { to } else { from });
        rest = &rest[at + from.len()..];
    }
    out.push_str(rest);
    out
}

/// Puts the real names back into text written with stand-ins.
pub fn restore_names(text: &str, aliases: &[Alias]) -> String {
    aliases.iter().fold(text.to_string(), |acc, a| {
        replace_word(&acc, &a.alias, &a.real)
    })
}

/// A prompt ready to send.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Prompt {
    pub system: String,
    pub user: String,
    /// Identifies the exact prompt a story was written from.
    pub hash: String,
    /// Whether this prompt asks for a murloc margin note.
    pub meddling: bool,
    /// Companions sent under stand-in names, if any.
    pub aliases: Vec<Alias>,
}

impl Prompt {
    fn new(
        system: String,
        user: String,
        meddling: bool,
        names: &[String],
        pseudonyms: bool,
    ) -> Self {
        let present: Vec<String> = names
            .iter()
            .filter(|n| user.contains(n.as_str()))
            .cloned()
            .collect();
        let (user, aliases) = if pseudonyms && !present.is_empty() {
            let aliases = aliases(&present);
            let user = aliases
                .iter()
                .fold(user, |acc, a| replace_word(&acc, &a.real, &a.alias));
            (user, aliases)
        } else {
            (user, Vec::new())
        };
        Self {
            hash: sha256_hex(&format!("{system}\u{0}{user}")),
            system,
            user,
            meddling,
            aliases,
        }
    }
}

/// What a day prompt is built from.
pub struct DayRequest<'a> {
    pub character_id: i64,
    pub journal: &'a Journal,
    pub day: &'a str,
    pub awards: &'a [Award],
    /// Running jokes from the character's stored continuity sheet.
    pub running_jokes: &'a [String],
    pub narrator: &'a Narrator,
    /// The prose language, e.g. `"fr"`.
    pub language: &'a str,
    pub meddling: Meddling,
    /// Send companions under stand-in names.
    pub pseudonyms: bool,
    pub thresholds: &'a Thresholds,
}

/// What a weekly chapter is built from.
pub struct WeekRequest<'a> {
    pub character_id: i64,
    pub journal: &'a Journal,
    /// The week's Monday.
    pub start_day: &'a str,
    pub awards: &'a [Award],
    pub running_jokes: &'a [String],
    /// Entries already written for days of the week, by day.
    pub entries: &'a BTreeMap<String, String>,
    pub narrator: &'a Narrator,
    pub language: &'a str,
    pub meddling: Meddling,
    pub pseudonyms: bool,
    pub thresholds: &'a Thresholds,
}

#[derive(Clone, Copy)]
enum Kind {
    Day,
    Week,
}

fn system(narrator: &Narrator, language: &str, meddling: bool, kind: Kind) -> String {
    let (events, when, length) = match kind {
        Kind::Day => (
            "What happened today",
            "today",
            format!("One or two short paragraphs, about {DAY_WORDS} words"),
        ),
        Kind::Week => (
            "The week, day by day",
            "this week",
            format!("Three short paragraphs at most, about {WEEK_WORDS} words"),
        ),
    };
    let murloc = if meddling {
        narrator.margin_note.trim()
    } else {
        "Do not add a footnote or margin note."
    };
    let length = if meddling {
        format!("{length}, then the margin note")
    } else {
        length
    };
    let mut example = format!(
        "Notes: {}\nEntry:\n{}",
        narrator.example.notes.trim(),
        narrator.example.entry.trim()
    );
    if meddling {
        example = format!("{example}\n{}", narrator.example.margin_note.trim());
    }
    format!(
        "{persona}\n\n\
         Rules, in order of importance:\n\
         1. Facts. Every event must come from \"{events}\". Do not add events, people, places, \
         items, dialogue or numbers, and do not invent lore, history or places beyond what is \
         given. You may describe how things felt, the light, the mood, but nothing that \
         happened. Keep every number and name exactly as given.\n\
         2. Earlier days are memory only. Never describe them as happening {when}.\n\
         3. Voice. {voice} Never mock the adventurer: the comedy comes from the narrator, from \
         the patterns of the day and from the murloc, never at the adventurer's expense.\n\
         4. The murloc is seasoning, not the meal. {murloc_rule} {murloc}\n\
         5. Length. {length}. Output only the entry: no title, no greeting, no signature.\n\
         6. Language. Write in {language}. Keep names of places, creatures, items, quests and \
         people exactly as given, untranslated.\n\n\
         Example of the expected shape, for a different day (do not reuse its facts):\n\n\
         {example}",
        persona = narrator.persona.trim(),
        voice = narrator.voice.trim(),
        murloc_rule = narrator.murloc.trim(),
        language = language_name(language),
    )
}

fn title_case(word: &str) -> String {
    let lower = word.to_lowercase();
    let mut chars = lower.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

fn dated(day: &str) -> String {
    match NaiveDate::parse_from_str(day, "%Y-%m-%d") {
        Ok(d) => format!("{}, {day}", d.format("%A")),
        Err(_) => day.to_string(),
    }
}

fn adventurer(journal: &Journal, level: Option<(i64, i64)>) -> String {
    let c = &journal.character;
    let name = [c.name.as_deref(), c.surname.as_deref()]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" ");
    let who = [c.race.as_deref(), c.class.as_deref()]
        .into_iter()
        .flatten()
        .map(title_case)
        .collect::<Vec<_>>()
        .join(" ");
    let mut line = format!(
        "Adventurer: {}",
        if name.is_empty() { "unnamed" } else { &name }
    );
    if !who.is_empty() {
        line = format!("{line}, {who}");
    }
    match level {
        Some((from, to)) if to > from => format!("{line}, level {from} (reached level {to})."),
        Some((from, _)) => format!("{line}, level {from}."),
        None => format!("{line}."),
    }
}

fn bullets(lines: &[String]) -> String {
    if lines.is_empty() {
        "- none".to_string()
    } else {
        lines
            .iter()
            .map(|l| format!("- {l}"))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

fn quest_lines(journal: &Journal, record: &DayRecord) -> Vec<String> {
    let mut lines = Vec::new();
    let mut ids: Vec<i64> = Vec::new();
    for id in record.accepted.iter().chain(&record.turned_in) {
        if !ids.contains(id) {
            ids.push(*id);
        }
    }
    for id in &ids {
        let what = match (record.accepted.contains(id), record.turned_in.contains(id)) {
            (true, true) => "accepted and turned in",
            (true, false) => "accepted",
            _ => "turned in",
        };
        let quest = journal.quests.get(id);
        let title = quest
            .and_then(|q| q.title.clone())
            .unwrap_or_else(|| format!("Quest {id}"));
        let text = quest.and_then(|q| q.text.as_deref()).map(|t| {
            let t = t.trim();
            match t.char_indices().nth(QUEST_TEXT_CHARS) {
                Some((cut, _)) => format!("{}...", &t[..cut]),
                None => t.to_string(),
            }
        });
        lines.push(match text {
            Some(t) if !t.is_empty() => format!("\"{title}\" ({what}): {t}"),
            _ => format!("\"{title}\" ({what})"),
        });
    }
    lines
}

fn hint_lines(narrator: &Narrator, patterns: &[&str]) -> Vec<String> {
    patterns
        .iter()
        .filter_map(|p| narrator.hints.get(*p).cloned())
        .collect()
}

/// Names that may be swapped for stand-ins: companions of the day and the recurring ones.
fn companion_names(records: &[&DayRecord], sheet: &ContinuitySheet) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    for name in records
        .iter()
        .flat_map(|r| r.companions.iter())
        .chain(sheet.companions.iter())
    {
        if !names.contains(name) {
            names.push(name.clone());
        }
    }
    names
}

/// The prompt for one day's entry.
pub fn day_prompt(r: &DayRequest) -> Prompt {
    let record = r
        .journal
        .days
        .get(r.day)
        .map(DayRecord::of)
        .unwrap_or_default();
    let day_beats = beats::extract(r.journal, r.day, r.awards, r.thresholds);
    let patterns: Vec<&str> = day_beats.patterns.iter().map(|p| p.key()).collect();
    let sheet = ContinuitySheet::compute(r.journal, r.awards, r.running_jokes, Some(r.day));
    let meddling = meddles(r.character_id, r.day, r.meddling);

    let mut memory = sheet.memory_lines();
    let earlier: Vec<&String> = r
        .journal
        .days
        .range(..r.day.to_string())
        .rev()
        .take(MEMORY_DAYS)
        .map(|(d, _)| d)
        .collect();
    for d in earlier.into_iter().rev() {
        let texts = beats::extract(r.journal, d, r.awards, r.thresholds).texts();
        if !texts.is_empty() {
            let kept: Vec<String> = texts.into_iter().take(MEMORY_BEATS).collect();
            memory.push(format!("{}: {}", dated(d), kept.join("; ")));
        }
    }

    let milestones_today: Vec<String> = r
        .awards
        .iter()
        .filter(|a| a.day == r.day)
        .map(|a| milestones::name(&a.id).to_string())
        .collect();
    let hints = hint_lines(r.narrator, &patterns);
    let level = record.level_from.zip(record.level_to);

    let mut user = format!(
        "Notes for {date}.\n\n{adventurer}\n\n\
         What happened today (the only events you may write about):\n{beats}\n\n\
         Patterns of the day: {patterns}\n",
        date = dated(r.day),
        adventurer = adventurer(r.journal, level),
        beats = bullets(&day_beats.texts()),
        patterns = if patterns.is_empty() {
            "none".to_string()
        } else {
            patterns.join(", ").replace('_', " ")
        },
    );
    if !hints.is_empty() {
        user.push_str(&format!(
            "How to tell a day like this:\n{}\n",
            bullets(&hints)
        ));
    }
    user.push_str(&format!(
        "\nMilestones stamped today: {milestones}\n\n\
         Quests touched today:\n{quests}\n\n\
         Earlier days (memory only, not today):\n{memory}\n\n\
         Write today's entry in {language}, about {DAY_WORDS} words.",
        milestones = if milestones_today.is_empty() {
            "none".to_string()
        } else {
            milestones_today.join(", ")
        },
        quests = bullets(&quest_lines(r.journal, &record)),
        memory = bullets(&memory),
        language = language_name(r.language),
    ));

    Prompt::new(
        system(r.narrator, r.language, meddling, Kind::Day),
        user,
        meddling,
        &companion_names(&[&record], &sheet),
        r.pseudonyms,
    )
}

/// The seven days starting on `monday`.
pub fn week_days(monday: &str) -> Vec<String> {
    let Ok(start) = NaiveDate::parse_from_str(monday, "%Y-%m-%d") else {
        return Vec::new();
    };
    start
        .iter_days()
        .take(7)
        .map(|d| d.format("%Y-%m-%d").to_string())
        .collect()
}

/// The Monday of the week holding `day`.
pub fn week_start(day: &str) -> Option<String> {
    let d = NaiveDate::parse_from_str(day, "%Y-%m-%d").ok()?;
    let monday = d - chrono::Days::new(u64::from(d.weekday().num_days_from_monday()));
    Some(monday.format("%Y-%m-%d").to_string())
}

/// The prompt for a weekly chapter: written from the week's entries where they exist and from
/// the beats otherwise.
pub fn week_prompt(r: &WeekRequest) -> Prompt {
    let days = week_days(r.start_day);
    let sheet = ContinuitySheet::compute(r.journal, r.awards, r.running_jokes, Some(r.start_day));
    let slot = format!("week:{}", r.start_day);
    let meddling = meddles(r.character_id, &slot, r.meddling);

    let mut blocks = Vec::new();
    let mut records = Vec::new();
    let mut quests = Vec::new();
    let mut patterns: Vec<&str> = Vec::new();
    for d in &days {
        let Some(day) = r.journal.days.get(d) else {
            continue;
        };
        let record = DayRecord::of(day);
        for q in quest_lines(r.journal, &record) {
            if !quests.contains(&q) {
                quests.push(q);
            }
        }
        let day_beats = beats::extract(r.journal, d, r.awards, r.thresholds);
        for p in &day_beats.patterns {
            if !patterns.contains(&p.key()) {
                patterns.push(p.key());
            }
        }
        blocks.push(match r.entries.get(d) {
            Some(entry) => format!(
                "{} (the entry written that day):\n{}",
                dated(d),
                entry.trim()
            ),
            None => format!("{}:\n{}", dated(d), bullets(&day_beats.texts())),
        });
        records.push(record);
    }
    let level = records
        .iter()
        .filter_map(|r| r.level_from)
        .min()
        .zip(records.iter().filter_map(|r| r.level_to).max());
    let milestones_week: Vec<String> = r
        .awards
        .iter()
        .filter(|a| days.contains(&a.day))
        .map(|a| milestones::name(&a.id).to_string())
        .collect();
    let end = days.last().cloned().unwrap_or_default();
    let user = format!(
        "Notes for the week of {from} to {to}.\n\n{adventurer}\n\n\
         The week, day by day (the only events you may write about):\n\n{blocks}\n\n\
         Patterns of the week: {patterns}\n\n\
         Milestones stamped this week: {milestones}\n\n\
         Quests touched this week:\n{quests}\n\n\
         Before this week (memory only):\n{memory}\n\n\
         Write this week's chapter in {language}, about {WEEK_WORDS} words.",
        from = dated(r.start_day),
        to = dated(&end),
        adventurer = adventurer(r.journal, level),
        blocks = if blocks.is_empty() {
            "- none".to_string()
        } else {
            blocks.join("\n\n")
        },
        patterns = if patterns.is_empty() {
            "none".to_string()
        } else {
            patterns.join(", ").replace('_', " ")
        },
        milestones = if milestones_week.is_empty() {
            "none".to_string()
        } else {
            milestones_week.join(", ")
        },
        quests = bullets(&quests),
        memory = bullets(&sheet.memory_lines()),
        language = language_name(r.language),
    );
    let record_refs: Vec<&DayRecord> = records.iter().collect();
    Prompt::new(
        system(r.narrator, r.language, meddling, Kind::Week),
        user,
        meddling,
        &companion_names(&record_refs, &sheet),
        r.pseudonyms,
    )
}

/// A second attempt after the fact check found claims the notes do not support: the same
/// prompt, with the first entry and the claims named.
pub fn rewrite_prompt(original: &Prompt, entry: &str, claims: &[String]) -> Prompt {
    let user = format!(
        "{}\n\nA first draft was written:\n\n{}\n\n\
         It mentions things that are not in the notes: {}. Rewrite the entry so that every \
         event, name and number comes from the notes above. Keep the voice.",
        original.user,
        entry.trim(),
        claims.join(", "),
    );
    Prompt {
        hash: sha256_hex(&format!("{}\u{0}{user}", original.system)),
        system: original.system.clone(),
        user,
        meddling: original.meddling,
        aliases: original.aliases.clone(),
    }
}
