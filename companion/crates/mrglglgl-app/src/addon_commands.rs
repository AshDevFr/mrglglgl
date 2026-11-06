//! Looking up the addon's releases and installing, updating or removing it in a game flavor. The
//! writes themselves, and every check that guards them, live in `mrglglgl_core::addon_install`;
//! this module finds the flavor, downloads the package and tells the core whether the game runs.

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use futures_util::StreamExt;
use mrglglgl_core::addon_install::{
    self, Installed, MAX_PACKAGE_BYTES, Ownership, RELEASES_REPOSITORY, Release, Rollback, Takeover,
};
use mrglglgl_core::addon_status::{self, FlavorAddon};
use mrglglgl_core::discovery::{self, DiscoveryOverrides, Flavor};
use mrglglgl_core::watch::now_unix;
use serde::Serialize;
use tauri::State;
use ts_rs::TS;

use crate::sync::AppState;
use crate::writing::game_running;

/// The releases listing is fetched at most once a day unless the player asks.
const CACHE_FOR: Duration = Duration::from_secs(24 * 60 * 60);
const TIMEOUT: Duration = Duration::from_secs(30);

static LISTING: Mutex<Option<(Instant, i64, String)>> = Mutex::new(None);

fn client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent(concat!("mrglglgl-app/", env!("CARGO_PKG_VERSION")))
        .timeout(TIMEOUT)
        .build()
        .map_err(|e| e.to_string())
}

/// The GitHub releases listing, from the cache when it is less than a day old.
async fn listing(force: bool) -> Result<(String, i64), String> {
    if !force
        && let Some((at, checked, text)) = LISTING.lock().map_err(|e| e.to_string())?.clone()
        && at.elapsed() < CACHE_FOR
    {
        return Ok((text, checked));
    }
    let url = format!("https://api.github.com/repos/{RELEASES_REPOSITORY}/releases?per_page=30");
    let response = client()?
        .get(url)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?;
    let text = response.text().await.map_err(|e| e.to_string())?;
    let checked = now_unix();
    *LISTING.lock().map_err(|e| e.to_string())? = Some((Instant::now(), checked, text.clone()));
    Ok((text, checked))
}

/// The newest release for the player's channel, and the flavors whose installed copy is older.
#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ReleaseCheck {
    pub release: Option<Release>,
    /// Whether test versions are offered.
    pub prereleases: bool,
    /// Flavor folders whose installed addon is older than the release.
    pub updates: Vec<PathBuf>,
    /// Unix time of the listing.
    pub checked: i64,
}

/// Every WoW: Forever flavor the app can see with the current settings.
fn forever_flavors(state: &AppState) -> Result<Vec<Flavor>, String> {
    let settings = state
        .db
        .lock()
        .map_err(|e| e.to_string())?
        .settings()
        .map_err(|e| e.to_string())?;
    Ok(discovery::discover(&DiscoveryOverrides {
        wow_root: settings.wow_root_override,
    })
    .iter()
    .flat_map(discovery::flavors)
    .filter(addon_status::is_forever)
    .collect())
}

/// The discovered flavor at `path`; a path the app did not find itself is refused.
fn find_flavor(state: &AppState, path: &Path) -> Result<Flavor, String> {
    forever_flavors(state)?
        .into_iter()
        .find(|flavor| flavor.path == path)
        .ok_or_else(|| {
            format!(
                "{} is not a WoW: Forever folder the app found",
                path.display()
            )
        })
}

#[tauri::command]
pub async fn addon_release(
    state: State<'_, AppState>,
    force: bool,
) -> Result<ReleaseCheck, String> {
    let prereleases = state
        .db
        .lock()
        .map_err(|e| e.to_string())?
        .addon_prereleases()
        .map_err(|e| e.to_string())?;
    let (text, checked) = listing(force).await?;
    let release = addon_install::latest_release(&text, prereleases);
    let updates = match &release {
        Some(release) => forever_flavors(&state)?
            .into_iter()
            .filter(|flavor| {
                addon_status::installed(flavor).is_some_and(|installed| {
                    !installed.development
                        && installed.version.as_deref().is_none_or(|v| {
                            addon_install::compare_versions(v, &release.version).is_lt()
                        })
                })
            })
            .map(|flavor| flavor.path)
            .collect(),
        None => Vec::new(),
    };
    Ok(ReleaseCheck {
        release,
        prereleases,
        updates,
        checked,
    })
}

#[tauri::command]
pub fn set_addon_prereleases(state: State<'_, AppState>, on: bool) -> Result<(), String> {
    state
        .db
        .lock()
        .map_err(|e| e.to_string())?
        .set_addon_prereleases(on)
        .map_err(|e| e.to_string())
}

/// The addon in one flavor, with who looks after it and the copy that could be put back.
#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AddonView {
    pub status: FlavorAddon,
    pub flavor_path: PathBuf,
    pub ownership: Ownership,
    /// The newest copy kept from an earlier install.
    pub rollback: Option<Rollback>,
}

#[tauri::command]
pub fn addon_views(state: State<'_, AppState>) -> Result<Vec<AddonView>, String> {
    let statuses = crate::commands::addon_statuses(&state)?;
    Ok(statuses
        .into_iter()
        .map(|(flavor, status)| AddonView {
            ownership: addon_install::ownership(&state.data_dir, &flavor),
            rollback: addon_install::rollbacks(&state.data_dir, &flavor)
                .into_iter()
                .next(),
            flavor_path: flavor.path,
            status,
        })
        .collect())
}

fn takeover(take_over: bool) -> Takeover {
    if take_over {
        Takeover::Allow
    } else {
        Takeover::Refuse
    }
}

async fn download(release: &Release) -> Result<Vec<u8>, String> {
    let response = client()?
        .get(&release.url)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?;
    if response
        .content_length()
        .is_some_and(|len| len > MAX_PACKAGE_BYTES)
    {
        return Err("the package is too large".into());
    }
    let mut bytes = Vec::new();
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        bytes.extend_from_slice(&chunk.map_err(|e| e.to_string())?);
        if bytes.len() as u64 > MAX_PACKAGE_BYTES {
            return Err("the package is too large".into());
        }
    }
    Ok(bytes)
}

/// Installs or updates the addon in a flavor from the newest release of the player's channel.
#[tauri::command]
pub async fn install_addon(
    state: State<'_, AppState>,
    flavor: PathBuf,
    take_over: bool,
) -> Result<Installed, String> {
    let flavor = find_flavor(&state, &flavor)?;
    if game_running() {
        return Err(addon_install::InstallError::GameRunning.to_string());
    }
    let prereleases = state
        .db
        .lock()
        .map_err(|e| e.to_string())?
        .addon_prereleases()
        .map_err(|e| e.to_string())?;
    let (text, _) = listing(false).await?;
    let release = addon_install::latest_release(&text, prereleases)
        .ok_or("no release of the addon was found")?;
    let bytes = download(&release).await?;
    let package =
        addon_install::verify(&bytes, &release.sha256, &release.tag).map_err(|e| e.to_string())?;
    addon_install::install(
        &flavor,
        &package,
        &state.data_dir,
        game_running(),
        takeover(take_over),
        now_unix(),
    )
    .map_err(|e| e.to_string())
}

/// Removes the addon folder from a flavor; recorded journals stay.
#[tauri::command]
pub fn remove_addon(
    state: State<'_, AppState>,
    flavor: PathBuf,
    take_over: bool,
) -> Result<(), String> {
    let flavor = find_flavor(&state, &flavor)?;
    addon_install::remove(
        &flavor,
        &state.data_dir,
        game_running(),
        takeover(take_over),
        now_unix(),
    )
    .map_err(|e| e.to_string())
}

/// Puts back the newest copy kept from an earlier install.
#[tauri::command]
pub fn restore_addon(state: State<'_, AppState>, flavor: PathBuf) -> Result<Installed, String> {
    let flavor = find_flavor(&state, &flavor)?;
    let rollback = addon_install::rollbacks(&state.data_dir, &flavor)
        .into_iter()
        .next()
        .ok_or("no earlier copy is kept")?;
    let package = addon_install::rollback_package(&rollback.path).map_err(|e| e.to_string())?;
    addon_install::install(
        &flavor,
        &package,
        &state.data_dir,
        game_running(),
        Takeover::Allow,
        now_unix(),
    )
    .map_err(|e| e.to_string())
}
