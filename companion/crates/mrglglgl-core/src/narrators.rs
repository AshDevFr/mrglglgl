//! Narrator profiles: the persona a prompt is written in. Each built-in profile is a TOML data
//! file in `narrators/`, one per narrator and prose language, so new voices and translations need
//! no code. Players' own narrators are files in the same format in the app's data folder, made
//! from a copy of a built-in or imported from a shared file. Built-ins are never changed.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use crate::beats::Pattern;

/// The id every player-made narrator starts with; no built-in id does.
pub const CUSTOM_PREFIX: &str = "custom_";
/// The longest text a profile field may hold, so the prompt stays within what small models
/// handle well.
pub const MAX_FIELD_CHARS: usize = 1500;

/// The narrator used when none is chosen, for every race and class.
pub const DEFAULT_NARRATOR: &str = "murloc_scribe";

fn default_intro() -> String {
    "Example of the expected shape, for a different day (do not reuse its facts):".to_string()
}

fn default_notes_label() -> String {
    "Notes:".to_string()
}

fn default_entry_label() -> String {
    "Entry:".to_string()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct Example {
    /// The line introducing the example, in the profile's language.
    #[serde(default = "default_intro")]
    pub intro: String,
    #[serde(default = "default_notes_label")]
    pub notes_label: String,
    #[serde(default = "default_entry_label")]
    pub entry_label: String,
    /// The notes of a made-up day.
    pub notes: String,
    /// The entry written from them.
    pub entry: String,
    /// The margin note, shown only when the prompt asks for one.
    pub margin_note: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct Narrator {
    pub id: String,
    /// The prose language this profile is written for, e.g. `"en"`.
    pub language: String,
    pub name: String,
    /// Shown only to players who turn it on.
    #[serde(default)]
    pub opt_in: bool,
    pub persona: String,
    pub voice: String,
    /// How the murloc may show when no margin note is asked for.
    pub murloc: String,
    /// The instruction used when the murloc meddles.
    pub margin_note: String,
    pub example: Example,
    /// What to emphasise on a day with a pattern, keyed by the pattern's name.
    #[serde(default)]
    pub hints: BTreeMap<String, String>,
}

/// A narrator as the settings screen lists it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase")]
pub struct NarratorInfo {
    pub id: String,
    pub name: String,
    pub opt_in: bool,
    /// Made by the player, not shipped with the app.
    pub custom: bool,
}

const BUILT_IN: &[&str] = &[
    include_str!("../narrators/murloc_scribe.en.toml"),
    include_str!("../narrators/murloc_scribe.fr.toml"),
    include_str!("../narrators/grumpy_dwarf.en.toml"),
    include_str!("../narrators/grumpy_dwarf.fr.toml"),
    include_str!("../narrators/pompous_noble.en.toml"),
    include_str!("../narrators/pompous_noble.fr.toml"),
    include_str!("../narrators/smug_academic.en.toml"),
    include_str!("../narrators/smug_academic.fr.toml"),
    include_str!("../narrators/easygoing_troll.en.toml"),
    include_str!("../narrators/easygoing_troll.fr.toml"),
    include_str!("../narrators/silly_elf.en.toml"),
    include_str!("../narrators/silly_elf.fr.toml"),
    include_str!("../narrators/murloc_speak.en.toml"),
    include_str!("../narrators/murloc_speak.fr.toml"),
];

/// Reads a profile from its TOML text.
pub fn parse(text: &str) -> Result<Narrator, toml::de::Error> {
    toml::from_str(text)
}

/// Every built-in profile, in every language it has.
pub fn all() -> &'static [Narrator] {
    static ALL: OnceLock<Vec<Narrator>> = OnceLock::new();
    ALL.get_or_init(|| {
        BUILT_IN
            .iter()
            .map(|text| parse(text).expect("built-in narrator profiles are valid"))
            .collect()
    })
}

/// The narrators to choose from, once each, in shipping order.
pub fn list() -> Vec<NarratorInfo> {
    let mut seen = Vec::new();
    for n in all() {
        if !seen.iter().any(|i: &NarratorInfo| i.id == n.id) {
            seen.push(NarratorInfo {
                id: n.id.clone(),
                name: n.name.clone(),
                opt_in: n.opt_in,
                custom: false,
            });
        }
    }
    seen
}

/// The profile for `id` in `language`. A narrator without that language falls back to its
/// English profile (the prompt still asks for prose in `language`); an unknown id falls back to
/// the default narrator.
pub fn get(id: &str, language: &str) -> &'static Narrator {
    let find = |id: &str, lang: &str| all().iter().find(|n| n.id == id && n.language == lang);
    find(id, language)
        .or_else(|| find(id, "en"))
        .or_else(|| find(DEFAULT_NARRATOR, language))
        .or_else(|| find(DEFAULT_NARRATOR, "en"))
        .expect("the default narrator ships in English")
}

/// The built-ins and the player's own narrators, read from a folder.
#[derive(Debug, Clone, Default)]
pub struct Library {
    custom: Vec<Narrator>,
}

impl Library {
    /// Reads every profile in `dir`. A missing folder means none; a file that does not read is
    /// skipped with a warning, so one broken file never takes the others down.
    pub fn load(dir: &Path) -> Self {
        let mut custom = Vec::new();
        let Ok(entries) = std::fs::read_dir(dir) else {
            return Self { custom };
        };
        let mut paths: Vec<PathBuf> = entries
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|e| e == "toml"))
            .collect();
        paths.sort();
        for path in paths {
            match std::fs::read_to_string(&path)
                .map_err(|e| e.to_string())
                .and_then(|t| parse(&t).map_err(|e| e.to_string()))
            {
                Ok(n) if n.id.starts_with(CUSTOM_PREFIX) => custom.push(n),
                Ok(n) => tracing::warn!("{}: {} is not a custom narrator id", path.display(), n.id),
                Err(e) => tracing::warn!("{}: {e}", path.display()),
            }
        }
        Self { custom }
    }

    /// The profile for `id`: the player's own narrator whatever the prose language, else the
    /// built-in lookup and its fallbacks.
    pub fn get(&self, id: &str, language: &str) -> &Narrator {
        self.custom
            .iter()
            .find(|n| n.id == id)
            .unwrap_or_else(|| get(id, language))
    }

    /// The built-ins in shipping order, then the player's own by name.
    pub fn list(&self) -> Vec<NarratorInfo> {
        let mut mine: Vec<NarratorInfo> = self
            .custom
            .iter()
            .map(|n| NarratorInfo {
                id: n.id.clone(),
                name: n.name.clone(),
                opt_in: false,
                custom: true,
            })
            .collect();
        mine.sort_by_key(|n| n.name.to_lowercase());
        let mut all = list();
        all.extend(mine);
        all
    }

    /// The player's own narrator with this id.
    pub fn custom(&self, id: &str) -> Option<&Narrator> {
        self.custom.iter().find(|n| n.id == id)
    }
}

/// What is wrong with a field of a profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase")]
pub enum Problem {
    Missing,
    TooLong,
    /// The example's margin note must start with "*", as the entries' notes do.
    NoStar,
    /// A hint for a pattern the app does not know.
    UnknownHint,
}

/// One field that cannot be saved as it is: `"persona"`, `"example.entry"`, `"hints.only_fished"`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct Invalid {
    pub field: String,
    pub problem: Problem,
}

#[derive(Debug, thiserror::Error)]
pub enum NarratorError {
    #[error("the narrator has fields to fix")]
    Invalid(Vec<Invalid>),
    #[error("the file is not a narrator: {0}")]
    Unreadable(String),
    #[error("built-in narrators cannot be changed or removed")]
    BuiltIn,
    #[error("there is no narrator {0}")]
    NotFound(String),
    #[error("could not write the narrator: {0}")]
    Io(#[from] std::io::Error),
}

/// Every field that cannot be saved as it is, in form order.
pub fn validate(n: &Narrator) -> Vec<Invalid> {
    let mut problems = Vec::new();
    let mut check = |field: &str, text: &str, required: bool| {
        let problem = if required && text.trim().is_empty() {
            Some(Problem::Missing)
        } else if text.chars().count() > MAX_FIELD_CHARS {
            Some(Problem::TooLong)
        } else {
            None
        };
        if let Some(problem) = problem {
            problems.push(Invalid {
                field: field.to_string(),
                problem,
            });
        }
    };
    check("name", &n.name, true);
    check("persona", &n.persona, true);
    check("voice", &n.voice, true);
    check("murloc", &n.murloc, true);
    check("margin_note", &n.margin_note, true);
    check("example.notes", &n.example.notes, true);
    check("example.entry", &n.example.entry, true);
    check("example.margin_note", &n.example.margin_note, true);
    check("example.intro", &n.example.intro, false);
    for (key, hint) in &n.hints {
        check(&format!("hints.{key}"), hint, false);
    }
    let note = n.example.margin_note.trim();
    if !note.is_empty() && !note.starts_with('*') {
        problems.push(Invalid {
            field: "example.margin_note".into(),
            problem: Problem::NoStar,
        });
    }
    for key in n.hints.keys() {
        if !Pattern::ALL.iter().any(|p| p.key() == key) {
            problems.push(Invalid {
                field: format!("hints.{key}"),
                problem: Problem::UnknownHint,
            });
        }
    }
    problems
}

/// A built-in (or custom) narrator as the start of a new one: every field, "(mine)" after its
/// name, and no id until it is saved.
pub fn copy_of(id: &str, language: &str) -> Narrator {
    let mut copy = get(id, language).clone();
    copy.id = String::new();
    copy.name = format!("{} (mine)", copy.name);
    copy.opt_in = false;
    copy
}

/// A new narrator from scratch, not saved yet: who they are, how they sound and the example entry
/// are left for the player, while the parts every narrator shares are filled in from the scribe
/// to start from: the made-up day the example is written from, its labels and introduction, and
/// how the murloc may show.
pub fn blank(language: &str) -> Narrator {
    let scribe = get(DEFAULT_NARRATOR, language);
    Narrator {
        id: String::new(),
        language: scribe.language.clone(),
        name: String::new(),
        opt_in: false,
        persona: String::new(),
        voice: String::new(),
        murloc: scribe.murloc.clone(),
        margin_note: scribe.margin_note.clone(),
        example: Example {
            entry: String::new(),
            margin_note: String::new(),
            ..scribe.example.clone()
        },
        hints: BTreeMap::new(),
    }
}

fn slug(name: &str) -> String {
    let mut out = String::new();
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.ends_with('_') {
            out.push('_');
        }
    }
    let out = out.trim_matches('_').to_string();
    if out.is_empty() {
        "narrator".to_string()
    } else {
        out
    }
}

fn file(dir: &Path, id: &str) -> PathBuf {
    dir.join(format!("{id}.toml"))
}

/// An id no file and no built-in uses yet, from the narrator's name.
fn fresh_id(dir: &Path, name: &str) -> String {
    let base = format!("{CUSTOM_PREFIX}{}", slug(name));
    let taken = |id: &str| file(dir, id).exists() || all().iter().any(|n| n.id == id);
    if !taken(&base) {
        return base;
    }
    (2..)
        .map(|n| format!("{base}_{n}"))
        .find(|id| !taken(id))
        .expect("some suffix is free")
}

/// Stores a player's narrator and returns it with its id. A narrator already saved keeps its id;
/// a new one, a copy of a built-in, or one with a made-up id gets a fresh `custom_` id.
pub fn save(dir: &Path, mut narrator: Narrator) -> Result<Narrator, NarratorError> {
    let problems = validate(&narrator);
    if !problems.is_empty() {
        return Err(NarratorError::Invalid(problems));
    }
    let known = narrator.id.starts_with(CUSTOM_PREFIX)
        && narrator.id.len() > CUSTOM_PREFIX.len()
        && narrator.id[CUSTOM_PREFIX.len()..]
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_')
        && file(dir, &narrator.id).exists();
    if !known {
        narrator.id = fresh_id(dir, &narrator.name);
    }
    narrator.opt_in = false;
    narrator.hints.retain(|_, hint| !hint.trim().is_empty());
    std::fs::create_dir_all(dir)?;
    // Written next to the profile and renamed over it, so a crash never leaves half a file.
    let path = file(dir, &narrator.id);
    let partial = path.with_extension("toml.partial");
    std::fs::write(&partial, to_toml(&narrator))?;
    std::fs::rename(&partial, &path)?;
    Ok(narrator)
}

/// Adds a shared narrator file as the player's own, under an id of its own.
pub fn import(dir: &Path, text: &str) -> Result<Narrator, NarratorError> {
    let mut narrator = parse(text).map_err(|e| NarratorError::Unreadable(e.to_string()))?;
    narrator.id = String::new();
    save(dir, narrator)
}

/// Removes a player's narrator. Entries it wrote keep their text.
pub fn delete(dir: &Path, id: &str) -> Result<(), NarratorError> {
    if !id.starts_with(CUSTOM_PREFIX) || all().iter().any(|n| n.id == id) {
        return Err(NarratorError::BuiltIn);
    }
    match std::fs::remove_file(file(dir, id)) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            Err(NarratorError::NotFound(id.to_string()))
        }
        Err(e) => Err(e.into()),
    }
}

/// A profile as the TOML file it is shared as.
pub fn to_toml(narrator: &Narrator) -> String {
    toml::to_string_pretty(narrator).expect("a narrator always serialises")
}
