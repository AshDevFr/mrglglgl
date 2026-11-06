//! Installing, updating and removing the addon: the one place the app writes under the World of
//! Warcraft folder.
//!
//! The only paths it may create, replace or remove are `<flavor>/Interface/AddOns/Mrglglgl/` and
//! its own working folders beside it, `<flavor>/Interface/AddOns/Mrglglgl-staging-<id>/`, for a
//! discovered WoW: Forever flavor; it may create `<flavor>/Interface/AddOns/` when `Interface/`
//! exists. It never touches `WTF/`, any SavedVariables, `Screenshots/` or another addon's folder,
//! and refuses while the game runs. Every write goes through [`managed_target`].
//!
//! A package is checked completely before anything is written: its size and SHA-256, every path
//! inside one `Mrglglgl/` folder with no way out of it, allowed file types only, no links, sizes
//! within limits, and a TOC for WoW: Forever whose version is the release's and whose journal
//! schema this app reads. The new copy is written to a staging folder whose name does not match
//! its TOC, so the game would never load it, then swapped in by two renames, restored on failure.
//! The copy it replaces is kept in the app's data folder, never under `AddOns` where the game would
//! load it as a second addon fighting over the same saved variables.

use std::collections::BTreeMap;
use std::fs;
use std::io::{Cursor, Read};
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::addon_status::{self, ADDON_FOLDER, display_version};
use crate::discovery::Flavor;
use crate::schema::SUPPORTED_SCHEMA;

/// The repository releases come from; any other download address is refused.
pub const RELEASES_REPOSITORY: &str = "AshDevFr/mrglglgl";
/// Release tags of the addon start with this.
pub const TAG_PREFIX: &str = "addon-v";
/// The package for WoW: Forever ends with this.
const FOREVER_ASSET_SUFFIX: &str = "-forever.zip";

/// A package larger than this is refused before it is read.
pub const MAX_PACKAGE_BYTES: u64 = 20 * 1024 * 1024;
const MAX_ENTRIES: usize = 2000;
const MAX_UNPACKED_BYTES: u64 = 50 * 1024 * 1024;
/// File types an addon is made of.
const ALLOWED_EXTENSIONS: &[&str] = &[
    "lua", "toc", "xml", "md", "txt", "tga", "blp", "ttf", "ogg", "mp3",
];
/// Plain text files packages carry without an extension, allowed by exact name.
const ALLOWED_NAMES: &[&str] = &[
    "LICENSE",
    "LICENCE",
    "COPYING",
    "README",
    "CHANGELOG",
    "NOTICE",
    "AUTHORS",
];
/// Rollback copies kept per flavor.
const ROLLBACKS_KEPT: usize = 3;
/// The prefix of the installer's working folders under `AddOns`.
const STAGING_PREFIX: &str = "Mrglglgl-staging-";

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum InstallError {
    #[error("the game is running")]
    GameRunning,
    #[error("{0} is not a WoW: Forever folder")]
    NotForever(String),
    #[error("{0} has no Interface folder")]
    NoInterface(String),
    #[error("the addon folder is a link to a development copy")]
    DevelopmentCopy,
    #[error("the addon folder was installed by something else")]
    NotManaged,
    #[error("the addon is not installed")]
    NotInstalled,
    #[error("the package is too large")]
    TooLarge,
    #[error("the package does not match its checksum")]
    Checksum,
    #[error("the package is not a readable zip: {0}")]
    Unreadable(String),
    #[error("the package holds a path that is not allowed: {0}")]
    BadPath(String),
    #[error("the package holds a file type that is not allowed: {0}")]
    BadFile(String),
    #[error("the package has no Mrglglgl.toc")]
    NoToc,
    #[error("the package is not for WoW: Forever (interface {0})")]
    WrongInterface(String),
    #[error("the package's version {found} is not the release's {expected}")]
    WrongVersion { found: String, expected: String },
    #[error("the package writes journal schema {0}, newer than this app reads")]
    NewerSchema(i64),
    #[error("{0}")]
    Io(String),
}

impl From<std::io::Error> for InstallError {
    fn from(error: std::io::Error) -> Self {
        InstallError::Io(error.to_string())
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// A checked addon package: the files of the `Mrglglgl/` folder, relative to it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Package {
    /// The version as the TOC gives it, e.g. `addon-v0.2.0`.
    pub version: String,
    pub interface: i64,
    /// The journal schema the addon writes; packages from before the TOC named it wrote 1.
    pub schema: i64,
    pub files: BTreeMap<PathBuf, Vec<u8>>,
}

fn allowed_file(path: &Path) -> bool {
    match path.extension().and_then(|e| e.to_str()) {
        Some(extension) => ALLOWED_EXTENSIONS.contains(&extension.to_ascii_lowercase().as_str()),
        None => path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| ALLOWED_NAMES.contains(&n.to_ascii_uppercase().as_str())),
    }
}

/// A path inside the addon folder made only of plain names: no `..`, no root, no drive.
fn plain_relative(path: &Path) -> bool {
    path.components().all(|c| matches!(c, Component::Normal(_)))
        && path.components().next().is_some()
}

fn toc_field<'a>(toc: &'a str, key: &str) -> Option<&'a str> {
    toc.lines().find_map(|line| {
        let (k, v) = line.strip_prefix("##")?.split_once(':')?;
        (k.trim() == key).then(|| v.trim())
    })
}

/// Checks a package's TOC: WoW: Forever's interface, the expected version, a schema this app reads.
fn check_toc(
    files: &BTreeMap<PathBuf, Vec<u8>>,
    tag: Option<&str>,
) -> Result<Package, InstallError> {
    let toc = files
        .get(Path::new(&format!("{ADDON_FOLDER}.toc")))
        .ok_or(InstallError::NoToc)?;
    let toc = String::from_utf8_lossy(toc);
    let interface_text = toc_field(&toc, "Interface").unwrap_or_default();
    let interface: i64 = interface_text
        .parse()
        .ok()
        .filter(|i| (16000..=19999).contains(i))
        .ok_or_else(|| InstallError::WrongInterface(interface_text.to_string()))?;
    let version = toc_field(&toc, "Version").unwrap_or_default().to_string();
    if let Some(tag) = tag
        && version != tag
    {
        return Err(InstallError::WrongVersion {
            found: version,
            expected: tag.to_string(),
        });
    }
    let schema = toc_field(&toc, "X-Mrglglgl-Schema")
        .and_then(|s| s.parse().ok())
        .unwrap_or(1);
    if schema > SUPPORTED_SCHEMA {
        return Err(InstallError::NewerSchema(schema));
    }
    Ok(Package {
        version,
        interface,
        schema,
        files: files.clone(),
    })
}

/// Reads and checks a downloaded release package, writing nothing.
pub fn verify(bytes: &[u8], sha256: &str, tag: &str) -> Result<Package, InstallError> {
    if bytes.len() as u64 > MAX_PACKAGE_BYTES {
        return Err(InstallError::TooLarge);
    }
    if !sha256_hex(bytes).eq_ignore_ascii_case(sha256.trim()) {
        return Err(InstallError::Checksum);
    }
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes))
        .map_err(|e| InstallError::Unreadable(e.to_string()))?;
    if zip.len() > MAX_ENTRIES {
        return Err(InstallError::TooLarge);
    }
    let mut files = BTreeMap::new();
    let mut unpacked: u64 = 0;
    for index in 0..zip.len() {
        let mut entry = zip
            .by_index(index)
            .map_err(|e| InstallError::Unreadable(e.to_string()))?;
        let name = entry
            .name()
            .map_err(|e| InstallError::Unreadable(e.to_string()))?
            .into_owned();
        let path = PathBuf::from(&name);
        let inside = path
            .strip_prefix(ADDON_FOLDER)
            .ok()
            .filter(|_| entry.enclosed_name().is_some() && !name.contains('\\'))
            .map(Path::to_path_buf)
            .ok_or_else(|| InstallError::BadPath(name.clone()))?;
        if entry.is_symlink() {
            return Err(InstallError::BadPath(name));
        }
        if entry.is_dir() {
            if !inside.as_os_str().is_empty() && !plain_relative(&inside) {
                return Err(InstallError::BadPath(name));
            }
            continue;
        }
        if !plain_relative(&inside) {
            return Err(InstallError::BadPath(name));
        }
        if !allowed_file(&inside) {
            return Err(InstallError::BadFile(name));
        }
        let left = MAX_UNPACKED_BYTES.saturating_sub(unpacked);
        if entry.size() > left {
            return Err(InstallError::TooLarge);
        }
        let mut content = Vec::new();
        (&mut entry)
            .take(left + 1)
            .read_to_end(&mut content)
            .map_err(|e| InstallError::Unreadable(e.to_string()))?;
        unpacked += content.len() as u64;
        if unpacked > MAX_UNPACKED_BYTES {
            return Err(InstallError::TooLarge);
        }
        files.insert(inside, content);
    }
    check_toc(&files, Some(tag))
}

/// The one folder the installer writes, after checking the flavor is a discovered WoW: Forever
/// folder with an `Interface` folder, and that the addon there is not a linked development copy.
pub fn managed_target(flavor: &Flavor) -> Result<PathBuf, InstallError> {
    if !addon_status::is_forever(flavor) {
        return Err(InstallError::NotForever(flavor.name.clone()));
    }
    let interface = flavor.path.join("Interface");
    let interface_meta = fs::symlink_metadata(&interface)
        .map_err(|_| InstallError::NoInterface(flavor.name.clone()))?;
    if !interface_meta.is_dir() {
        return Err(InstallError::NoInterface(flavor.name.clone()));
    }
    let target = addon_status::addon_folder(flavor);
    if fs::symlink_metadata(&target).is_ok_and(|m| m.file_type().is_symlink()) {
        return Err(InstallError::DevelopmentCopy);
    }
    Ok(target)
}

/// What the app recorded about the copy it installed in a flavor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    pub version: String,
    pub schema: i64,
    pub installed_at: i64,
    /// Every file's SHA-256, by its path inside the addon folder.
    pub files: BTreeMap<String, String>,
}

/// Who looks after the addon folder in a flavor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase")]
pub enum Ownership {
    NotInstalled,
    /// Installed by this app, unchanged since.
    Managed,
    /// Installed by this app, then changed by something else.
    Changed,
    /// Installed by something else: an addon manager or by hand.
    Foreign,
    /// A linked folder: someone working on the addon.
    Development,
}

fn flavor_data(data_dir: &Path, flavor: &Flavor) -> PathBuf {
    data_dir.join("addon").join(flavor.name.trim_matches('_'))
}

fn manifest_path(data_dir: &Path, flavor: &Flavor) -> PathBuf {
    flavor_data(data_dir, flavor).join("manifest.json")
}

pub fn manifest(data_dir: &Path, flavor: &Flavor) -> Option<Manifest> {
    let text = fs::read_to_string(manifest_path(data_dir, flavor)).ok()?;
    serde_json::from_str(&text).ok()
}

/// Every file in a folder, by its path relative to it, with its SHA-256. Links are not followed.
fn hash_tree(root: &Path) -> std::io::Result<BTreeMap<String, String>> {
    let mut found = BTreeMap::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            let kind = entry.file_type()?;
            if kind.is_dir() {
                stack.push(entry.path());
            } else if kind.is_file() {
                let relative = entry
                    .path()
                    .strip_prefix(root)
                    .unwrap_or(&entry.path())
                    .to_path_buf();
                found.insert(
                    relative.to_string_lossy().replace('\\', "/"),
                    sha256_hex(&fs::read(entry.path())?),
                );
            }
        }
    }
    Ok(found)
}

/// Who looks after the addon folder in a flavor, comparing it with what the app installed.
pub fn ownership(data_dir: &Path, flavor: &Flavor) -> Ownership {
    let target = addon_status::addon_folder(flavor);
    match fs::symlink_metadata(&target) {
        Err(_) => Ownership::NotInstalled,
        Ok(meta) if meta.file_type().is_symlink() => Ownership::Development,
        Ok(_) => match manifest(data_dir, flavor) {
            None => Ownership::Foreign,
            Some(manifest) => match hash_tree(&target) {
                Ok(files) if files == manifest.files => Ownership::Managed,
                _ => Ownership::Changed,
            },
        },
    }
}

fn unique_suffix() -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default();
    format!("{:x}{:x}", nanos, std::process::id())
}

/// Removes working folders an interrupted install left beside the addon. Only real folders whose
/// name starts with the installer's prefix; a link is unlinked, never followed.
fn clean_staging(addons: &Path) -> std::io::Result<()> {
    let Ok(entries) = fs::read_dir(addons) else {
        return Ok(());
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if !name.starts_with(STAGING_PREFIX) {
            continue;
        }
        let kind = entry.file_type()?;
        if kind.is_symlink() || kind.is_file() {
            fs::remove_file(entry.path())?;
        } else if kind.is_dir() {
            fs::remove_dir_all(entry.path())?;
        }
    }
    Ok(())
}

fn copy_tree(from: &Path, to: &Path) -> std::io::Result<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        let dest = to.join(entry.file_name());
        if kind.is_dir() {
            copy_tree(&entry.path(), &dest)?;
        } else if kind.is_file() {
            fs::copy(entry.path(), dest)?;
        }
    }
    Ok(())
}

/// A copy of an earlier install, kept in the app's data folder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase")]
pub struct Rollback {
    pub path: PathBuf,
    /// The version shown to the player, when the copy had one.
    pub version: Option<String>,
    /// Unix time the copy was taken.
    pub taken: i64,
}

fn rollback_dir(data_dir: &Path, flavor: &Flavor) -> PathBuf {
    flavor_data(data_dir, flavor).join("rollback")
}

/// Copies kept of earlier installs in a flavor, newest first.
pub fn rollbacks(data_dir: &Path, flavor: &Flavor) -> Vec<Rollback> {
    let mut found: Vec<Rollback> = fs::read_dir(rollback_dir(data_dir, flavor))
        .map(|entries| {
            entries
                .flatten()
                .filter(|e| e.file_type().is_ok_and(|k| k.is_dir()))
                .filter_map(|e| {
                    let name = e.file_name().to_string_lossy().into_owned();
                    let (taken, version) = name.split_once('-')?;
                    Some(Rollback {
                        path: e.path(),
                        version: display_version(version),
                        taken: taken.parse().ok()?,
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    found.sort_by(|a, b| b.taken.cmp(&a.taken).then(b.path.cmp(&a.path)));
    found
}

/// Keeps a copy of the installed addon in the data folder, then drops the oldest beyond three.
fn keep_rollback(data_dir: &Path, flavor: &Flavor, target: &Path, now: i64) -> std::io::Result<()> {
    let version = addon_status::installed(flavor)
        .map(|i| {
            if i.development {
                "development".to_string()
            } else {
                i.version.unwrap_or_else(|| "unknown".to_string())
            }
        })
        .unwrap_or_else(|| "unknown".to_string());
    let safe: String = version
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '.' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let dest = rollback_dir(data_dir, flavor).join(format!("{now}-{safe}"));
    if dest.exists() {
        fs::remove_dir_all(&dest)?;
    }
    copy_tree(target, &dest)?;
    for old in rollbacks(data_dir, flavor).into_iter().skip(ROLLBACKS_KEPT) {
        fs::remove_dir_all(old.path)?;
    }
    Ok(())
}

/// What an install did.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase")]
pub struct Installed {
    /// The version shown to the player.
    pub version: Option<String>,
    /// Whether an earlier copy was replaced (and kept in the data folder).
    pub replaced: bool,
}

/// How far an install may go with a folder it did not put there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Takeover {
    /// Refuse to replace or remove a copy installed by something else.
    Refuse,
    /// The player chose to let the app look after that copy from now on.
    Allow,
}

/// Installs or updates the addon in a flavor from a checked package.
pub fn install(
    flavor: &Flavor,
    package: &Package,
    data_dir: &Path,
    game_running: bool,
    takeover: Takeover,
    now: i64,
) -> Result<Installed, InstallError> {
    if game_running {
        return Err(InstallError::GameRunning);
    }
    let target = managed_target(flavor)?;
    let owner = ownership(data_dir, flavor);
    if owner == Ownership::Foreign && takeover == Takeover::Refuse {
        return Err(InstallError::NotManaged);
    }
    let addons = target
        .parent()
        .expect("the addon folder has a parent")
        .to_path_buf();
    fs::create_dir_all(&addons)?;
    clean_staging(&addons)?;

    let id = unique_suffix();
    let staging = addons.join(format!("{STAGING_PREFIX}{id}"));
    let write = || -> std::io::Result<()> {
        for (path, content) in &package.files {
            let dest = staging.join(path);
            fs::create_dir_all(dest.parent().expect("a file has a folder"))?;
            fs::write(dest, content)?;
        }
        Ok(())
    };
    if let Err(error) = write() {
        let _ = fs::remove_dir_all(&staging);
        return Err(error.into());
    }

    let replaced = target.exists();
    if replaced {
        keep_rollback(data_dir, flavor, &target, now)?;
        let old = addons.join(format!("{STAGING_PREFIX}{id}-old"));
        fs::rename(&target, &old)?;
        if let Err(error) = fs::rename(&staging, &target) {
            // Put the old copy back so the game finds an addon where it left one.
            let _ = fs::rename(&old, &target);
            let _ = fs::remove_dir_all(&staging);
            return Err(error.into());
        }
        fs::remove_dir_all(&old)?;
    } else if let Err(error) = fs::rename(&staging, &target) {
        let _ = fs::remove_dir_all(&staging);
        return Err(error.into());
    }

    let record = Manifest {
        version: package.version.clone(),
        schema: package.schema,
        installed_at: now,
        files: hash_tree(&target)?,
    };
    let path = manifest_path(data_dir, flavor);
    fs::create_dir_all(path.parent().expect("the manifest has a folder"))?;
    fs::write(
        &path,
        serde_json::to_string_pretty(&record).map_err(|e| InstallError::Io(e.to_string()))?,
    )?;
    Ok(Installed {
        version: display_version(&package.version),
        replaced,
    })
}

/// Reads a kept copy back as a package, to install it again.
pub fn rollback_package(rollback: &Path) -> Result<Package, InstallError> {
    let mut files = BTreeMap::new();
    for relative in hash_tree(rollback)?.into_keys() {
        let path = PathBuf::from(&relative);
        if !plain_relative(&path) || !allowed_file(&path) {
            return Err(InstallError::BadFile(relative));
        }
        files.insert(path.clone(), fs::read(rollback.join(&path))?);
    }
    check_toc(&files, None)
}

/// Removes the addon folder from a flavor. Recorded journals stay where they are: in the game's
/// `WTF` folder and in the app's archive. A copy is kept in the data folder first.
pub fn remove(
    flavor: &Flavor,
    data_dir: &Path,
    game_running: bool,
    takeover: Takeover,
    now: i64,
) -> Result<(), InstallError> {
    if game_running {
        return Err(InstallError::GameRunning);
    }
    let target = managed_target(flavor)?;
    match ownership(data_dir, flavor) {
        Ownership::NotInstalled => return Err(InstallError::NotInstalled),
        Ownership::Foreign if takeover == Takeover::Refuse => {
            return Err(InstallError::NotManaged);
        }
        _ => {}
    }
    keep_rollback(data_dir, flavor, &target, now)?;
    let addons = target.parent().expect("the addon folder has a parent");
    let old = addons.join(format!("{STAGING_PREFIX}{}-old", unique_suffix()));
    fs::rename(&target, &old)?;
    fs::remove_dir_all(&old)?;
    let _ = fs::remove_file(manifest_path(data_dir, flavor));
    Ok(())
}

/// A published release of the addon.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase")]
pub struct Release {
    pub tag: String,
    /// The version shown to the player.
    pub version: String,
    pub prerelease: bool,
    pub url: String,
    pub size: u64,
    pub sha256: String,
    pub published: Option<String>,
}

#[derive(Deserialize)]
struct ApiRelease {
    tag_name: String,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
    published_at: Option<String>,
    #[serde(default)]
    assets: Vec<ApiAsset>,
}

#[derive(Deserialize)]
struct ApiAsset {
    name: String,
    size: u64,
    browser_download_url: String,
    digest: Option<String>,
}

/// The newest release of the addon in a GitHub releases listing: a stable one, or with
/// `prereleases` a test one too. Only packages for WoW: Forever, from the pinned repository, with
/// a SHA-256 digest, are considered.
pub fn latest_release(listing: &str, prereleases: bool) -> Option<Release> {
    let releases: Vec<ApiRelease> = serde_json::from_str(listing).ok()?;
    let prefix = format!("https://github.com/{RELEASES_REPOSITORY}/releases/download/");
    releases
        .into_iter()
        .filter(|r| !r.draft && r.tag_name.starts_with(TAG_PREFIX))
        .filter(|r| prereleases || !r.prerelease)
        .filter_map(|r| {
            let asset = r.assets.into_iter().find(|a| {
                a.name.ends_with(FOREVER_ASSET_SUFFIX)
                    && a.browser_download_url.starts_with(&prefix)
                    && a.size <= MAX_PACKAGE_BYTES
            })?;
            let sha256 = asset
                .digest?
                .strip_prefix("sha256:")
                .filter(|d| d.len() == 64 && d.chars().all(|c| c.is_ascii_hexdigit()))?
                .to_ascii_lowercase();
            Some(Release {
                version: display_version(&r.tag_name)?,
                tag: r.tag_name,
                prerelease: r.prerelease,
                url: asset.browser_download_url,
                size: asset.size,
                sha256,
                published: r.published_at,
            })
        })
        .max_by(|a, b| compare_versions(&a.version, &b.version))
}

fn split_version(version: &str) -> (Vec<u64>, Option<&str>) {
    let (core, pre) = match version.split_once('-') {
        Some((core, pre)) => (core, Some(pre)),
        None => (version, None),
    };
    (
        core.split('.').map(|p| p.parse().unwrap_or(0)).collect(),
        pre,
    )
}

/// Orders versions like `0.2.0`, `0.2.0-beta.3`: numbers first, then a release above its test
/// versions, then test versions by their numbered parts.
pub fn compare_versions(a: &str, b: &str) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    let (core_a, pre_a) = split_version(a);
    let (core_b, pre_b) = split_version(b);
    let len = core_a.len().max(core_b.len());
    for i in 0..len {
        let order = core_a.get(i).unwrap_or(&0).cmp(core_b.get(i).unwrap_or(&0));
        if order != Ordering::Equal {
            return order;
        }
    }
    match (pre_a, pre_b) {
        (None, None) => Ordering::Equal,
        (None, Some(_)) => Ordering::Greater,
        (Some(_), None) => Ordering::Less,
        (Some(a), Some(b)) => {
            let parts_a: Vec<&str> = a.split('.').collect();
            let parts_b: Vec<&str> = b.split('.').collect();
            for (x, y) in parts_a.iter().zip(&parts_b) {
                let order = match (x.parse::<u64>(), y.parse::<u64>()) {
                    (Ok(x), Ok(y)) => x.cmp(&y),
                    _ => x.cmp(y),
                };
                if order != Ordering::Equal {
                    return order;
                }
            }
            parts_a.len().cmp(&parts_b.len())
        }
    }
}
