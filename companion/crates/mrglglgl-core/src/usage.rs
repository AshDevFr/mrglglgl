//! Which recorded data the app uses for a character. Everything the addon records is used by
//! default; the player can set a category aside per character, and then the app neither shows it,
//! nor tells it to a narrator, nor puts it on the website or in an export. The archive keeps it
//! whole, so turning the category back on brings everything back.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::archive::{self, ArchiveError};
use crate::db::Database;
use crate::milestones::{self, Award};
use crate::schema::{Day, Digest, EventKind, Journal, Money};

/// The categories a player can set aside, in the order the app lists them. They follow the
/// addon's recording categories.
pub const CATEGORIES: &[&str] = &[
    "sessions",
    "levels",
    "quests",
    "zones",
    "kills",
    "deaths",
    "fishing",
    "professions",
    "loot",
    "money",
    "group",
    "chat",
];

/// Who a journal is read for. The player sets data aside from each separately.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Audience {
    /// Narrators, the website and exports.
    Shared,
    /// The app's own window: the days and what happened in them.
    View,
}

/// The categories set aside for one character, per audience. Empty sets: everything is used.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(default)]
pub struct DataUse {
    /// Kept from narrators, the website and exports.
    pub shared: BTreeSet<String>,
    /// Hidden in the window.
    pub view: BTreeSet<String>,
}

/// The category an event kind belongs to; none for kinds every category shares (screenshots,
/// kinds a newer addon added).
fn category(kind: &EventKind) -> Option<&'static str> {
    Some(match kind {
        EventKind::SessionStart | EventKind::SessionEnd => "sessions",
        EventKind::Level => "levels",
        EventKind::Zone | EventKind::Subzone | EventKind::Discover { .. } => "zones",
        EventKind::Death { .. } => "deaths",
        EventKind::Boss { .. } => "kills",
        EventKind::QuestAccept { .. } | EventKind::QuestTurnin { .. } => "quests",
        EventKind::Fish { .. } => "fishing",
        EventKind::Loot { .. } => "loot",
        EventKind::GroupJoin { .. } => "group",
        EventKind::Auction { .. } => "money",
        EventKind::Screenshot { .. } | EventKind::Unknown { .. } => return None,
    })
}

/// A copy of `journal` without the categories in `unused`.
pub fn strip(journal: &Journal, unused: &BTreeSet<String>) -> Journal {
    let mut j = journal.clone();
    if unused.is_empty() {
        return j;
    }
    let off = |c: &str| unused.contains(c);
    for day in j.days.values_mut() {
        strip_day(day, &off);
    }
    if off("quests") {
        j.quests.clear();
    }
    if off("zones") {
        j.discovered.clear();
    }
    if off("group") {
        j.companions.clear();
    }
    if off("kills") {
        j.stats.murlocs_slain = 0;
    }
    if off("deaths") {
        j.stats.deaths_by_murloc = 0;
        j.stats.drownings = 0;
    }
    if off("fishing") {
        j.stats.boots = 0;
    }
    j
}

fn strip_day(day: &mut Day, off: &dyn Fn(&str) -> bool) {
    if let Some(events) = &mut day.events {
        events.retain(|e| category(&e.kind).is_none_or(|c| !off(c)));
        for e in events.iter_mut() {
            if off("zones") {
                e.zone = None;
                e.subzone = None;
                e.map_id = None;
            }
            if off("levels") {
                e.level = None;
            }
            if off("money") {
                e.balance = None;
            }
        }
    }
    if off("kills") {
        day.kills.clear();
        day.anon_kills = 0;
    }
    if off("professions") {
        day.skills.clear();
        day.crafted.clear();
        day.gathered.clear();
    }
    if off("money") {
        day.money = Money::default();
    }
    if off("chat") {
        day.chat.clear();
    }
    if let Some(d) = &mut day.digest {
        strip_digest(d, off);
    }
}

fn strip_digest(d: &mut Digest, off: &dyn Fn(&str) -> bool) {
    let c = &mut d.counts;
    if off("sessions") {
        c.sessions = 0;
    }
    if off("levels") {
        c.levels = 0;
        d.level_from = None;
        d.level_to = None;
    }
    if off("quests") {
        c.accepted = 0;
        c.turned_in = 0;
        d.accepted.clear();
        d.turned_in.clear();
    }
    if off("zones") {
        c.discoveries = 0;
        d.zones.clear();
        d.subzones.clear();
        d.discovered.clear();
        for death in &mut d.deaths {
            death.zone = None;
            death.subzone = None;
        }
    }
    if off("kills") {
        c.kills = 0;
        c.murlocs = 0;
        c.anon_kills = 0;
        c.bosses = 0;
        d.kills.clear();
        d.notable_kills.clear();
        d.bosses.clear();
    }
    if off("deaths") {
        c.deaths = 0;
        d.deaths.clear();
    }
    if off("fishing") {
        c.fish = 0;
        c.junk = 0;
        c.boots = 0;
        d.fish.clear();
        d.only_fished = false;
    }
    if off("loot") {
        c.loot = 0;
        d.loot.clear();
    }
    if off("professions") {
        c.skill_ups = 0;
        c.crafted = 0;
        c.gathered = 0;
        d.skills.clear();
        d.crafted.clear();
        d.gathered.clear();
    }
    if off("money") {
        c.money_in = 0;
        c.money_out = 0;
        c.auctions = 0;
        d.money = Money::default();
        d.auctions.clear();
        d.bal_start = None;
        d.bal_end = None;
    }
    if off("group") {
        c.group = 0;
        d.companions.clear();
    }
    if off("chat") {
        c.chat = 0;
    }
}

/// The awards still earned once the set-aside categories are gone: a milestone that came from
/// hidden data would hint at it.
pub fn awards_in_use(
    awards: Vec<Award>,
    journal_in_use: &Journal,
    unused: &BTreeSet<String>,
) -> Vec<Award> {
    if unused.is_empty() {
        return awards;
    }
    let earned: BTreeSet<String> = milestones::compute(journal_in_use)
        .into_iter()
        .map(|a| a.id)
        .collect();
    awards
        .into_iter()
        .filter(|a| earned.contains(&a.id))
        .collect()
}

/// A character's milestones without those that came from data set aside from `audience`. Reads
/// the journal only when something is set aside.
pub fn awards(
    db: &Database,
    character_id: i64,
    audience: Audience,
) -> Result<Vec<Award>, ArchiveError> {
    let all = milestones::awards(db, character_id)?;
    let hidden = db.hidden(character_id, audience)?;
    if hidden.is_empty() {
        return Ok(all);
    }
    let Some(journal) = archive::load(db, character_id)? else {
        return Ok(all);
    };
    Ok(awards_in_use(all, &strip(&journal, &hidden), &hidden))
}
