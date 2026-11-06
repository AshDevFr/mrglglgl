//! Whether the addon is installed in a game flavor, and whether it is recording: what the
//! installed copy says about itself (its TOC), what the game client is (`.flavor.info` and
//! `.build.info`), and what the addon last noted in a journal at login. Nothing here writes to
//! disk.

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::discovery::Flavor;
use crate::schema::{AddonErrors, AddonHeartbeat, SUPPORTED_SCHEMA};

/// The addon's folder under `Interface/AddOns`, and the name of its TOC file.
pub const ADDON_FOLDER: &str = "Mrglglgl";

/// The product code `.flavor.info` and `.build.info` give WoW: Forever.
pub const FOREVER_PRODUCT: &str = "wowf";

/// A flavor whose last login is older than this is quiet: the addon may be disabled.
pub const QUIET_AFTER_SECONDS: i64 = 7 * 24 * 60 * 60;

/// Addon errors this recent are shown; older ones were seen already or are fixed.
pub const RECENT_ERRORS_SECONDS: i64 = 7 * 24 * 60 * 60;

/// The packaging placeholder a copy taken straight from the source tree still carries.
const VERSION_PLACEHOLDER: &str = "@project-version@";

/// What the installed copy of the addon says about itself.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase")]
pub struct InstalledAddon {
    /// The release version, without the `addon-v` tag prefix; absent for a development copy.
    pub version: Option<String>,
    /// The client interface the TOC was written for, e.g. 16001.
    pub interface: Option<i64>,
    /// The journal schema the addon writes (`X-Mrglglgl-Schema`).
    pub schema: Option<i64>,
    /// A linked folder or an unpackaged copy of the source: someone working on the addon.
    pub development: bool,
}

/// The most recent login the addon noted in any journal of the flavor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase")]
pub struct LastRun {
    /// The character's name, as the journal gives it.
    pub character: String,
    /// The version that ran, without the tag prefix; absent for a development copy.
    pub version: Option<String>,
    /// The client's interface at that login.
    pub interface: Option<i64>,
    /// Unix time of the login.
    pub login: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase")]
pub enum AddonState {
    NotInstalled,
    /// Installed, and no journal in this flavor says it has run yet.
    Waiting,
    /// Ran within the last week.
    Recording,
    /// Installed, but nothing noted for over a week.
    Quiet,
}

/// Something the player should look at, with a next step for each.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum AddonProblem {
    /// The installed addon writes a journal this app cannot read.
    NewerSchema { schema: i64, supported: i64 },
    /// A different version is installed than the one that last ran: it loads after a /reload or
    /// the next login.
    UpdatePending { running: String, installed: String },
    /// The client is newer than the addon's TOC: the game lists it as out of date.
    OutOfDate { addon: i64, client: i64 },
    /// The addon caught errors recently; recording carried on.
    Errors {
        count: i64,
        message: Option<String>,
        time: Option<i64>,
    },
}

/// The addon in one Forever flavor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase")]
pub struct FlavorAddon {
    pub flavor: String,
    /// Where the addon is, or would be, installed.
    pub folder: PathBuf,
    /// The game client's version, e.g. `1.60.1.70790`.
    pub client_version: Option<String>,
    pub installed: Option<InstalledAddon>,
    pub last_run: Option<LastRun>,
    pub state: AddonState,
    pub problems: Vec<AddonProblem>,
}

/// A version as the player knows it: `addon-v0.1.0` is shown as `0.1.0`; the packaging
/// placeholder of a development copy is no version at all.
pub fn display_version(raw: &str) -> Option<String> {
    let raw = raw.trim();
    if raw.is_empty() || raw == VERSION_PLACEHOLDER {
        return None;
    }
    Some(raw.strip_prefix("addon-v").unwrap_or(raw).to_string())
}

/// Reads the `## Key: value` header lines of a TOC.
fn toc_field<'a>(toc: &'a str, key: &str) -> Option<&'a str> {
    toc.lines().find_map(|line| {
        let (k, v) = line.strip_prefix("##")?.split_once(':')?;
        (k.trim() == key).then(|| v.trim())
    })
}

/// The addon's folder in a flavor.
pub fn addon_folder(flavor: &Flavor) -> PathBuf {
    flavor
        .path
        .join("Interface")
        .join("AddOns")
        .join(ADDON_FOLDER)
}

/// The installed copy of the addon in a flavor, read from its TOC; `None` when it is not there.
pub fn installed(flavor: &Flavor) -> Option<InstalledAddon> {
    let folder = addon_folder(flavor);
    let toc = std::fs::read_to_string(folder.join(format!("{ADDON_FOLDER}.toc"))).ok()?;
    let linked = std::fs::symlink_metadata(&folder).is_ok_and(|m| m.file_type().is_symlink());
    let raw_version = toc_field(&toc, "Version").unwrap_or_default();
    let version = display_version(raw_version);
    Some(InstalledAddon {
        development: linked || raw_version.trim() == VERSION_PLACEHOLDER,
        version,
        interface: toc_field(&toc, "Interface").and_then(|v| v.parse().ok()),
        schema: toc_field(&toc, "X-Mrglglgl-Schema").and_then(|v| v.parse().ok()),
    })
}

/// The product code of a flavor folder, from its `.flavor.info`.
pub fn product(flavor: &Flavor) -> Option<String> {
    let info = std::fs::read_to_string(flavor.path.join(".flavor.info")).ok()?;
    // A header line naming the column, then the value.
    info.lines()
        .skip(1)
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(str::to_string)
}

/// Whether a flavor folder is WoW: Forever's. Without a `.flavor.info`, the folder name decides.
pub fn is_forever(flavor: &Flavor) -> bool {
    match product(flavor) {
        Some(product) => product == FOREVER_PRODUCT,
        None => flavor.name == "_forever_",
    }
}

/// The client version of a product, from the install's `.build.info`: a `|`-separated table
/// whose header names each column `Name!TYPE:size`.
pub fn client_version(install: &Path, product: &str) -> Option<String> {
    let info = std::fs::read_to_string(install.join(".build.info")).ok()?;
    let mut lines = info.lines();
    let header: Vec<&str> = lines
        .next()?
        .split('|')
        .map(|column| column.split('!').next().unwrap_or(column))
        .collect();
    let at = |name: &str| header.iter().position(|column| *column == name);
    let (product_at, version_at) = (at("Product")?, at("Version")?);
    lines.find_map(|line| {
        let fields: Vec<&str> = line.split('|').collect();
        (fields.get(product_at) == Some(&product))
            .then(|| fields.get(version_at).map(|v| v.to_string()))
            .flatten()
            .filter(|v| !v.is_empty())
    })
}

/// One character's notes from the addon, as archived.
pub struct CharacterRun<'a> {
    pub character: &'a str,
    pub heartbeat: Option<&'a AddonHeartbeat>,
    pub errors: Option<&'a AddonErrors>,
}

/// The addon's state in one flavor, at `now`.
pub fn assess(
    flavor: &Flavor,
    client_version: Option<String>,
    installed: Option<InstalledAddon>,
    runs: &[CharacterRun<'_>],
    now: i64,
) -> FlavorAddon {
    let last_run = runs
        .iter()
        .filter_map(|run| {
            let heartbeat = run.heartbeat?;
            Some(LastRun {
                character: run.character.to_string(),
                version: heartbeat.version.as_deref().and_then(display_version),
                interface: heartbeat.interface,
                login: heartbeat.login?,
            })
        })
        .max_by_key(|run| run.login);

    let state = match (&installed, &last_run) {
        (None, _) => AddonState::NotInstalled,
        (Some(_), None) => AddonState::Waiting,
        (Some(_), Some(run)) if now - run.login > QUIET_AFTER_SECONDS => AddonState::Quiet,
        (Some(_), Some(_)) => AddonState::Recording,
    };

    let mut problems = Vec::new();
    if let Some(addon) = &installed {
        if let Some(schema) = addon.schema.filter(|schema| *schema > SUPPORTED_SCHEMA) {
            problems.push(AddonProblem::NewerSchema {
                schema,
                supported: SUPPORTED_SCHEMA,
            });
        }
        if let Some(run) = &last_run {
            if let (Some(running), Some(on_disk)) = (&run.version, &addon.version)
                && running != on_disk
            {
                problems.push(AddonProblem::UpdatePending {
                    running: running.clone(),
                    installed: on_disk.clone(),
                });
            }
            if let (Some(toc), Some(client)) = (addon.interface, run.interface)
                && toc < client
            {
                problems.push(AddonProblem::OutOfDate { addon: toc, client });
            }
        }
    }
    let recent = runs
        .iter()
        .filter_map(|run| run.errors)
        .filter(|errors| {
            errors.count > 0
                && errors
                    .last_time
                    .is_some_and(|time| now - time <= RECENT_ERRORS_SECONDS)
        })
        .max_by_key(|errors| errors.last_time);
    if let Some(errors) = recent {
        problems.push(AddonProblem::Errors {
            count: runs
                .iter()
                .filter_map(|run| run.errors)
                .map(|e| e.count)
                .sum(),
            message: errors.last_message.clone(),
            time: errors.last_time,
        });
    }

    FlavorAddon {
        flavor: flavor.name.clone(),
        folder: addon_folder(flavor),
        client_version,
        installed,
        last_run,
        state,
        problems,
    }
}
