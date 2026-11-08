//! A day or week laid bare for debugging an entry: what the addon recorded, the facts the
//! narrator is told, the prompt the next write gets, and every version written so far. Written
//! as JSON so a person, or a tool, can read why an entry came out as it did.

use serde::Serialize;

use crate::archive;
use crate::beats::{self, Thresholds};
use crate::db::{Database, DbError};
use crate::milestones;
use crate::narrators::Library;
use crate::queue;
use crate::schema::Day;
use crate::stories::{self, Story, StoryKind};
use crate::usage::Audience;

/// Who the entry is about.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Who {
    pub name: Option<String>,
    pub surname: Option<String>,
    pub race: Option<String>,
    pub class: Option<String>,
    pub level: Option<i64>,
}

/// The prompt as the model would receive it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptText {
    pub system: String,
    pub user: String,
    /// The words the entry aims for.
    pub words: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DebugFile {
    pub app: String,
    pub character: Who,
    pub kind: StoryKind,
    pub start_day: String,
    /// The day as the addon recorded it, with only what the player lets narrators use; none for
    /// a week.
    pub recorded: Option<Day>,
    /// The facts the narrator is told about the day, one per line as in the prompt.
    pub facts: Vec<String>,
    /// The prompt the next write gets, with today's settings.
    pub prompt: Option<PromptText>,
    /// Why there is no prompt, when there is none.
    pub prompt_problem: Option<String>,
    /// Every version written for this day or week, newest first.
    pub versions: Vec<Story>,
}

#[derive(Debug, thiserror::Error)]
pub enum DebugError {
    #[error("that character is not in the app any more")]
    NoCharacter,
    #[error("the character's journal could not be read: {0}")]
    Journal(String),
    #[error(transparent)]
    Db(#[from] DbError),
}

/// Gathers the debug file for one day or week of a character.
pub fn day(
    db: &Database,
    library: &Library,
    character_id: i64,
    kind: StoryKind,
    start_day: &str,
    language_fallback: &str,
) -> Result<DebugFile, DebugError> {
    let character = db.character(character_id)?.ok_or(DebugError::NoCharacter)?;
    let journal = archive::load_for(db, character_id, Audience::Shared)
        .map_err(|e| DebugError::Journal(e.to_string()))?
        .ok_or(DebugError::NoCharacter)?;
    let language = db
        .settings()?
        .writing
        .prose_language
        .unwrap_or_else(|| language_fallback.to_string());
    let (recorded, facts) = match kind {
        StoryKind::Day => {
            let unused = db.hidden(character_id, Audience::Shared)?;
            let awards = crate::usage::awards_in_use(
                milestones::awards(db, character_id)?,
                &journal,
                &unused,
            );
            let facts = beats::extract(&journal, start_day, &awards, &Thresholds::default())
                .texts_in(&language);
            (journal.days.get(start_day).cloned(), facts)
        }
        StoryKind::Week => (None, Vec::new()),
    };
    let (prompt, prompt_problem) = match queue::next_prompt(
        db,
        library,
        character_id,
        kind,
        start_day,
        language_fallback,
        false,
    )? {
        Ok(p) => (
            Some(PromptText {
                system: p.system,
                user: p.user,
                words: p.words,
            }),
            None,
        ),
        Err(problem) => (None, Some(problem)),
    };
    Ok(DebugFile {
        app: env!("CARGO_PKG_VERSION").to_string(),
        character: Who {
            name: character.name,
            surname: character.surname,
            race: character.race,
            class: character.class,
            level: archive::current_level(db, character_id)
                .map_err(|e| DebugError::Journal(e.to_string()))?,
        },
        kind,
        start_day: start_day.to_string(),
        recorded,
        facts,
        prompt,
        prompt_problem,
        versions: stories::versions(db, character_id, kind, start_day)?,
    })
}
