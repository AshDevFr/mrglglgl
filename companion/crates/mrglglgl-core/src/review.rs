//! The To review list: a character's finished days and weeks that have no accepted entry, that
//! have a draft waiting, or that have a new version being written for review, with what stands
//! in each one's way.

use serde::Serialize;

use crate::db::{Database, DbError};
use crate::prompt;
use crate::queue::{self, Job};
use crate::schema::Journal;
use crate::stories::{self, Story, StoryKind, StoryStatus};

/// One day or week on the To review list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase")]
pub struct ReviewItem {
    pub kind: StoryKind,
    /// The day, or the week's Monday.
    pub start_day: String,
    /// The day, or the week's Sunday.
    pub end_day: String,
    /// Drafts waiting for the player, newest first.
    pub drafts: Vec<Story>,
    /// The journal already has an accepted entry here; a draft would replace it.
    pub has_entry: bool,
    /// The player threw away every draft and none is waiting.
    pub discarded: bool,
    /// The writing job for this day or week, if one is queued, running or failed.
    pub job: Option<Job>,
}

/// The finished days and weeks without an accepted entry, with a draft waiting, or with a new
/// version on its way for review, newest first.
pub fn items(
    db: &Database,
    character_id: i64,
    journal: &Journal,
    today: &str,
) -> Result<Vec<ReviewItem>, DbError> {
    let all = stories::all(db, character_id)?;
    let jobs = queue::jobs(db, character_id)?;
    let mut items = Vec::new();
    // Weeks off: a chapter is listed only for a draft still waiting from before.
    let weekly = db.settings()?.writing.weekly;
    for (kind, start_day) in queue::finished_slots(journal, today, true) {
        let here: Vec<&Story> = all
            .iter()
            .filter(|s| s.kind == kind && s.start_day == start_day)
            .collect();
        let has = |status| here.iter().any(|s| s.status == status);
        let drafts: Vec<Story> = here
            .iter()
            .filter(|s| s.status == StoryStatus::Draft)
            .map(|s| (*s).clone())
            .collect();
        // A day told only so far still waits for its full entry.
        let has_entry = here
            .iter()
            .any(|s| s.status == StoryStatus::Accepted && !s.partial);
        let job = jobs
            .iter()
            .find(|j| j.kind == kind && j.start_day == start_day)
            .cloned();
        // A new version asked for To review has its place there from the moment it is asked
        // for, so the words appear where the draft will wait, and a failure shows there too.
        let for_review = job.as_ref().is_some_and(|j| j.draft);
        if has_entry && drafts.is_empty() && !for_review {
            continue;
        }
        if kind == StoryKind::Week && !weekly && drafts.is_empty() && !for_review {
            continue;
        }
        let end_day = match kind {
            StoryKind::Day => start_day.clone(),
            StoryKind::Week => prompt::week_days(&start_day).pop().unwrap_or_default(),
        };
        items.push(ReviewItem {
            discarded: drafts.is_empty() && has(StoryStatus::Discarded),
            job,
            kind,
            start_day,
            end_day,
            drafts,
            has_entry,
        });
    }
    // Newest first by the day each one ends; a week before the Sunday it ends on.
    items.sort_by(|a, b| b.end_day.cmp(&a.end_day).then(b.kind.cmp(&a.kind)));
    Ok(items)
}

/// How many days and weeks have a draft waiting for the player.
pub fn waiting(db: &Database, character_id: i64) -> Result<usize, DbError> {
    let mut slots: Vec<(StoryKind, String)> = stories::all(db, character_id)?
        .into_iter()
        .filter(|s| s.status == StoryStatus::Draft)
        .map(|s| (s.kind, s.start_day))
        .collect();
    // `all` is ordered by day or week, so a slot's drafts are neighbours.
    slots.dedup();
    Ok(slots.len())
}
