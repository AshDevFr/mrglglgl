//! Typed journal and settings, read from parsed SavedVariables as `docs/schema.md` describes them.
//!
//! Reading is tolerant by design: unknown fields are ignored, an unknown event kind is kept as
//! `EventKind::Unknown`, and a field of the wrong type reads as absent. A newer addon adds fields
//! without changing the schema number, and the app must keep working when it does. Only three
//! things are refused, each for one character at a time: a file that does not parse, a file with
//! no journal in it, and a schema number outside what this app understands.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::sv::{self, Key, ParseError, SavedVariables, Table, Value};

/// The newest journal schema this app reads.
pub const SUPPORTED_SCHEMA: i64 = 1;

/// The oldest journal schema this app reads. Schema 0 never shipped; it exists to exercise the
/// addon's migration, and a journal still at 0 is upgraded by logging in once with the addon.
pub const OLDEST_SCHEMA: i64 = 1;

const JOURNAL_VARIABLE: &str = "MrglglglCharDB";
const SETTINGS_VARIABLE: &str = "MrglglglDB";

#[derive(Debug, Clone, PartialEq)]
pub enum JournalError {
    Parse(ParseError),
    /// The file has no journal variable: another addon's file, or one the addon never wrote to.
    NoJournal,
    /// Written by an older addon; logging in once with a current addon upgrades it.
    OlderSchema {
        found: i64,
    },
    /// Written by a newer addon than this app understands; updating the app fixes it.
    NewerSchema {
        found: i64,
        supported: i64,
    },
}

impl fmt::Display for JournalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JournalError::Parse(e) => write!(f, "the journal file could not be read: {e}"),
            JournalError::NoJournal => write!(f, "the file holds no Mrglglgl journal"),
            JournalError::OlderSchema { found } => {
                write!(
                    f,
                    "older journal format ({found}): log in once with the addon to upgrade it"
                )
            }
            JournalError::NewerSchema { found, supported } => write!(
                f,
                "made by a newer addon version (journal format {found}, this app reads up to {supported}): update the app"
            ),
        }
    }
}

impl std::error::Error for JournalError {}

impl From<ParseError> for JournalError {
    fn from(e: ParseError) -> Self {
        JournalError::Parse(e)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct Journal {
    pub schema: i64,
    pub character: Character,
    /// Keyed by local calendar date, `YYYY-MM-DD`, so iteration is in date order.
    pub days: BTreeMap<String, Day>,
    pub quests: BTreeMap<i64, Quest>,
    pub companions: BTreeMap<String, Companion>,
    pub discovered: BTreeSet<String>,
    /// Every Legacy Challenge (achievement) completed, by ID, including those from before the
    /// addon was installed.
    #[serde(default)]
    pub challenges: BTreeMap<i64, Challenge>,
    pub stats: Stats,
    /// Which addon and client last wrote the journal, and when.
    pub addon: Option<AddonHeartbeat>,
    /// Errors the addon caught in its handlers; recording carried on.
    pub errors: Option<AddonErrors>,
}

/// A Legacy Challenge (achievement) completed, and the day the game says it was.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct Challenge {
    pub name: Option<String>,
    pub category: Option<String>,
    /// `YYYY-MM-DD`.
    pub day: Option<String>,
}

/// What the addon notes at each login.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct AddonHeartbeat {
    /// The addon's packaged version, e.g. `addon-v0.1.0`.
    pub version: Option<String>,
    pub build: Option<String>,
    pub interface: Option<i64>,
    /// Unix time of the last login.
    pub login: Option<i64>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct AddonErrors {
    /// All time.
    pub count: i64,
    pub last_message: Option<String>,
    pub last_event: Option<String>,
    pub last_time: Option<i64>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct Character {
    pub name: Option<String>,
    pub surname: Option<String>,
    pub realm: Option<String>,
    pub realm_id: Option<i64>,
    pub race: Option<String>,
    pub class: Option<String>,
    pub faction: Option<String>,
    /// The language stored names and quest text are in.
    pub locale: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct Stats {
    pub boots: i64,
    pub murlocs_slain: i64,
    pub deaths_by_murloc: i64,
    pub drownings: i64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase")]
// Days are archived as JSON and read back by later builds: a field added since falls back to
// its default instead of failing the whole journal. The same holds for every archived record.
#[serde(default)]
pub struct Day {
    /// Raw events in time order; `None` once the addon compacted the day.
    pub events: Option<Vec<Event>>,
    pub kills: BTreeMap<i64, KillAggregate>,
    pub anon_kills: i64,
    /// Profession skills that rose that day, by skill ID.
    pub skills: BTreeMap<i64, SkillRecord>,
    /// Items crafted that day, by item ID.
    pub crafted: BTreeMap<i64, ItemCount>,
    /// Items gathered from nodes and corpses that day, by item ID.
    pub gathered: BTreeMap<i64, ItemCount>,
    /// Money earned and spent that day, by source.
    pub money: Money,
    /// Chat lines in the channels the player chose, in time order.
    pub chat: Vec<ChatLine>,
    /// Seconds grouped with each companion that day, keyed as in the journal's companions.
    pub grouped: BTreeMap<String, i64>,
    pub digest: Option<Digest>,
    /// The game compacted the day: `events` holds only its key moments, and the digest is what
    /// the whole day was.
    pub compacted: bool,
}

/// A line said in a chat channel.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase")]
pub struct ChatLine {
    /// Unix time.
    pub time: i64,
    /// `say`, `emote`, `textEmote`, `yell`, `party`, `raid`, `guild`, `officer`, `whisper` or
    /// `channel`.
    pub channel: String,
    pub text: String,
    /// The speaker; none for the player's own lines. Other players' words are theirs: they stay
    /// out of the website, exports and anything sent to a narrator.
    pub who: Option<String>,
    /// For a whisper the player sent: who it went to.
    pub to: Option<String>,
    /// For a numbered channel: its name, e.g. "General".
    pub channel_name: Option<String>,
}

impl ChatLine {
    pub fn own(&self) -> bool {
        self.who.is_none()
    }
}

/// Money earned (`inc`) and spent (`out`) over a day, in copper, by where it came from or went:
/// `loot`, `quest`, `vendor`, `repair`, `trainer`, `travel`, `mail`, `auction`, `trade` or
/// `other`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(default)]
pub struct Money {
    pub inc: BTreeMap<String, i64>,
    pub out: BTreeMap<String, i64>,
}

impl Money {
    pub fn earned(&self) -> i64 {
        self.inc.values().sum()
    }

    pub fn spent(&self) -> i64 {
        self.out.values().sum()
    }

    pub fn is_empty(&self) -> bool {
        self.inc.is_empty() && self.out.is_empty()
    }
}

/// A skill's rise over one day: the first rank seen and the last.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase")]
pub struct SkillRecord {
    pub name: Option<String>,
    /// The heading the skill sits under in the game's skills list, e.g. "Professions".
    pub category: Option<String>,
    pub from: i64,
    pub to: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase")]
pub struct ItemCount {
    pub name: Option<String>,
    pub count: i64,
    /// For gathered items: the gathering skill used, e.g. "Mining".
    pub via: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase")]
pub struct Event {
    /// Unix time.
    pub time: i64,
    pub zone: Option<String>,
    pub subzone: Option<String>,
    pub map_id: Option<i64>,
    pub level: Option<i64>,
    /// The balance in copper, on session events when money is recorded.
    #[serde(default)]
    pub balance: Option<i64>,
    pub kind: EventKind,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum EventKind {
    SessionStart,
    SessionEnd,
    Level,
    Zone,
    Subzone,
    Discover {
        area: Option<String>,
        /// `"message"` or `"visit"`.
        source: Option<String>,
    },
    Death {
        killer: Option<Killer>,
        /// The environmental cause, e.g. `"Drowning"`.
        environment: Option<String>,
        /// On a hardcore character, a death that ends it.
        #[serde(default)]
        final_death: bool,
    },
    Boss {
        encounter_id: Option<i64>,
        name: Option<String>,
    },
    QuestAccept {
        quest_id: Option<i64>,
    },
    QuestTurnin {
        quest_id: Option<i64>,
        xp: Option<i64>,
        money: Option<i64>,
    },
    Fish {
        item_id: Option<i64>,
        name: Option<String>,
        junk: Option<bool>,
        boot: Option<bool>,
    },
    Loot {
        item_id: Option<i64>,
        name: Option<String>,
        quality: Option<i64>,
        quantity: Option<i64>,
        /// Read from a group loot window (`src = "window"`): it dropped, not necessarily for
        /// the player. Defaults for events archived before it existed.
        #[serde(default)]
        seen: bool,
    },
    GroupJoin {
        who: Option<String>,
    },
    Screenshot {
        moment: Option<String>,
    },
    /// A spell or ability learned; druid forms are spells.
    Spell {
        spell_id: Option<i64>,
        name: Option<String>,
    },
    Mount {
        name: Option<String>,
    },
    Pet {
        name: Option<String>,
    },
    /// A reputation standing that rose.
    Reputation {
        faction: Option<String>,
        /// The client's label, e.g. "Friendly".
        standing: Option<String>,
    },
    /// An item put on with a higher item level than its slot held.
    Gear {
        item_id: Option<i64>,
        name: Option<String>,
        slot: Option<i64>,
        item_level: Option<i64>,
        quality: Option<i64>,
    },
    /// The hearthstone set here.
    Bind {
        area: Option<String>,
    },
    /// A new flight path learned here.
    FlightPath {
        area: Option<String>,
    },
    Duel {
        won: Option<bool>,
        /// Another player's name: theirs, kept out of what is shared.
        opponent: Option<String>,
    },
    /// Entering (`act = "enter"`) or leaving (`"leave"`) a dungeon or raid.
    Instance {
        act: Option<String>,
        name: Option<String>,
        /// `"party"` or `"raid"`.
        instance_kind: Option<String>,
        /// Seconds inside, on leaving.
        duration: Option<i64>,
    },
    /// An achievement completed; on Forever, a Legacy Challenge.
    Achievement {
        achievement_id: Option<i64>,
        name: Option<String>,
        /// Its category, e.g. "Kalimdor" or "Druid".
        category: Option<String>,
        points: Option<i64>,
    },
    /// At the auction house.
    Auction {
        /// `"listed"`, `"sold"`, `"won"`, `"outbid"`, `"expired"` or `"cancelled"`.
        act: Option<String>,
        /// The item; absent for `"listed"`, whose message does not name it.
        name: Option<String>,
    },
    /// A kind added by a newer addon, kept so the day still shows it happened.
    Unknown {
        name: String,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase")]
pub struct Killer {
    pub name: Option<String>,
    pub npc_id: Option<i64>,
    pub murloc: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase")]
pub struct KillAggregate {
    pub count: i64,
    pub name: Option<String>,
    /// `"normal"`, `"elite"`, `"rare"`, `"rareelite"`, `"worldboss"`, `"trivial"`, `"minus"` or
    /// `"unknown"`.
    pub classification: String,
    pub murloc: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct Quest {
    pub title: Option<String>,
    pub text: Option<String>,
    /// Day the quest was last accepted or turned in; its text is dropped in game once this falls
    /// outside the retention window.
    pub last: Option<String>,
    /// The quest's level, from the quest log when it was accepted.
    pub level: Option<i64>,
    /// `"elite"`, `"dungeon"`, `"raid"` or `"group"`; none for an ordinary quest.
    pub kind: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct Companion {
    pub first: Option<String>,
    pub last: Option<String>,
    pub days: i64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct Digest {
    pub version: i64,
    pub counts: DigestCounts,
    pub level_from: Option<i64>,
    pub level_to: Option<i64>,
    pub zones: Vec<String>,
    pub subzones: Vec<String>,
    pub discovered: Vec<String>,
    pub accepted: Vec<i64>,
    pub turned_in: Vec<i64>,
    pub deaths: Vec<DigestDeath>,
    pub kills: Vec<DigestKill>,
    pub notable_kills: Vec<DigestKill>,
    pub fish: Vec<DigestFish>,
    pub loot: Vec<DigestLoot>,
    pub companions: Vec<String>,
    pub bosses: Vec<String>,
    pub skills: Vec<DigestSkill>,
    pub crafted: Vec<DigestItem>,
    pub gathered: Vec<DigestItem>,
    pub money: Money,
    pub auctions: Vec<DigestAuction>,
    pub achievements: Vec<DigestAchievement>,
    /// The day's first and last known balance, in copper.
    pub bal_start: Option<i64>,
    pub bal_end: Option<i64>,
    pub only_fished: bool,
    pub nothing_happened: bool,
    /// Older than 90 days: the lists hold only the highlights; counts are exact.
    pub thin: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct DigestCounts {
    pub sessions: i64,
    pub levels: i64,
    pub accepted: i64,
    pub turned_in: i64,
    pub deaths: i64,
    pub fish: i64,
    pub junk: i64,
    pub boots: i64,
    pub loot: i64,
    pub discoveries: i64,
    pub bosses: i64,
    pub group: i64,
    pub screenshots: i64,
    /// All kills that day, anonymous ones included.
    pub kills: i64,
    pub murlocs: i64,
    pub anon_kills: i64,
    /// Ranks gained across all skills.
    pub skill_ups: i64,
    pub crafted: i64,
    pub gathered: i64,
    /// Copper earned and spent.
    pub money_in: i64,
    pub money_out: i64,
    pub auctions: i64,
    /// Chat lines recorded.
    pub chat: i64,
    pub achievements: i64,
    pub spells: i64,
    /// Mounts and pets added.
    pub collections: i64,
    pub reputation: i64,
    pub gear: i64,
    /// Hearthstone set and flight paths learned.
    pub travel: i64,
    pub duels: i64,
    /// Dungeons and raids entered.
    pub instances: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase")]
pub struct DigestAchievement {
    pub achievement_id: Option<i64>,
    pub name: Option<String>,
    pub category: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase")]
pub struct DigestAuction {
    pub act: Option<String>,
    pub name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase")]
pub struct DigestSkill {
    pub skill_id: Option<i64>,
    pub name: Option<String>,
    pub category: Option<String>,
    pub from: i64,
    pub to: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase")]
pub struct DigestItem {
    pub item_id: Option<i64>,
    pub name: Option<String>,
    pub count: i64,
    pub via: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase")]
pub struct DigestDeath {
    pub killer: Option<Killer>,
    pub environment: Option<String>,
    pub zone: Option<String>,
    pub subzone: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase")]
pub struct DigestKill {
    pub npc_id: Option<i64>,
    pub name: Option<String>,
    pub count: i64,
    pub classification: String,
    pub murloc: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase")]
pub struct DigestFish {
    pub item_id: Option<i64>,
    pub name: Option<String>,
    pub count: i64,
    pub junk: Option<bool>,
    pub boot: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase")]
pub struct DigestLoot {
    pub item_id: Option<i64>,
    pub name: Option<String>,
    pub quality: Option<i64>,
    pub quantity: Option<i64>,
    /// Seen in a group loot window rather than received.
    #[serde(default)]
    pub seen: bool,
}

/// The account-wide settings file. The app shows what the addon records; it never changes it.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase")]
pub struct AccountSettings {
    pub schema: i64,
    /// Recording categories (`sessions`, `kills`, ...) and whether each is on.
    pub record: BTreeMap<String, bool>,
    pub loot_quality: Option<i64>,
    pub retention_days: Option<i64>,
    /// The addon prints one line at login about the last day with something in it.
    #[serde(default)]
    pub login_summary: bool,
    /// Screenshot moments (`levelUp`, ...) and whether each is on.
    pub screenshots: BTreeMap<String, bool>,
    /// Chat channels and whether each is recorded: `say` for the player's own lines,
    /// `sayOthers` for other players'.
    #[serde(default)]
    pub chat: BTreeMap<String, bool>,
    /// The addon has said once, at a first login, that it records.
    #[serde(default)]
    pub welcomed: bool,
}

// ── Reading ──────────────────────────────────────────────────────────────────

fn string(t: &Table, key: &str) -> Option<String> {
    t.get(key).and_then(Value::as_str).map(str::to_string)
}

fn int(t: &Table, key: &str) -> Option<i64> {
    t.get(key).and_then(Value::as_i64)
}

fn boolean(t: &Table, key: &str) -> Option<bool> {
    t.get(key).and_then(Value::as_bool)
}

fn table<'a>(t: &'a Table, key: &str) -> Option<&'a Table> {
    t.get(key).and_then(Value::as_table)
}

fn strings(t: &Table, key: &str) -> Vec<String> {
    table(t, key)
        .map(|list| {
            list.sequence()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn ints(t: &Table, key: &str) -> Vec<i64> {
    table(t, key)
        .map(|list| list.sequence().filter_map(Value::as_i64).collect())
        .unwrap_or_default()
}

fn records<T>(t: &Table, key: &str, read: impl Fn(&Table) -> T) -> Vec<T> {
    table(t, key)
        .map(|list| {
            list.sequence()
                .filter_map(Value::as_table)
                .map(&read)
                .collect()
        })
        .unwrap_or_default()
}

fn flags(t: Option<&Table>) -> BTreeMap<String, bool> {
    t.map(|t| {
        t.iter()
            .filter_map(|(k, v)| match (k, v) {
                (Key::String(name), Value::Bool(on)) => Some((name.clone(), *on)),
                _ => None,
            })
            .collect()
    })
    .unwrap_or_default()
}

fn int_key(key: &Key) -> Option<i64> {
    match key {
        Key::Integer(i) => Some(*i),
        Key::String(s) => s.parse().ok(),
        _ => None,
    }
}

fn killer(t: &Table) -> Killer {
    Killer {
        name: string(t, "name"),
        npc_id: int(t, "npc"),
        murloc: boolean(t, "murloc").unwrap_or(false),
    }
}

fn event(t: &Table) -> Option<Event> {
    let kind_name = string(t, "k")?;
    let time = t.get("t").and_then(Value::as_f64)? as i64;
    let kind = match kind_name.as_str() {
        "session_start" => EventKind::SessionStart,
        "session_end" => EventKind::SessionEnd,
        "level" => EventKind::Level,
        "zone" => EventKind::Zone,
        "subzone" => EventKind::Subzone,
        "discover" => EventKind::Discover {
            area: string(t, "area"),
            source: string(t, "src"),
        },
        "death" => EventKind::Death {
            killer: table(t, "killer").map(killer),
            environment: string(t, "env"),
            final_death: boolean(t, "final").unwrap_or(false),
        },
        "boss" => EventKind::Boss {
            encounter_id: int(t, "enc"),
            name: string(t, "name"),
        },
        "quest_accept" => EventKind::QuestAccept {
            quest_id: int(t, "q"),
        },
        "quest_turnin" => EventKind::QuestTurnin {
            quest_id: int(t, "q"),
            xp: int(t, "xp"),
            money: int(t, "money"),
        },
        "fish" => EventKind::Fish {
            item_id: int(t, "item"),
            name: string(t, "name"),
            junk: boolean(t, "junk"),
            boot: boolean(t, "boot"),
        },
        "loot" => EventKind::Loot {
            item_id: int(t, "item"),
            name: string(t, "name"),
            quality: int(t, "ql"),
            quantity: int(t, "qty"),
            seen: string(t, "src").as_deref() == Some("window"),
        },
        "group_join" => EventKind::GroupJoin {
            who: string(t, "who"),
        },
        "screenshot" => EventKind::Screenshot {
            moment: string(t, "moment"),
        },
        "spell" => EventKind::Spell {
            spell_id: int(t, "spell"),
            name: string(t, "name"),
        },
        "mount" => EventKind::Mount {
            name: string(t, "name"),
        },
        "pet" => EventKind::Pet {
            name: string(t, "name"),
        },
        "rep" => EventKind::Reputation {
            faction: string(t, "faction"),
            standing: string(t, "standing"),
        },
        "gear" => EventKind::Gear {
            item_id: int(t, "item"),
            name: string(t, "name"),
            slot: int(t, "slot"),
            item_level: int(t, "ilvl"),
            quality: int(t, "ql"),
        },
        "bind" => EventKind::Bind {
            area: string(t, "area"),
        },
        "flight_path" => EventKind::FlightPath {
            area: string(t, "area"),
        },
        "duel" => EventKind::Duel {
            won: boolean(t, "won"),
            opponent: string(t, "who"),
        },
        "instance" => EventKind::Instance {
            act: string(t, "act"),
            name: string(t, "name"),
            instance_kind: string(t, "kind"),
            duration: int(t, "dur"),
        },
        "achievement" => EventKind::Achievement {
            achievement_id: int(t, "ach"),
            name: string(t, "name"),
            category: string(t, "cat"),
            points: int(t, "pts"),
        },
        "auction" => EventKind::Auction {
            act: string(t, "act"),
            name: string(t, "name"),
        },
        _ => EventKind::Unknown { name: kind_name },
    };
    Some(Event {
        time,
        zone: string(t, "z"),
        subzone: string(t, "sz"),
        map_id: int(t, "m"),
        level: int(t, "lv"),
        balance: int(t, "bal"),
        kind,
    })
}

fn kill_aggregate(t: &Table) -> KillAggregate {
    KillAggregate {
        count: int(t, "n").unwrap_or(0),
        name: string(t, "name"),
        classification: string(t, "cls").unwrap_or_else(|| "unknown".to_string()),
        murloc: boolean(t, "murloc").unwrap_or(false),
    }
}

fn digest_kill(t: &Table) -> DigestKill {
    DigestKill {
        npc_id: int(t, "npc"),
        name: string(t, "name"),
        count: int(t, "n").unwrap_or(0),
        classification: string(t, "cls").unwrap_or_else(|| "unknown".to_string()),
        murloc: boolean(t, "murloc").unwrap_or(false),
    }
}

fn digest(t: &Table) -> Digest {
    let counts = table(t, "counts");
    let count = |key: &str| counts.and_then(|c| int(c, key)).unwrap_or(0);
    Digest {
        version: int(t, "v").unwrap_or(0),
        counts: DigestCounts {
            sessions: count("sessions"),
            levels: count("levels"),
            accepted: count("accepted"),
            turned_in: count("turnedIn"),
            deaths: count("deaths"),
            fish: count("fish"),
            junk: count("junk"),
            boots: count("boots"),
            loot: count("loot"),
            discoveries: count("discoveries"),
            bosses: count("bosses"),
            group: count("group"),
            screenshots: count("screenshots"),
            kills: count("kills"),
            murlocs: count("murlocs"),
            anon_kills: count("anonKills"),
            skill_ups: count("skillUps"),
            crafted: count("crafted"),
            gathered: count("gathered"),
            money_in: count("moneyIn"),
            money_out: count("moneyOut"),
            auctions: count("auctions"),
            chat: count("chat"),
            achievements: count("achievements"),
            spells: count("spells"),
            collections: count("collections"),
            reputation: count("reputation"),
            gear: count("gear"),
            travel: count("travel"),
            duels: count("duels"),
            instances: count("instances"),
        },
        level_from: int(t, "levelFrom"),
        level_to: int(t, "levelTo"),
        zones: strings(t, "zones"),
        subzones: strings(t, "subzones"),
        discovered: strings(t, "discovered"),
        accepted: ints(t, "accepted"),
        turned_in: ints(t, "turnedIn"),
        deaths: records(t, "deaths", |d| DigestDeath {
            killer: table(d, "killer").map(killer),
            environment: string(d, "env"),
            zone: string(d, "z"),
            subzone: string(d, "sz"),
        }),
        kills: records(t, "kills", digest_kill),
        notable_kills: records(t, "notableKills", digest_kill),
        fish: records(t, "fish", |f| DigestFish {
            item_id: int(f, "item"),
            name: string(f, "name"),
            count: int(f, "n").unwrap_or(0),
            junk: boolean(f, "junk"),
            boot: boolean(f, "boot"),
        }),
        loot: records(t, "loot", |l| DigestLoot {
            item_id: int(l, "item"),
            name: string(l, "name"),
            quality: int(l, "ql"),
            quantity: int(l, "qty"),
            seen: string(l, "src").as_deref() == Some("window"),
        }),
        companions: strings(t, "companions"),
        bosses: strings(t, "bosses"),
        skills: records(t, "skills", |r| DigestSkill {
            skill_id: int(r, "skill"),
            name: string(r, "name"),
            category: string(r, "cat"),
            from: int(r, "from").unwrap_or(0),
            to: int(r, "to").unwrap_or(0),
        }),
        crafted: records(t, "crafted", digest_item),
        gathered: records(t, "gathered", digest_item),
        money: table(t, "money").map(money).unwrap_or_default(),
        auctions: records(t, "auctions", |a| DigestAuction {
            act: string(a, "act"),
            name: string(a, "name"),
        }),
        achievements: records(t, "achievements", |a| DigestAchievement {
            achievement_id: int(a, "ach"),
            name: string(a, "name"),
            category: string(a, "cat"),
        }),
        bal_start: int(t, "balStart"),
        bal_end: int(t, "balEnd"),
        only_fished: boolean(t, "onlyFished").unwrap_or(false),
        nothing_happened: boolean(t, "nothingHappened").unwrap_or(false),
        thin: boolean(t, "thin").unwrap_or(false),
    }
}

/// Reads `{ inc = {source = copper}, out = {source = copper} }`.
fn money(t: &Table) -> Money {
    let side = |key: &str| {
        table(t, key)
            .map(|m| {
                m.iter()
                    .filter_map(|(k, v)| match k {
                        Key::String(source) => Some((source.clone(), v.as_i64()?)),
                        _ => None,
                    })
                    .collect()
            })
            .unwrap_or_default()
    };
    Money {
        inc: side("inc"),
        out: side("out"),
    }
}

fn digest_item(t: &Table) -> DigestItem {
    DigestItem {
        item_id: int(t, "item"),
        name: string(t, "name"),
        count: int(t, "n").unwrap_or(0),
        via: string(t, "via"),
    }
}

/// Reads a per-day map keyed by numeric ID, such as `skills` or `crafted`.
fn by_id<T>(t: &Table, key: &str, read: impl Fn(&Table) -> T) -> BTreeMap<i64, T> {
    table(t, key)
        .map(|m| {
            m.iter()
                .filter_map(|(k, v)| Some((int_key(k)?, read(v.as_table()?))))
                .collect()
        })
        .unwrap_or_default()
}

fn item_count(t: &Table) -> ItemCount {
    ItemCount {
        name: string(t, "name"),
        count: int(t, "n").unwrap_or(0),
        via: string(t, "via"),
    }
}

fn day(t: &Table) -> Day {
    Day {
        events: table(t, "ev").map(|ev| {
            ev.sequence()
                .filter_map(Value::as_table)
                .filter_map(event)
                .collect()
        }),
        kills: table(t, "kills")
            .map(|kills| {
                kills
                    .iter()
                    .filter_map(|(k, v)| Some((int_key(k)?, kill_aggregate(v.as_table()?))))
                    .collect()
            })
            .unwrap_or_default(),
        anon_kills: int(t, "anonKills").unwrap_or(0),
        skills: by_id(t, "skills", |s| SkillRecord {
            name: string(s, "name"),
            category: string(s, "cat"),
            from: int(s, "from").unwrap_or(0),
            to: int(s, "to").unwrap_or(0),
        }),
        crafted: by_id(t, "crafted", item_count),
        gathered: by_id(t, "gathered", item_count),
        money: table(t, "money").map(money).unwrap_or_default(),
        chat: records(t, "chat", |c| {
            Some(ChatLine {
                time: c.get("t").and_then(Value::as_f64)? as i64,
                channel: string(c, "ch")?,
                text: string(c, "msg")?,
                who: string(c, "who"),
                to: string(c, "to"),
                channel_name: string(c, "cn"),
            })
        })
        .into_iter()
        .flatten()
        .collect(),
        grouped: table(t, "grp")
            .map(|grp| {
                grp.iter()
                    .filter_map(|(k, v)| match (k, v.as_f64()) {
                        (Key::String(who), Some(seconds)) => Some((who.clone(), seconds as i64)),
                        _ => None,
                    })
                    .collect()
            })
            .unwrap_or_default(),
        digest: table(t, "digest").map(digest),
        compacted: boolean(t, "c").unwrap_or(false),
    }
}

/// Reads the journal from a parsed SavedVariables file.
pub fn journal_from(file: &SavedVariables) -> Result<Journal, JournalError> {
    let db = file
        .get(JOURNAL_VARIABLE)
        .and_then(Value::as_table)
        .ok_or(JournalError::NoJournal)?;
    let schema = int(db, "schema").unwrap_or(0);
    if schema > SUPPORTED_SCHEMA {
        return Err(JournalError::NewerSchema {
            found: schema,
            supported: SUPPORTED_SCHEMA,
        });
    }
    if schema < OLDEST_SCHEMA {
        return Err(JournalError::OlderSchema { found: schema });
    }

    let character = table(db, "char")
        .map(|c| Character {
            name: string(c, "name"),
            surname: string(c, "surname"),
            realm: string(c, "realm"),
            realm_id: int(c, "realmId"),
            race: string(c, "race"),
            class: string(c, "class"),
            faction: string(c, "faction"),
            locale: string(c, "locale"),
        })
        .unwrap_or_default();

    let days = table(db, "days")
        .map(|days| {
            days.iter()
                .filter_map(|(k, v)| match (k, v) {
                    (Key::String(date), Value::Table(t)) => Some((date.clone(), day(t))),
                    _ => None,
                })
                .collect()
        })
        .unwrap_or_default();

    let quests = table(db, "quests")
        .map(|quests| {
            quests
                .iter()
                .filter_map(|(k, v)| {
                    let q = v.as_table()?;
                    Some((
                        int_key(k)?,
                        Quest {
                            title: string(q, "title"),
                            text: string(q, "text"),
                            last: string(q, "last"),
                            level: int(q, "lv"),
                            kind: string(q, "kind"),
                        },
                    ))
                })
                .collect()
        })
        .unwrap_or_default();

    let companions = table(db, "companions")
        .map(|companions| {
            companions
                .iter()
                .filter_map(|(k, v)| match (k, v) {
                    (Key::String(who), Value::Table(c)) => Some((
                        who.clone(),
                        Companion {
                            first: string(c, "first"),
                            last: string(c, "last"),
                            days: int(c, "days").unwrap_or(0),
                        },
                    )),
                    _ => None,
                })
                .collect()
        })
        .unwrap_or_default();

    let discovered = flags(table(db, "discovered"))
        .into_iter()
        .filter_map(|(area, seen)| seen.then_some(area))
        .collect();

    let challenges = by_id(db, "challenges", |c| Challenge {
        name: string(c, "name"),
        category: string(c, "cat"),
        day: string(c, "d"),
    });

    let stats = table(db, "stats")
        .map(|s| Stats {
            boots: int(s, "boots").unwrap_or(0),
            murlocs_slain: int(s, "murlocsSlain").unwrap_or(0),
            deaths_by_murloc: int(s, "deathsByMurloc").unwrap_or(0),
            drownings: int(s, "drownings").unwrap_or(0),
        })
        .unwrap_or_default();

    let addon = table(db, "addon").map(|a| AddonHeartbeat {
        version: string(a, "version"),
        build: string(a, "build"),
        interface: int(a, "interface"),
        login: int(a, "login"),
    });
    let errors = table(db, "errors").map(|e| {
        let last = table(e, "last");
        AddonErrors {
            count: int(e, "n").unwrap_or(0),
            last_message: last.and_then(|l| string(l, "msg")),
            last_event: last.and_then(|l| string(l, "event")),
            last_time: last.and_then(|l| int(l, "t")),
        }
    });

    Ok(Journal {
        schema,
        character,
        days,
        quests,
        companions,
        discovered,
        challenges,
        stats,
        addon,
        errors,
    })
}

/// Parses and reads a journal from the text of a character's SavedVariables file.
pub fn journal_from_text(text: &str) -> Result<Journal, JournalError> {
    journal_from(&sv::parse(text)?)
}

/// Reads the account-wide settings, or `None` when the file holds none.
pub fn account_settings_from(file: &SavedVariables) -> Option<AccountSettings> {
    let db = file.get(SETTINGS_VARIABLE).and_then(Value::as_table)?;
    let settings = table(db, "settings");
    Some(AccountSettings {
        schema: int(db, "schema").unwrap_or(0),
        record: flags(settings.and_then(|s| table(s, "record"))),
        loot_quality: settings.and_then(|s| int(s, "lootQuality")),
        retention_days: settings.and_then(|s| int(s, "retentionDays")),
        login_summary: settings
            .and_then(|s| boolean(s, "loginSummary"))
            .unwrap_or(false),
        screenshots: flags(settings.and_then(|s| table(s, "screenshots"))),
        chat: flags(settings.and_then(|s| table(s, "chat"))),
        welcomed: boolean(db, "welcomed").unwrap_or(false),
    })
}
