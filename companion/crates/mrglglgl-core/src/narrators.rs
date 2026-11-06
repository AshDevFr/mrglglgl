//! Narrator profiles: the persona a prompt is written in. Each profile is a TOML data file in
//! `narrators/`, one per narrator and prose language, so new voices and translations need no code.
//! The format is meant to be the one community profiles use later.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

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

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
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
}

const BUILT_IN: &[&str] = &[
    include_str!("../narrators/murloc_scribe.en.toml"),
    include_str!("../narrators/murloc_scribe.fr.toml"),
    include_str!("../narrators/grumpy_dwarf.en.toml"),
    include_str!("../narrators/pompous_noble.en.toml"),
    include_str!("../narrators/smug_academic.en.toml"),
    include_str!("../narrators/murloc_speak.en.toml"),
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
