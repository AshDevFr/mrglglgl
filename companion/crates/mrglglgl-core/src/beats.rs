//! What a day was about, worked out without any model: the day's facts collapsed into a short,
//! ranked list of beats, and flags for the patterns that make a day funny on its own (died three
//! times, only fished). The prompt is built from these, and the fact check measures an entry
//! against the same record.
//!
//! Beats are written in English whatever the prose language: they are notes for the model, and
//! names stay as the game recorded them.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::milestones::{self, Award};
use crate::schema::{Day, EventKind, Journal, Money};

/// Thresholds for the day patterns, kept as data so they can be tuned without code changes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Thresholds {
    /// The most beats a day keeps; the rest are dropped by rank.
    pub max_beats: usize,
    /// Deaths in one day that count as many.
    pub many_deaths: i64,
    /// Levels gained in one day that count as a big jump.
    pub big_level_jump: i64,
    /// Murlocs slain in one day that make it murloc-heavy.
    pub murloc_heavy: i64,
    /// The lowest item quality (3 is rare) that counts as notable loot.
    pub rare_loot_quality: i64,
    /// Days of fishing in a row that make a streak worth mentioning.
    pub fishing_streak_days: i64,
    /// The most of the player's own chat lines a day hands the narrator.
    pub spoken_lines: usize,
}

impl Default for Thresholds {
    fn default() -> Self {
        static DEFAULT: OnceLock<Thresholds> = OnceLock::new();
        DEFAULT
            .get_or_init(|| {
                serde_json::from_str(include_str!("../data/patterns.json"))
                    .expect("data/patterns.json matches Thresholds")
            })
            .clone()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KillLine {
    pub name: Option<String>,
    pub count: i64,
    pub classification: String,
    pub murloc: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeathLine {
    pub killer: Option<String>,
    pub murloc: bool,
    /// The environmental cause, e.g. `"Drowning"`.
    pub environment: Option<String>,
    /// Where it happened: the subzone when there was one, else the zone.
    pub place: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatchLine {
    pub name: Option<String>,
    pub count: i64,
    pub junk: bool,
    pub boot: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillLine {
    pub name: String,
    /// A trade or secondary skill, as opposed to weapon practice and the like.
    pub profession: bool,
    pub from: i64,
    pub to: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemLine {
    pub name: String,
    pub count: i64,
    /// For gathered items: the gathering skill used, e.g. "Mining".
    pub via: Option<String>,
}

/// Skill IDs of the trade and secondary skills. IDs do not depend on the client's language;
/// the skill list headings do, so the heading is only a fallback when an ID is missing.
const PROFESSION_SKILLS: &[i64] = &[
    129, // First Aid
    164, // Blacksmithing
    165, // Leatherworking
    171, // Alchemy
    182, // Herbalism
    185, // Cooking
    186, // Mining
    197, // Tailoring
    202, // Engineering
    333, // Enchanting
    356, // Fishing
    393, // Skinning
];

pub(crate) fn is_profession(id: Option<i64>, category: Option<&str>) -> bool {
    match id {
        Some(id) => PROFESSION_SKILLS.contains(&id),
        None => matches!(category, Some("Professions" | "Secondary Skills")),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LootLine {
    pub name: Option<String>,
    pub quality: Option<i64>,
    /// Seen in a group loot window: it dropped, not necessarily for the adventurer.
    pub seen: bool,
}

/// How a character grew in a day beyond levels and quests.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Progress {
    pub spells: Vec<String>,
    /// Mounts and pets, by name.
    pub collected: Vec<String>,
    /// `(faction, standing)`.
    pub reputations: Vec<(String, String)>,
    /// Better items put on, by name.
    pub gear: Vec<String>,
    /// Where the hearthstone was set.
    pub bound: Option<String>,
    pub flight_paths: Vec<String>,
    pub duels_won: i64,
    pub duels_lost: i64,
    /// Dungeons and raids, with the minutes spent inside when known.
    pub instances: Vec<(String, Option<i64>)>,
}

impl Progress {
    fn of(day: &Day) -> Self {
        let mut p = Progress::default();
        for event in day.events.iter().flatten() {
            match &event.kind {
                EventKind::Spell { name: Some(n), .. } => push_unique(&mut p.spells, n),
                EventKind::Mount { name: Some(n) } | EventKind::Pet { name: Some(n) } => {
                    push_unique(&mut p.collected, n)
                }
                EventKind::Reputation {
                    faction: Some(f),
                    standing: Some(s),
                } => p.reputations.push((f.clone(), s.clone())),
                EventKind::Gear { name: Some(n), .. } => push_unique(&mut p.gear, n),
                EventKind::Bind { area } => p.bound = area.clone().or(p.bound.take()),
                EventKind::FlightPath { area: Some(a) } => push_unique(&mut p.flight_paths, a),
                EventKind::Duel { won, .. } => match won {
                    Some(true) => p.duels_won += 1,
                    _ => p.duels_lost += 1,
                },
                EventKind::Instance {
                    act,
                    name: Some(n),
                    duration,
                    ..
                } => match act.as_deref() {
                    Some("enter") if !p.instances.iter().any(|(i, _)| i == n) => {
                        p.instances.push((n.clone(), None))
                    }
                    Some("leave") => {
                        if let Some(run) = p.instances.iter_mut().rev().find(|(i, _)| i == n) {
                            run.1 = duration.map(|d| (d + 30) / 60);
                        }
                    }
                    _ => {}
                },
                _ => {}
            }
        }
        p
    }

    pub fn is_empty(&self) -> bool {
        *self == Progress::default()
    }
}

/// One day's facts in one shape, from raw events when the archive has them and from the digest
/// once the game compacted the day.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DayRecord {
    pub zones: Vec<String>,
    pub subzones: Vec<String>,
    pub discovered: Vec<String>,
    pub level_from: Option<i64>,
    pub level_to: Option<i64>,
    pub levels_gained: i64,
    /// Most killed first.
    pub kills: Vec<KillLine>,
    pub anon_kills: i64,
    pub deaths: Vec<DeathLine>,
    pub fish: i64,
    /// Junk among the fish, boots included.
    pub junk: i64,
    pub boots: i64,
    /// Murlocs slain; exact even when a digest lists only some kills.
    pub murlocs: i64,
    pub catches: Vec<CatchLine>,
    pub loot: Vec<LootLine>,
    /// Character names without their realm.
    pub companions: Vec<String>,
    /// Seconds grouped with each companion, by name without the realm.
    pub grouped: BTreeMap<String, i64>,
    pub bosses: Vec<String>,
    pub accepted: Vec<i64>,
    pub turned_in: Vec<i64>,
    /// Skills that rose, professions first.
    pub skills: Vec<SkillLine>,
    pub crafted: Vec<ItemLine>,
    pub gathered: Vec<ItemLine>,
    /// Copper earned and spent, by source.
    pub money: Money,
    /// Auction house events as `(act, item)`, in order.
    pub auctions: Vec<(String, Option<String>)>,
    /// The player's own lines said aloud (say, emote, yell), as `(channel, text)`. Other
    /// players' lines are never part of a record: they are not the player's to hand a narrator.
    pub spoken: Vec<(String, String)>,
    /// Achievements completed (Legacy Challenges on Forever), by name, in order.
    pub achievements: Vec<String>,
    /// How the character grew, from the progress events, which compaction keeps.
    pub progress: Progress,
    /// A hardcore character died for good that day.
    pub final_death: bool,
    pub only_fished: bool,
    pub nothing_happened: bool,
}

const NOTABLE: &[&str] = &["elite", "rare", "rareelite", "worldboss"];

fn push_unique(list: &mut Vec<String>, item: &str) {
    if !item.is_empty() && !list.iter().any(|i| i == item) {
        list.push(item.to_string());
    }
}

/// Time grouped with a companion before they are named in the day's notes: shorter is a group
/// formed to kill something, not a session together.
const COMPANION_SECONDS: i64 = 15 * 60;

/// Seconds as minutes, to the nearest quarter hour, never below one.
fn quarter_hours(seconds: i64) -> i64 {
    (((seconds as f64 / 60.0 / 15.0).round() as i64) * 15).max(15)
}

/// "45 minutes", "1 hour", "1 hour 15 minutes", "2 hours".
fn duration_en(minutes: i64) -> String {
    let (h, m) = (minutes / 60, minutes % 60);
    let hours = match h {
        0 => String::new(),
        1 => "1 hour".to_string(),
        _ => format!("{h} hours"),
    };
    match (h, m) {
        (0, _) => format!("{m} minutes"),
        (_, 0) => hours,
        _ => format!("{hours} {m} minutes"),
    }
}

/// "45 minutes", "1 heure", "1 heure 15", "2 heures".
pub(crate) fn duration_fr(minutes: i64) -> String {
    let (h, m) = (minutes / 60, minutes % 60);
    let hours = if h == 1 {
        "1 heure".to_string()
    } else {
        format!("{h} heures")
    };
    match (h, m) {
        (0, _) => format!("{m} minutes"),
        (_, 0) => hours,
        _ => format!("{hours} {m}"),
    }
}

/// "Rowan-Silverpine" is shown as "Rowan".
fn companion_name(who: &str) -> String {
    who.split('-').next().unwrap_or(who).to_string()
}

fn place(zone: &Option<String>, subzone: &Option<String>) -> Option<String> {
    subzone
        .clone()
        .filter(|s| !s.is_empty())
        .or_else(|| zone.clone().filter(|z| !z.is_empty()))
}

impl DayRecord {
    pub fn of(day: &Day) -> Self {
        let mut r = Self::of_events(day);
        r.read_professions(day);
        r.read_money(day);
        r.spoken = day
            .chat
            .iter()
            .filter(|l| l.own() && matches!(l.channel.as_str(), "say" | "emote" | "yell"))
            .map(|l| (l.channel.clone(), l.text.clone()))
            .collect();
        r.progress = Progress::of(day);
        for (who, seconds) in &day.grouped {
            *r.grouped.entry(companion_name(who)).or_insert(0) += seconds;
        }
        if !r.spoken.is_empty() || !r.achievements.is_empty() || !r.progress.is_empty() {
            r.nothing_happened = false;
        }
        if !r.skills.is_empty()
            || !r.crafted.is_empty()
            || !r.gathered.is_empty()
            || !r.money.is_empty()
            || !r.auctions.is_empty()
        {
            r.nothing_happened = false;
        }
        r
    }

    /// Money and auctions: the day's own records while it has them, the digest otherwise. A
    /// compacted day keeps its money and its auction events.
    fn read_money(&mut self, day: &Day) {
        let digest = day.digest.as_ref();
        self.money = if day.money.is_empty() {
            digest.map(|d| d.money.clone()).unwrap_or_default()
        } else {
            day.money.clone()
        };
        self.achievements = match &day.events {
            Some(events) => events
                .iter()
                .filter_map(|e| match &e.kind {
                    EventKind::Achievement { name, .. } => name.clone(),
                    _ => None,
                })
                .collect(),
            None => digest
                .map(|d| {
                    d.achievements
                        .iter()
                        .filter_map(|a| a.name.clone())
                        .collect()
                })
                .unwrap_or_default(),
        };
        self.auctions = match &day.events {
            Some(events) => events
                .iter()
                .filter_map(|e| match &e.kind {
                    EventKind::Auction { act, name } => Some((act.clone()?, name.clone())),
                    _ => None,
                })
                .collect(),
            None => digest
                .map(|d| {
                    d.auctions
                        .iter()
                        .filter_map(|a| Some((a.act.clone()?, a.name.clone())))
                        .collect()
                })
                .unwrap_or_default(),
        };
    }

    /// Profession activity: the day's own maps while it has them, the digest after compaction.
    fn read_professions(&mut self, day: &Day) {
        let digest = day.digest.as_ref();
        self.skills = if day.skills.is_empty() {
            digest
                .map(|d| {
                    d.skills
                        .iter()
                        .map(|s| SkillLine {
                            name: s.name.clone().unwrap_or_default(),
                            profession: is_profession(s.skill_id, s.category.as_deref()),
                            from: s.from,
                            to: s.to,
                        })
                        .collect()
                })
                .unwrap_or_default()
        } else {
            day.skills
                .iter()
                .map(|(id, s)| SkillLine {
                    name: s.name.clone().unwrap_or_default(),
                    profession: is_profession(Some(*id), s.category.as_deref()),
                    from: s.from,
                    to: s.to,
                })
                .collect()
        };
        self.skills.retain(|s| !s.name.is_empty() && s.to > s.from);
        // The client lists some professions under two skill IDs with one name and rank; only one
        // of them is a known profession ID. One skill per name, a profession if either says so.
        let mut merged: Vec<SkillLine> = Vec::new();
        for skill in std::mem::take(&mut self.skills) {
            match merged.iter_mut().find(|m| m.name == skill.name) {
                Some(m) => {
                    m.profession |= skill.profession;
                    m.from = m.from.min(skill.from);
                    m.to = m.to.max(skill.to);
                }
                None => merged.push(skill),
            }
        }
        self.skills = merged;
        // Stable: professions first, each group in skill ID order.
        self.skills.sort_by_key(|s| !s.profession);

        let items = |own: &BTreeMap<i64, crate::schema::ItemCount>,
                     digested: Option<&Vec<crate::schema::DigestItem>>| {
            let lines: Vec<ItemLine> = if own.is_empty() {
                digested
                    .map(|list| {
                        list.iter()
                            .filter_map(|i| {
                                Some(ItemLine {
                                    name: i.name.clone()?,
                                    count: i.count,
                                    via: i.via.clone(),
                                })
                            })
                            .collect()
                    })
                    .unwrap_or_default()
            } else {
                own.values()
                    .filter_map(|i| {
                        Some(ItemLine {
                            name: i.name.clone()?,
                            count: i.count,
                            via: i.via.clone(),
                        })
                    })
                    .collect()
            };
            lines
        };
        self.crafted = items(&day.crafted, digest.map(|d| &d.crafted));
        self.gathered = items(&day.gathered, digest.map(|d| &d.gathered));
    }

    fn of_events(day: &Day) -> Self {
        let facts = milestones::facts(day);
        let mut r = DayRecord {
            only_fished: facts.only_fished,
            nothing_happened: facts.nothing_happened,
            ..DayRecord::default()
        };
        // A compacted day keeps a few key events; its digest is what the whole day was.
        let Some(events) = day.events.as_ref().filter(|_| !day.compacted) else {
            if let Some(d) = &day.digest {
                r.zones = d.zones.clone();
                r.subzones = d.subzones.clone();
                r.discovered = d.discovered.clone();
                r.level_from = d.level_from;
                r.level_to = d.level_to;
                r.levels_gained = d.counts.levels;
                r.kills = d
                    .kills
                    .iter()
                    .map(|k| KillLine {
                        name: k.name.clone(),
                        count: k.count,
                        classification: k.classification.clone(),
                        murloc: k.murloc,
                    })
                    .collect();
                // A digest's lists are highlights and its counts are exact: kills beyond the
                // listed ones are told as other creatures.
                let listed: i64 = d.kills.iter().map(|k| k.count).sum();
                r.anon_kills =
                    d.counts.anon_kills + (d.counts.kills - d.counts.anon_kills - listed).max(0);
                r.murlocs = d.counts.murlocs;
                r.deaths = d
                    .deaths
                    .iter()
                    .map(|death| DeathLine {
                        killer: death.killer.as_ref().and_then(|k| k.name.clone()),
                        murloc: death.killer.as_ref().is_some_and(|k| k.murloc),
                        environment: death.environment.clone(),
                        place: place(&death.zone, &death.subzone),
                    })
                    .collect();
                // Deaths beyond the listed ones are told without a cause.
                let unlisted = (d.counts.deaths - r.deaths.len() as i64).max(0);
                r.deaths.extend((0..unlisted).map(|_| DeathLine {
                    killer: None,
                    murloc: false,
                    environment: None,
                    place: None,
                }));
                r.fish = d.counts.fish;
                r.junk = d.counts.junk;
                r.boots = d.counts.boots;
                r.catches = d
                    .fish
                    .iter()
                    .map(|f| CatchLine {
                        name: f.name.clone(),
                        count: f.count,
                        junk: f.junk == Some(true),
                        boot: f.boot == Some(true),
                    })
                    .collect();
                r.loot = d
                    .loot
                    .iter()
                    .map(|l| LootLine {
                        name: l.name.clone(),
                        quality: l.quality,
                        seen: l.seen,
                    })
                    .collect();
                r.companions = d.companions.iter().map(|c| companion_name(c)).collect();
                r.bosses = d.bosses.clone();
                r.accepted = d.accepted.clone();
                r.turned_in = d.turned_in.clone();
            }
            r.sort_kills();
            return r;
        };

        r.kills = day
            .kills
            .values()
            .map(|k| KillLine {
                name: k.name.clone(),
                count: k.count,
                classification: k.classification.clone(),
                murloc: k.murloc,
            })
            .collect();
        r.anon_kills = day.anon_kills;
        let mut catches: Vec<(String, CatchLine)> = Vec::new();
        for event in events {
            if let Some(level) = event.level {
                r.level_from = Some(r.level_from.map_or(level, |l| l.min(level)));
                r.level_to = Some(r.level_to.map_or(level, |l| l.max(level)));
            }
            if let Some(zone) = &event.zone {
                push_unique(&mut r.zones, zone);
            }
            if let Some(subzone) = &event.subzone {
                push_unique(&mut r.subzones, subzone);
            }
            match &event.kind {
                EventKind::Level => r.levels_gained += 1,
                EventKind::Discover { area: Some(a), .. } => push_unique(&mut r.discovered, a),
                EventKind::Death {
                    killer,
                    environment,
                    final_death,
                } => {
                    r.final_death |= *final_death;
                    r.deaths.push(DeathLine {
                        killer: killer.as_ref().and_then(|k| k.name.clone()),
                        murloc: killer.as_ref().is_some_and(|k| k.murloc),
                        environment: environment.clone(),
                        place: place(&event.zone, &event.subzone),
                    });
                }
                EventKind::Fish {
                    item_id,
                    name,
                    junk,
                    boot,
                } => {
                    r.fish += 1;
                    if *boot == Some(true) {
                        r.boots += 1;
                    }
                    let key =
                        item_id.map_or_else(|| name.clone().unwrap_or_default(), |i| i.to_string());
                    match catches.iter_mut().find(|(k, _)| *k == key) {
                        Some((_, line)) => line.count += 1,
                        None => catches.push((
                            key,
                            CatchLine {
                                name: name.clone(),
                                count: 1,
                                junk: *junk == Some(true),
                                boot: *boot == Some(true),
                            },
                        )),
                    }
                }
                EventKind::Loot {
                    name,
                    quality,
                    seen,
                    ..
                } => r.loot.push(LootLine {
                    name: name.clone(),
                    quality: *quality,
                    seen: *seen,
                }),
                EventKind::GroupJoin { who: Some(who) } => {
                    push_unique(&mut r.companions, &companion_name(who))
                }
                EventKind::Boss { name: Some(n), .. } => push_unique(&mut r.bosses, n),
                EventKind::QuestAccept { quest_id: Some(q) } => r.accepted.push(*q),
                EventKind::QuestTurnin {
                    quest_id: Some(q), ..
                } => r.turned_in.push(*q),
                _ => {}
            }
        }
        r.catches = catches.into_iter().map(|(_, line)| line).collect();
        r.junk = r.catches.iter().filter(|c| c.junk).map(|c| c.count).sum();
        r.murlocs = r.kills.iter().filter(|k| k.murloc).map(|k| k.count).sum();
        r.sort_kills();
        r
    }

    fn sort_kills(&mut self) {
        // Stable, so equal counts keep the archive's order (npc id for raw days).
        self.kills.sort_by_key(|k| std::cmp::Reverse(k.count));
    }

    pub fn murloc_kills(&self) -> i64 {
        self.murlocs
    }

    /// Every place the day touched.
    pub fn places(&self) -> impl Iterator<Item = &String> {
        self.zones
            .iter()
            .chain(&self.subzones)
            .chain(&self.discovered)
    }
}

/// A day pattern. The names are what the prompt and the narrator profiles' hints use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Pattern {
    ManyDeaths,
    OnlyFished,
    BigLevelJump,
    NewZone,
    RareLoot,
    MurlocHeavy,
    MurlocDeath,
    NothingHappened,
    /// A hardcore character's last day.
    FinalDeath,
}

impl Pattern {
    pub const ALL: [Pattern; 9] = [
        Pattern::ManyDeaths,
        Pattern::OnlyFished,
        Pattern::BigLevelJump,
        Pattern::NewZone,
        Pattern::RareLoot,
        Pattern::MurlocHeavy,
        Pattern::MurlocDeath,
        Pattern::NothingHappened,
        Pattern::FinalDeath,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Pattern::ManyDeaths => "many_deaths",
            Pattern::OnlyFished => "only_fished",
            Pattern::BigLevelJump => "big_level_jump",
            Pattern::NewZone => "new_zone",
            Pattern::RareLoot => "rare_loot",
            Pattern::MurlocHeavy => "murloc_heavy",
            Pattern::MurlocDeath => "murloc_death",
            Pattern::NothingHappened => "nothing_happened",
            Pattern::FinalDeath => "final_death",
        }
    }
}

/// One thing worth telling about a day.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Beat {
    /// Where the day was spent.
    Travelled {
        zones: Vec<String>,
    },
    /// Places never seen on an earlier day.
    FirstVisit {
        places: Vec<String>,
    },
    /// Every named kill, collapsed by creature.
    Kills {
        lines: Vec<(String, i64)>,
        unnamed: i64,
    },
    /// An elite, rare or world boss.
    Notable {
        name: String,
        classification: String,
        count: i64,
    },
    Fishing {
        fish: i64,
        junk: Vec<(String, i64)>,
        /// Days of fishing in a row, today included, when it reaches the streak threshold.
        streak: Option<i64>,
    },
    Boot {
        item: String,
        count: i64,
    },
    Loot {
        items: Vec<String>,
    },
    /// Rare items seen dropping in a group loot window, which may have gone to someone else.
    SeenLoot {
        items: Vec<String>,
    },
    Death {
        killer: Option<String>,
        environment: Option<String>,
        place: Option<String>,
        count: i64,
    },
    LevelUp {
        from: Option<i64>,
        to: i64,
    },
    Companions {
        names: Vec<String>,
        new: Vec<String>,
        /// Minutes spent with each, in the order of `names`, rounded to a quarter hour; empty on
        /// a day recorded before the addon counted time grouped.
        minutes: Vec<i64>,
    },
    Bosses {
        names: Vec<String>,
    },
    /// Milestones stamped that day (by id, named in the prose language) and Legacy Challenges
    /// (the game's achievements) completed, as one beat so they do not crowd out the day itself.
    Milestones {
        names: Vec<String>,
        challenges: Vec<String>,
    },
    /// A trade or secondary skill that rose.
    Profession {
        name: String,
        from: i64,
        to: i64,
    },
    Crafted {
        items: Vec<(String, i64)>,
    },
    /// Items gathered with one gathering skill.
    Gathered {
        via: Option<String>,
        items: Vec<(String, i64)>,
    },
    /// Weapon and other skills that rose; only told on otherwise quiet days.
    Practice {
        skills: Vec<(String, i64, i64)>,
    },
    /// Money earned and spent, with where most of each came from or went.
    Money {
        earned: i64,
        spent: i64,
        earned_mostly: Option<String>,
        spent_mostly: Option<String>,
    },
    Learned {
        spells: Vec<String>,
    },
    /// Mounts and pets added.
    Collected {
        names: Vec<String>,
    },
    Reputation {
        lines: Vec<(String, String)>,
    },
    /// Better gear put on.
    Gear {
        items: Vec<String>,
    },
    /// The hearthstone set and flight paths learned.
    Travel {
        bound: Option<String>,
        flight_paths: Vec<String>,
    },
    /// Duels, counted: opponents are other players, whose names are not told.
    Duels {
        won: i64,
        lost: i64,
    },
    /// Dungeons and raids run, with minutes inside when known.
    Instances {
        runs: Vec<(String, Option<i64>)>,
    },
    /// A hardcore character died for good: the day is its last.
    Farewell,
    /// A few of the player's own lines, said or acted out in character.
    Spoke {
        lines: Vec<(String, String)>,
    },
    /// Items sold and bought at the auction house.
    Auctions {
        sold: Vec<String>,
        won: Vec<String>,
    },
    /// Quests handed in; `unnamed` counts those whose title the journal never learned.
    QuestsDone {
        titles: Vec<String>,
        unnamed: i64,
    },
    /// Quests taken on and not finished the same day.
    QuestsTaken {
        titles: Vec<String>,
        unnamed: i64,
    },
}

impl Beat {
    /// How much the beat matters when the day has more than fit: lower is kept first.
    fn rank(&self) -> u8 {
        match self {
            Beat::Milestones { .. } => 0,
            Beat::Boot { .. } => 1,
            Beat::Death { .. } => 2,
            // What the day was mostly spent doing; it must survive a crowded day.
            Beat::Kills { .. } => 3,
            Beat::LevelUp { .. } => 4,
            // A finished quest is often what the day was for.
            Beat::QuestsDone { .. } => 4,
            Beat::Notable { .. } => 5,
            Beat::Bosses { .. } => 6,
            Beat::FirstVisit { .. } => 7,
            Beat::Loot { .. } => 8,
            Beat::SeenLoot { .. } => 10,
            Beat::Fishing { .. } => 9,
            Beat::Profession { .. } => 9,
            Beat::Crafted { .. } => 9,
            Beat::Gathered { .. } => 9,
            // A real session with friends is what the day was; a name met in passing is not.
            Beat::Companions { minutes, .. } if !minutes.is_empty() => 3,
            Beat::Companions { .. } => 10,
            Beat::QuestsTaken { .. } => 10,
            Beat::Travelled { .. } => 11,
            Beat::Practice { .. } => 12,
            Beat::Auctions { .. } => 9,
            Beat::Money { .. } => 11,
            Beat::Spoke { .. } => 8,
            Beat::Instances { .. } => 4,
            Beat::Farewell => 0,
            Beat::Collected { .. } => 5,
            Beat::Reputation { .. } => 7,
            Beat::Gear { .. } => 8,
            Beat::Duels { .. } => 8,
            Beat::Learned { .. } => 9,
            Beat::Travel { .. } => 10,
        }
    }

    /// Where the beat sits when the day is told, roughly in the order a day unfolds.
    fn order(&self) -> u8 {
        match self {
            Beat::Travelled { .. } => 0,
            Beat::FirstVisit { .. } => 1,
            Beat::Companions { .. } => 2,
            Beat::QuestsTaken { .. } => 2,
            Beat::Kills { .. } => 3,
            Beat::Notable { .. } => 4,
            Beat::Bosses { .. } => 5,
            Beat::Gathered { .. } => 6,
            Beat::Crafted { .. } => 6,
            Beat::Fishing { .. } => 6,
            Beat::Boot { .. } => 7,
            Beat::Loot { .. } => 8,
            Beat::SeenLoot { .. } => 8,
            Beat::QuestsDone { .. } => 8,
            Beat::Death { .. } => 9,
            Beat::LevelUp { .. } => 10,
            Beat::Profession { .. } => 10,
            Beat::Practice { .. } => 10,
            Beat::Auctions { .. } => 10,
            Beat::Money { .. } => 10,
            Beat::Spoke { .. } => 9,
            Beat::Travel { .. } => 1,
            Beat::Farewell => 12,
            Beat::Instances { .. } => 5,
            Beat::Learned { .. } => 10,
            Beat::Collected { .. } => 10,
            Beat::Reputation { .. } => 10,
            Beat::Gear { .. } => 8,
            Beat::Duels { .. } => 9,
            Beat::Milestones { .. } => 11,
        }
    }

    /// The beat as a line of notes for the prompt, in English.
    /// The beat in the prose language: French for `"fr"`, English otherwise. Game names are kept
    /// as recorded either way.
    pub fn text_in(&self, language: &str) -> String {
        if language.starts_with("fr") {
            self.text_fr()
        } else {
            self.text()
        }
    }

    pub fn text(&self) -> String {
        match self {
            Beat::Travelled { zones } if zones.len() == 1 => {
                format!("Spent the day in {}", zones[0])
            }
            Beat::Travelled { zones } => format!("Travelled through {}", and_list(zones)),
            Beat::FirstVisit { places } => {
                format!("Visited for the first time: {}", and_list(places))
            }
            Beat::Kills { lines, unnamed } => {
                let mut parts: Vec<String> = lines
                    .iter()
                    .map(|(name, count)| format!("{name} ×{count}"))
                    .collect();
                if *unnamed > 0 {
                    parts.push(if *unnamed == 1 {
                        "1 other creature".to_string()
                    } else {
                        format!("{unnamed} other creatures")
                    });
                }
                format!("Killed {}", and_list(&parts))
            }
            Beat::Notable {
                name,
                classification,
                count,
            } => {
                let kind = match classification.as_str() {
                    "elite" => "elite",
                    "rare" => "rare",
                    "rareelite" => "rare elite",
                    _ => "world boss",
                };
                if *count == 1 {
                    format!("Killed {name}, {} {kind}", article(kind))
                } else {
                    format!("Killed {name} ×{count}, {kind}")
                }
            }
            Beat::Fishing { fish, junk, streak } => {
                let mut text = format!("Went fishing and caught {fish} fish");
                if !junk.is_empty() {
                    let items: Vec<String> = junk
                        .iter()
                        .map(|(name, count)| {
                            if *count == 1 {
                                format!("{} {name}", article(name))
                            } else {
                                format!("{name} ×{count}")
                            }
                        })
                        .collect();
                    text = format!("{text}, plus {}", and_list(&items));
                }
                if let Some(days) = streak {
                    text = format!("{text} (day {days} of fishing in a row)");
                }
                text
            }
            Beat::Boot { item, count } if *count == 1 => {
                format!("Fished up {} {item} and kept it", article(item))
            }
            Beat::Boot { item, count } => format!("Fished up {item} ×{count} and kept them all"),
            Beat::Loot { items } => format!("Looted {}", and_list(items)),
            Beat::SeenLoot { items } => format!("Saw {} drop", and_list(items)),
            Beat::Death {
                killer,
                environment,
                place,
                count,
            } => {
                let how = match (killer, environment.as_deref()) {
                    (_, Some("Drowning")) => "Drowned".to_string(),
                    (_, Some("Falling")) => "Died from a fall".to_string(),
                    (_, Some("Fatigue")) => "Died of fatigue in deep water".to_string(),
                    (_, Some("Fire")) => "Died in fire".to_string(),
                    (_, Some("Lava")) => "Died in lava".to_string(),
                    (_, Some("Slime")) => "Died in slime".to_string(),
                    (_, Some(other)) => format!("Died ({other})"),
                    (Some(k), None) => format!("Died to {} {k}", article(k)),
                    (None, None) => "Died".to_string(),
                };
                let times = if *count > 1 {
                    format!(" ({count} times)")
                } else {
                    String::new()
                };
                match place {
                    Some(p) => format!("{how} in {p}{times}"),
                    None => format!("{how}{times}"),
                }
            }
            Beat::LevelUp { from: Some(f), to } if to - f >= 2 => {
                format!("Went from level {f} to level {to}")
            }
            Beat::LevelUp { to, .. } => format!("Reached level {to}"),
            Beat::Companions {
                names,
                new,
                minutes,
            } => {
                let parts: Vec<String> = names
                    .iter()
                    .enumerate()
                    .map(|(i, n)| {
                        let mut part = n.clone();
                        if new.contains(n) {
                            part.push_str(" (for the first time)");
                        }
                        if let Some(m) = minutes.get(i) {
                            part.push_str(&format!(" for {}", duration_en(*m)));
                        }
                        part
                    })
                    .collect();
                format!("Grouped with {}", and_list(&parts))
            }
            Beat::Bosses { names } => format!("Defeated {}", and_list(names)),
            Beat::Milestones { names, challenges } => {
                let told = |list: &[String], one: &str, many: &str| {
                    let quoted: Vec<String> = list.iter().map(|n| format!("\"{n}\"")).collect();
                    let kind = if list.len() == 1 { one } else { many };
                    format!("{kind} {}", and_list(&quoted))
                };
                let mut parts = Vec::new();
                if !names.is_empty() {
                    let named: Vec<String> = names
                        .iter()
                        .map(|id| milestones::name(id).to_string())
                        .collect();
                    parts.push(told(
                        &named,
                        "earned the milestone",
                        "earned the milestones",
                    ));
                }
                if !challenges.is_empty() {
                    parts.push(told(
                        challenges,
                        "completed the Legacy Challenge",
                        "completed the Legacy Challenges",
                    ));
                }
                let text = parts.join(" and ");
                let mut chars = text.chars();
                match chars.next() {
                    Some(first) => format!("{}{}", first.to_uppercase(), chars.as_str()),
                    None => text,
                }
            }
            Beat::Profession { name, from, to } => format!("{name} rose from {from} to {to}"),
            Beat::Money {
                earned,
                spent,
                earned_mostly,
                spent_mostly,
            } => {
                let side = |verb: &str, copper: i64, mostly: &Option<String>| {
                    let mut part = format!("{verb} {}", gold(copper));
                    if let Some(where_) = mostly {
                        part.push_str(&format!(", mostly {where_}"));
                    }
                    part
                };
                let mut parts = Vec::new();
                if *earned > 0 {
                    parts.push(side("Earned", *earned, &money_place(earned_mostly, true)));
                }
                if *spent > 0 {
                    let verb = if parts.is_empty() { "Spent" } else { "spent" };
                    parts.push(side(verb, *spent, &money_place(spent_mostly, false)));
                }
                parts.join("; ")
            }
            Beat::Learned { spells } => format!("Learned {}", and_list(spells)),
            Beat::Collected { names } => format!("Added {} to the collection", and_list(names)),
            Beat::Reputation { lines } => {
                let parts: Vec<String> = lines
                    .iter()
                    .map(|(faction, standing)| format!("{standing} with {faction}"))
                    .collect();
                format!("Became {}", and_list(&parts))
            }
            Beat::Gear { items } => format!("Put on a better {}", and_list(items)),
            Beat::Travel {
                bound,
                flight_paths,
            } => {
                let mut parts = Vec::new();
                if let Some(place) = bound {
                    parts.push(format!("set the hearthstone at {place}"));
                }
                if !flight_paths.is_empty() {
                    let kind = if flight_paths.len() == 1 {
                        "the flight path at"
                    } else {
                        "the flight paths at"
                    };
                    parts.push(format!("learned {kind} {}", and_list(flight_paths)));
                }
                let text = parts.join(" and ");
                let mut chars = text.chars();
                match chars.next() {
                    Some(first) => format!("{}{}", first.to_uppercase(), chars.as_str()),
                    None => text,
                }
            }
            Beat::Duels { won, lost } => {
                let count = |n: i64| {
                    if n == 1 {
                        "a duel".to_string()
                    } else {
                        format!("{n} duels")
                    }
                };
                match (won, lost) {
                    (0, l) => format!("Lost {}", count(*l)),
                    (w, 0) => format!("Won {}", count(*w)),
                    (w, l) => format!("Won {} and lost {}", count(*w), count(*l)),
                }
            }
            Beat::Instances { runs } => {
                let parts: Vec<String> = runs
                    .iter()
                    .map(|(name, minutes)| match minutes {
                        Some(m) => format!("{name} ({m} minutes)"),
                        None => name.clone(),
                    })
                    .collect();
                format!("Ran {}", and_list(&parts))
            }
            Beat::Farewell => {
                "Died for good: on this hardcore character, this was the last day".to_string()
            }
            Beat::Spoke { lines } => {
                let told: Vec<String> = lines
                    .iter()
                    .map(|(channel, text)| match channel.as_str() {
                        "emote" => format!("*{text}*"),
                        _ => format!("\"{text}\""),
                    })
                    .collect();
                format!("In character: {}", told.join(" "))
            }
            Beat::Auctions { sold, won } => {
                let mut parts = Vec::new();
                if !sold.is_empty() {
                    parts.push(format!("sold {}", and_list(sold)));
                }
                if !won.is_empty() {
                    parts.push(format!("won {}", and_list(won)));
                }
                let told = parts.join(" and ");
                let mut chars = told.chars();
                let first = chars.next().map(|c| c.to_uppercase().to_string());
                format!(
                    "{}{} at the auction house",
                    first.unwrap_or_default(),
                    chars.as_str()
                )
            }
            Beat::Crafted { items } => format!("Crafted {}", and_list(&counted(items))),
            Beat::Gathered { via, items } => match via {
                Some(skill) => format!("Gathered {} ({skill})", and_list(&counted(items))),
                None => format!("Gathered {}", and_list(&counted(items))),
            },
            Beat::QuestsDone { titles, unnamed } => {
                format!("Completed {}", quest_list(titles, *unnamed))
            }
            Beat::QuestsTaken { titles, unnamed } => {
                format!("Took on {}", quest_list(titles, *unnamed))
            }
            Beat::Practice { skills } => {
                let parts: Vec<String> = skills
                    .iter()
                    .map(|(name, from, to)| format!("{name} from {from} to {to}"))
                    .collect();
                format!("Practised {}", and_list(&parts))
            }
        }
    }
}

/// Copper as the game shows it: "3g 20s 5c", leaving out empty parts.
pub fn gold(copper: i64) -> String {
    let (g, s, c) = (copper / 10_000, copper / 100 % 100, copper % 100);
    let mut parts = Vec::new();
    if g > 0 {
        parts.push(format!("{g}g"));
    }
    if s > 0 {
        parts.push(format!("{s}s"));
    }
    if c > 0 || parts.is_empty() {
        parts.push(format!("{c}c"));
    }
    parts.join(" ")
}

/// Where money came from or went, as told: "at the auction house", "on repairs".
fn money_place(source: &Option<String>, earned: bool) -> Option<String> {
    let told = match (source.as_deref()?, earned) {
        ("loot", _) => "from loot",
        ("quest", true) => "from quests",
        ("quest", false) => "on quests",
        ("vendor", true) => "selling to vendors",
        ("vendor", false) => "at vendors",
        ("repair", _) => "on repairs",
        ("trainer", _) => "on training",
        ("travel", _) => "on flights",
        ("mail", _) => "by mail",
        ("auction", _) => "at the auction house",
        ("trade", _) => "in trades",
        _ => return None,
    };
    Some(told.to_string())
}

/// The source holding at least half of a side, if one does.
fn mostly(side: &BTreeMap<String, i64>) -> Option<String> {
    let total: i64 = side.values().sum();
    side.iter()
        .filter(|(_, copper)| **copper * 2 >= total && **copper > 0)
        .max_by_key(|(_, copper)| **copper)
        .map(|(source, _)| source.clone())
}

/// "the quest \"A\"", "the quests \"A\" and \"B\", and 1 other quest", "2 quests".
fn quest_list(titles: &[String], unnamed: i64) -> String {
    let others = |n: i64, other: bool| {
        let kind = if other { "other quest" } else { "quest" };
        if n == 1 {
            format!("1 {kind}")
        } else {
            format!("{n} {kind}s")
        }
    };
    if titles.is_empty() {
        return others(unnamed, false);
    }
    let quoted: Vec<String> = titles.iter().map(|t| format!("\"{t}\"")).collect();
    let named = if titles.len() == 1 {
        format!("the quest {}", quoted[0])
    } else {
        format!("the quests {}", and_list(&quoted))
    };
    if unnamed > 0 {
        format!("{named}, and {}", others(unnamed, true))
    } else {
        named
    }
}

fn article(word: &str) -> &'static str {
    match word.chars().next() {
        Some(c) if "AEIOUaeiou".contains(c) => "an",
        _ => "a",
    }
}

/// Names with how many of each.
type Counted = Vec<(String, i64)>;

fn counted(items: &[(String, i64)]) -> Vec<String> {
    items
        .iter()
        .map(|(name, count)| format!("{name} ×{count}"))
        .collect()
}

fn and_list(items: &[String]) -> String {
    match items {
        [] => String::new(),
        [one] => one.clone(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

/// A day's beats, in telling order, and its patterns.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DayBeats {
    pub beats: Vec<Beat>,
    pub patterns: BTreeSet<Pattern>,
}

impl DayBeats {
    pub fn texts(&self) -> Vec<String> {
        self.beats.iter().map(Beat::text).collect()
    }

    /// The beats in the prose language: French for `"fr"`, English otherwise.
    pub fn texts_in(&self, language: &str) -> Vec<String> {
        self.beats.iter().map(|b| b.text_in(language)).collect()
    }
}

fn previous_day(day: &str) -> Option<String> {
    let date = NaiveDate::parse_from_str(day, "%Y-%m-%d").ok()?;
    Some(date.pred_opt()?.format("%Y-%m-%d").to_string())
}

/// Days of fishing in a row ending on `day`.
fn fishing_streak(journal: &Journal, day: &str) -> i64 {
    let mut streak = 0;
    let mut current = Some(day.to_string());
    while let Some(d) = current {
        match journal.days.get(&d) {
            Some(record) if milestones::facts(record).fish > 0 => streak += 1,
            _ => break,
        }
        current = previous_day(&d);
    }
    streak
}

/// The beats and patterns of `day` in `journal`. `awards` are the character's milestones; only
/// those earned on `day` are used. Pure: the same inputs always give the same beats.
pub fn extract(journal: &Journal, day: &str, awards: &[Award], t: &Thresholds) -> DayBeats {
    let Some(today) = journal.days.get(day) else {
        return DayBeats::default();
    };
    let record = DayRecord::of(today);

    let mut seen_places = BTreeSet::new();
    let mut seen_companions = BTreeSet::new();
    for (_, earlier) in journal.days.range(..day.to_string()) {
        let r = DayRecord::of(earlier);
        seen_places.extend(r.places().cloned());
        seen_companions.extend(r.companions);
    }

    let mut beats = Vec::new();
    if !record.zones.is_empty() {
        beats.push(Beat::Travelled {
            zones: record.zones.clone(),
        });
    }
    let mut new_places = Vec::new();
    for p in record.places() {
        if !seen_places.contains(p) {
            push_unique(&mut new_places, p);
        }
    }
    if !new_places.is_empty() {
        beats.push(Beat::FirstVisit {
            places: new_places.clone(),
        });
    }

    let named: Vec<(String, i64)> = record
        .kills
        .iter()
        .filter(|k| !NOTABLE.contains(&k.classification.as_str()))
        .filter_map(|k| Some((k.name.clone()?, k.count)))
        .collect();
    let unnamed = record
        .kills
        .iter()
        .filter(|k| k.name.is_none())
        .map(|k| k.count)
        .sum::<i64>()
        + record.anon_kills;
    if !named.is_empty() || unnamed > 0 {
        beats.push(Beat::Kills {
            lines: named,
            unnamed,
        });
    }
    for k in &record.kills {
        if let (true, Some(name)) = (NOTABLE.contains(&k.classification.as_str()), &k.name) {
            beats.push(Beat::Notable {
                name: name.clone(),
                classification: k.classification.clone(),
                count: k.count,
            });
        }
    }
    if !record.bosses.is_empty() {
        beats.push(Beat::Bosses {
            names: record.bosses.clone(),
        });
    }

    if record.fish > 0 {
        let streak = fishing_streak(journal, day);
        beats.push(Beat::Fishing {
            // From the counts, which are exact; a digest lists only some catches.
            fish: record.fish - record.junk,
            junk: record
                .catches
                .iter()
                .filter(|c| c.junk && !c.boot)
                .filter_map(|c| Some((c.name.clone()?, c.count)))
                .collect(),
            streak: (streak >= t.fishing_streak_days).then_some(streak),
        });
    }
    if record.boots > 0 {
        let item = record
            .catches
            .iter()
            .find(|c| c.boot)
            .and_then(|c| c.name.clone())
            .unwrap_or_else(|| "Old Boot".to_string());
        beats.push(Beat::Boot {
            item,
            count: record.boots,
        });
    }

    let rare = |seen: bool| -> Vec<String> {
        record
            .loot
            .iter()
            .filter(|l| l.seen == seen && l.quality.is_some_and(|q| q >= t.rare_loot_quality))
            .filter_map(|l| l.name.clone())
            .collect()
    };
    let rare_loot = rare(false);
    if !rare_loot.is_empty() {
        beats.push(Beat::Loot {
            items: rare_loot.clone(),
        });
    }
    let seen_loot = rare(true);
    if !seen_loot.is_empty() {
        beats.push(Beat::SeenLoot { items: seen_loot });
    }

    // Deaths collapse by cause and place, in the order they first happened.
    let mut deaths: Vec<(DeathLine, i64)> = Vec::new();
    for d in &record.deaths {
        match deaths.iter_mut().find(|(line, _)| line == d) {
            Some((_, n)) => *n += 1,
            None => deaths.push((d.clone(), 1)),
        }
    }
    for (d, count) in &deaths {
        beats.push(Beat::Death {
            killer: d.killer.clone(),
            environment: d.environment.clone(),
            place: d.place.clone(),
            count: *count,
        });
    }

    if let (true, Some(to)) = (record.levels_gained > 0, record.level_to) {
        beats.push(Beat::LevelUp {
            from: record.level_from.filter(|f| *f < to),
            to,
        });
    }
    // With time grouped counted, only companions who stayed a while are named, longest first;
    // a day recorded before that names everyone met.
    let (companions, minutes): (Vec<String>, Vec<i64>) = if record.grouped.is_empty() {
        (record.companions.clone(), Vec::new())
    } else {
        let mut long: Vec<(&String, i64)> = record
            .grouped
            .iter()
            .filter(|(_, s)| **s >= COMPANION_SECONDS)
            .map(|(who, s)| (who, *s))
            .collect();
        long.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
        long.into_iter()
            .map(|(who, s)| (who.clone(), quarter_hours(s)))
            .unzip()
    };
    if !companions.is_empty() {
        beats.push(Beat::Companions {
            new: companions
                .iter()
                .filter(|c| !seen_companions.contains(*c))
                .cloned()
                .collect(),
            names: companions,
            minutes,
        });
    }
    for s in record.skills.iter().filter(|s| s.profession) {
        beats.push(Beat::Profession {
            name: s.name.clone(),
            from: s.from,
            to: s.to,
        });
    }
    if !record.crafted.is_empty() {
        beats.push(Beat::Crafted {
            items: record
                .crafted
                .iter()
                .map(|i| (i.name.clone(), i.count))
                .collect(),
        });
    }
    let mut gathered: Vec<(Option<String>, Counted)> = Vec::new();
    for item in &record.gathered {
        let entry = (item.name.clone(), item.count);
        match gathered.iter_mut().find(|(via, _)| *via == item.via) {
            Some((_, items)) => items.push(entry),
            None => gathered.push((item.via.clone(), vec![entry])),
        }
    }
    for (via, items) in gathered {
        beats.push(Beat::Gathered { via, items });
    }
    // A quest taken and handed in the same day is told once, as completed.
    let titled = |ids: &[i64]| {
        let mut titles: Vec<String> = Vec::new();
        let mut unnamed = 0;
        for id in ids {
            match journal.quests.get(id).and_then(|q| q.title.clone()) {
                Some(title) if !titles.contains(&title) => titles.push(title),
                Some(_) => {}
                None => unnamed += 1,
            }
        }
        (titles, unnamed)
    };
    let mut done: Vec<i64> = Vec::new();
    for id in &record.turned_in {
        if !done.contains(id) {
            done.push(*id);
        }
    }
    let mut taken: Vec<i64> = Vec::new();
    for id in &record.accepted {
        if !done.contains(id) && !taken.contains(id) {
            taken.push(*id);
        }
    }
    if !done.is_empty() {
        let (titles, unnamed) = titled(&done);
        beats.push(Beat::QuestsDone { titles, unnamed });
    }
    if !taken.is_empty() {
        let (titles, unnamed) = titled(&taken);
        beats.push(Beat::QuestsTaken { titles, unnamed });
    }
    let practice: Vec<(String, i64, i64)> = record
        .skills
        .iter()
        .filter(|s| !s.profession)
        .map(|s| (s.name.clone(), s.from, s.to))
        .collect();
    if !practice.is_empty() {
        beats.push(Beat::Practice { skills: practice });
    }
    let (earned, spent) = (record.money.earned(), record.money.spent());
    if earned > 0 || spent > 0 {
        beats.push(Beat::Money {
            earned,
            spent,
            earned_mostly: mostly(&record.money.inc),
            spent_mostly: mostly(&record.money.out),
        });
    }
    let traded = |wanted: &str| -> Vec<String> {
        let mut items = Vec::new();
        for (act, name) in &record.auctions {
            if let (true, Some(name)) = (act == wanted, name) {
                push_unique(&mut items, name);
            }
        }
        items
    };
    if record.final_death {
        beats.push(Beat::Farewell);
    }
    let p = &record.progress;
    if !p.instances.is_empty() {
        beats.push(Beat::Instances {
            runs: p.instances.clone(),
        });
    }
    if !p.collected.is_empty() {
        beats.push(Beat::Collected {
            names: p.collected.clone(),
        });
    }
    if !p.reputations.is_empty() {
        beats.push(Beat::Reputation {
            lines: p.reputations.clone(),
        });
    }
    if !p.gear.is_empty() {
        beats.push(Beat::Gear {
            items: p.gear.clone(),
        });
    }
    if p.duels_won + p.duels_lost > 0 {
        beats.push(Beat::Duels {
            won: p.duels_won,
            lost: p.duels_lost,
        });
    }
    if !p.spells.is_empty() {
        beats.push(Beat::Learned {
            spells: p.spells.clone(),
        });
    }
    if p.bound.is_some() || !p.flight_paths.is_empty() {
        beats.push(Beat::Travel {
            bound: p.bound.clone(),
            flight_paths: p.flight_paths.clone(),
        });
    }
    if !record.spoken.is_empty() {
        beats.push(Beat::Spoke {
            lines: record.spoken.iter().take(t.spoken_lines).cloned().collect(),
        });
    }
    let (sold, won) = (traded("sold"), traded("won"));
    if !sold.is_empty() || !won.is_empty() {
        beats.push(Beat::Auctions { sold, won });
    }
    // A day with many stamps (a character's first days earn several) tells only the rarest.
    let earned: Vec<String> = milestones::rarest(
        awards.iter().filter(|a| a.day == day),
        milestones::TOLD_PER_DAY,
    )
    .into_iter()
    .map(|a| a.id.clone())
    .collect();
    if !earned.is_empty() || !record.achievements.is_empty() {
        beats.push(Beat::Milestones {
            names: earned,
            challenges: record.achievements.clone(),
        });
    }

    // Keep the most important beats, then tell them in day order.
    let mut ranked: Vec<(usize, Beat)> = beats.into_iter().enumerate().collect();
    ranked.sort_by_key(|(i, b)| (b.rank(), *i));
    ranked.truncate(t.max_beats);
    ranked.sort_by_key(|(i, b)| (b.order(), *i));

    let mut patterns = BTreeSet::new();
    if record.deaths.len() as i64 >= t.many_deaths {
        patterns.insert(Pattern::ManyDeaths);
    }
    if record.only_fished {
        patterns.insert(Pattern::OnlyFished);
    }
    let gained = record.levels_gained.max(
        record
            .level_to
            .zip(record.level_from)
            .map_or(0, |(to, from)| to - from),
    );
    if gained >= t.big_level_jump {
        patterns.insert(Pattern::BigLevelJump);
    }
    if !new_places.is_empty() {
        patterns.insert(Pattern::NewZone);
    }
    if !rare_loot.is_empty() {
        patterns.insert(Pattern::RareLoot);
    }
    if record.murloc_kills() >= t.murloc_heavy {
        patterns.insert(Pattern::MurlocHeavy);
    }
    if record.deaths.iter().any(|d| d.murloc) {
        patterns.insert(Pattern::MurlocDeath);
    }
    if record.nothing_happened {
        patterns.insert(Pattern::NothingHappened);
    }
    if record.final_death {
        patterns.insert(Pattern::FinalDeath);
    }

    DayBeats {
        beats: ranked.into_iter().map(|(_, b)| b).collect(),
        patterns,
    }
}

/// Every day of the journal, oldest first, with its record. Handy for counting things up to a
/// given day.
pub fn records(journal: &Journal) -> BTreeMap<&str, DayRecord> {
    journal
        .days
        .iter()
        .map(|(d, day)| (d.as_str(), DayRecord::of(day)))
        .collect()
}
