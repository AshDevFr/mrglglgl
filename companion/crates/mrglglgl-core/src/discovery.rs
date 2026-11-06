//! Finding World of Warcraft installs, their flavor folders and the characters with a Mrglglgl
//! journal.
//!
//! Folder names below `WTF/Account/` are treated as opaque: on WoW: Forever the realm level is a
//! number that is not the realm ID, and character folders are `Firstname-Surname`. The realm,
//! name and surname shown to the player come from the journal itself. Nothing here writes to disk.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde::Serialize;

/// The journal file the addon's SavedVariables are written to.
pub const JOURNAL_FILE: &str = "Mrglglgl.lua";

/// Where an install's path came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase")]
pub enum RootSource {
    /// Chosen by the player in settings. Always wins.
    Override,
    /// The Windows registry. Unverified on real Windows hardware.
    Registry,
    /// The Battle.net launcher's configuration file. Unverified on real Windows hardware.
    BattleNetConfig,
    /// A usual install location.
    DefaultPath,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase")]
pub struct WowRoot {
    pub path: PathBuf,
    pub source: RootSource,
}

/// One game flavor's folder, such as `_forever_` or `_classic_era_`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase")]
pub struct Flavor {
    pub name: String,
    pub path: PathBuf,
}

/// A character with a Mrglglgl journal. The folder names identify it on disk only.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase")]
pub struct CharacterSource {
    pub flavor: String,
    pub account: String,
    pub realm_folder: String,
    pub character_folder: String,
    pub journal_path: PathBuf,
}

/// Settings that steer discovery.
#[derive(Debug, Clone, Default)]
pub struct DiscoveryOverrides {
    /// A WoW folder chosen by the player. The game folder or one of its flavor folders both work.
    pub wow_root: Option<PathBuf>,
}

/// True for a flavor folder name: `_forever_`, `_retail_`, `_classic_era_` and any flavor to come.
fn is_flavor_name(name: &str) -> bool {
    name.len() > 2 && name.starts_with('_') && name.ends_with('_')
}

fn is_flavor_dir(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .is_some_and(is_flavor_name)
        && path.join("WTF").is_dir()
}

/// The game folder for a path the player picked or a source reported: the folder itself when it
/// holds flavor folders, its parent when it is a flavor folder. `None` when neither.
pub fn normalize_root(path: &Path) -> Option<PathBuf> {
    if is_flavor_dir(path) {
        return path.parent().map(Path::to_path_buf);
    }
    let has_flavor = std::fs::read_dir(path)
        .ok()?
        .flatten()
        .any(|entry| is_flavor_dir(&entry.path()));
    has_flavor.then(|| path.to_path_buf())
}

/// Install paths from the Windows registry, read through `read(key, value)`. The keys are the
/// ones addon managers commonly read; they are unverified on real hardware.
pub fn roots_from_registry(read: impl Fn(&str, &str) -> Option<String>) -> Vec<PathBuf> {
    const KEYS: &[(&str, &str)] = &[
        (
            r"HKEY_LOCAL_MACHINE\SOFTWARE\WOW6432Node\Blizzard Entertainment\World of Warcraft",
            "InstallPath",
        ),
        (
            r"HKEY_LOCAL_MACHINE\SOFTWARE\Blizzard Entertainment\World of Warcraft",
            "InstallPath",
        ),
        (
            r"HKEY_LOCAL_MACHINE\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\World of Warcraft",
            "InstallLocation",
        ),
    ];
    KEYS.iter()
        .filter_map(|(key, value)| read(key, value))
        .map(|p| PathBuf::from(p.trim_end_matches(['\\', '/'])))
        .collect()
}

/// Paths mentioned anywhere in the Battle.net launcher's configuration (JSON). The file's layout
/// is not documented, so rather than rely on particular keys this takes every string that looks
/// like a path; `discover` keeps only the ones that turn out to be WoW folders.
pub fn roots_from_battlenet_config(config: &str) -> Vec<PathBuf> {
    fn collect(value: &serde_json::Value, out: &mut Vec<PathBuf>) {
        match value {
            serde_json::Value::String(s) if s.contains(['\\', '/']) => {
                let path = PathBuf::from(s);
                out.push(path.join("World of Warcraft"));
                out.push(path);
            }
            serde_json::Value::Array(items) => items.iter().for_each(|v| collect(v, out)),
            serde_json::Value::Object(map) => map.values().for_each(|v| collect(v, out)),
            _ => {}
        }
    }
    let mut out = Vec::new();
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(config) {
        collect(&value, &mut out);
    }
    out
}

/// The usual install locations for this OS.
pub fn default_paths() -> Vec<PathBuf> {
    if cfg!(target_os = "macos") {
        vec![PathBuf::from("/Applications/World of Warcraft")]
    } else if cfg!(windows) {
        vec![
            PathBuf::from(r"C:\Program Files (x86)\World of Warcraft"),
            PathBuf::from(r"C:\Program Files\World of Warcraft"),
        ]
    } else {
        Vec::new()
    }
}

/// The candidate paths this machine offers, by source, before checking them.
pub fn system_candidates() -> Vec<(PathBuf, RootSource)> {
    let mut candidates = Vec::new();
    #[cfg(windows)]
    {
        candidates.extend(
            roots_from_registry(windows::read_registry)
                .into_iter()
                .map(|p| (p, RootSource::Registry)),
        );
        if let Some(config) = std::env::var_os("APPDATA")
            .map(|appdata| PathBuf::from(appdata).join(r"Battle.net\Battle.net.config"))
            .and_then(|path| std::fs::read_to_string(path).ok())
        {
            candidates.extend(
                roots_from_battlenet_config(&config)
                    .into_iter()
                    .map(|p| (p, RootSource::BattleNetConfig)),
            );
        }
    }
    candidates.extend(
        default_paths()
            .into_iter()
            .map(|p| (p, RootSource::DefaultPath)),
    );
    candidates
}

/// WoW installs found from `candidates`, de-duplicated, in the order given. A player's override
/// always wins: when it is set, it is the only root considered.
pub fn discover_from(
    overrides: &DiscoveryOverrides,
    candidates: Vec<(PathBuf, RootSource)>,
) -> Vec<WowRoot> {
    if let Some(path) = &overrides.wow_root {
        return normalize_root(path)
            .map(|path| WowRoot {
                path,
                source: RootSource::Override,
            })
            .into_iter()
            .collect();
    }
    let mut seen = BTreeSet::new();
    candidates
        .into_iter()
        .filter_map(|(path, source)| Some((normalize_root(&path)?, source)))
        .filter(|(path, _)| seen.insert(path.clone()))
        .map(|(path, source)| WowRoot { path, source })
        .collect()
}

/// WoW installs on this machine.
pub fn discover(overrides: &DiscoveryOverrides) -> Vec<WowRoot> {
    discover_from(overrides, system_candidates())
}

fn sorted_dirs(path: &Path) -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(path)
        .map(|entries| {
            entries
                .flatten()
                .map(|entry| entry.path())
                .filter(|p| p.is_dir())
                .collect()
        })
        .unwrap_or_default();
    dirs.sort();
    dirs
}

fn name_of(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// The flavor folders in a WoW install: every `_name_` folder that holds a `WTF` folder. No
/// flavor name is assumed.
pub fn flavors(root: &WowRoot) -> Vec<Flavor> {
    sorted_dirs(&root.path)
        .into_iter()
        .filter(|p| is_flavor_dir(p))
        .map(|path| Flavor {
            name: name_of(&path),
            path,
        })
        .collect()
}

/// Characters in a flavor that have a Mrglglgl journal, walking
/// `WTF/Account/<account>/<realm folder>/<character folder>/SavedVariables/Mrglglgl.lua`.
/// Account-level folders that are not realms (the account's own `SavedVariables`) are skipped.
pub fn characters(flavor: &Flavor) -> Vec<CharacterSource> {
    let mut found = Vec::new();
    for account in sorted_dirs(&flavor.path.join("WTF").join("Account")) {
        for realm in sorted_dirs(&account) {
            if name_of(&realm) == "SavedVariables" {
                continue;
            }
            for character in sorted_dirs(&realm) {
                let journal_path = character.join("SavedVariables").join(JOURNAL_FILE);
                if journal_path.is_file() {
                    found.push(CharacterSource {
                        flavor: flavor.name.clone(),
                        account: name_of(&account),
                        realm_folder: name_of(&realm),
                        character_folder: name_of(&character),
                        journal_path,
                    });
                }
            }
        }
    }
    found
}

/// The account-wide settings file for an account, when the addon has written one.
pub fn account_settings_path(flavor: &Flavor, account: &str) -> Option<PathBuf> {
    let path = flavor
        .path
        .join("WTF")
        .join("Account")
        .join(account)
        .join("SavedVariables")
        .join(JOURNAL_FILE);
    path.is_file().then_some(path)
}

#[cfg(windows)]
mod windows {
    use winreg::RegKey;
    use winreg::enums::HKEY_LOCAL_MACHINE;

    /// Reads a string value under HKEY_LOCAL_MACHINE.
    pub(super) fn read_registry(key: &str, value: &str) -> Option<String> {
        let subkey = key.strip_prefix(r"HKEY_LOCAL_MACHINE\")?;
        RegKey::predef(HKEY_LOCAL_MACHINE)
            .open_subkey(subkey)
            .ok()?
            .get_value(value)
            .ok()
    }
}
