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

use crate::failure::Failure;
use crate::sync::AppState;
use crate::writing::game_running;

/// The releases listing is fetched at most once a day unless the player asks.
const CACHE_FOR: Duration = Duration::from_secs(24 * 60 * 60);
const TIMEOUT: Duration = Duration::from_secs(30);
/// GitHub's gateway errors usually pass within seconds, so one more try is worth the wait.
const RETRY_AFTER: Duration = Duration::from_secs(2);

/// When the listing was fetched, as an instant and as Unix time, and its text.
type Cached = Option<(Instant, i64, String)>;

static LISTING: Mutex<Cached> = Mutex::new(None);

fn client() -> Result<reqwest::Client, Failure> {
    reqwest::Client::builder()
        .user_agent(concat!("mrglglgl-app/", env!("CARGO_PKG_VERSION")))
        .timeout(TIMEOUT)
        .build()
        .map_err(Failure::from)
}

fn releases_url() -> String {
    format!("https://api.github.com/repos/{RELEASES_REPOSITORY}/releases?per_page=30")
}

/// The releases listing and when it was fetched.
#[derive(Debug, PartialEq)]
struct Listing {
    text: String,
    checked: i64,
    /// GitHub could not be reached, and this is the listing fetched earlier.
    stale: bool,
}

async fn fetch(url: &str) -> Result<String, Failure> {
    let response = client()?
        .get(url)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await?
        .error_for_status()?;
    Ok(response.text().await?)
}

/// The listing from `url`, from `cache` when it is fresh. A server error or a slow answer is
/// tried once more after `retry_after`; when that fails too, the listing fetched earlier, if any,
/// is better than nothing.
async fn listing_from(
    url: &str,
    force: bool,
    cache: &Mutex<Cached>,
    retry_after: Duration,
) -> Result<Listing, Failure> {
    let cached = cache.lock().map_err(|e| e.to_string())?.clone();
    if !force
        && let Some((at, checked, text)) = &cached
        && at.elapsed() < CACHE_FOR
    {
        return Ok(Listing {
            text: text.clone(),
            checked: *checked,
            stale: false,
        });
    }
    let fetched = match fetch(url).await {
        Err(Failure::ServerTrouble { .. }) => {
            tokio::time::sleep(retry_after).await;
            fetch(url).await
        }
        other => other,
    };
    match (fetched, cached) {
        (Ok(text), _) => {
            let checked = now_unix();
            *cache.lock().map_err(|e| e.to_string())? =
                Some((Instant::now(), checked, text.clone()));
            Ok(Listing {
                text,
                checked,
                stale: false,
            })
        }
        (Err(failure), Some((_, checked, text))) => {
            tracing::warn!(?failure, "showing the addon releases fetched earlier");
            Ok(Listing {
                text,
                checked,
                stale: true,
            })
        }
        (Err(failure), None) => Err(failure),
    }
}

async fn listing(force: bool) -> Result<Listing, Failure> {
    listing_from(&releases_url(), force, &LISTING, RETRY_AFTER).await
}

/// The newest release for the player's channel, and the flavors whose installed copy is older.
#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ReleaseCheck {
    pub release: Option<Release>,
    /// Whether test versions are offered.
    pub prereleases: bool,
    /// The newest test version, when test versions are off and there is no release yet.
    pub test_version: Option<String>,
    /// Flavor folders whose installed addon is older than the release.
    pub updates: Vec<PathBuf>,
    /// Unix time of the listing.
    pub checked: i64,
    /// GitHub could not be reached: this is the listing fetched earlier.
    pub stale: bool,
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
) -> Result<ReleaseCheck, Failure> {
    let prereleases = state
        .db
        .lock()
        .map_err(|e| e.to_string())?
        .addon_prereleases()
        .map_err(|e| e.to_string())?;
    let Listing {
        text,
        checked,
        stale,
    } = listing(force).await.inspect_err(|e| {
        tracing::warn!(?e, "the addon releases could not be fetched");
    })?;
    let release = addon_install::latest_release(&text, prereleases);
    let test_version = release
        .is_none()
        .then(|| addon_install::latest_release(&text, true))
        .flatten()
        .map(|r| r.version);
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
    tracing::debug!(
        "addon release: {:?} (test versions {}), {} flavors to update{}",
        release.as_ref().map(|r| &r.version),
        if prereleases { "on" } else { "off" },
        updates.len(),
        if stale { ", from an earlier fetch" } else { "" }
    );
    Ok(ReleaseCheck {
        release,
        prereleases,
        test_version,
        updates,
        checked,
        stale,
    })
}

#[tauri::command]
pub fn set_addon_prereleases(state: State<'_, AppState>, on: bool) -> Result<(), String> {
    tracing::info!(
        "addon test versions {}",
        if on { "offered" } else { "not offered" }
    );
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

async fn download(release: &Release) -> Result<Vec<u8>, Failure> {
    let response = client()?
        .get(&release.url)
        .send()
        .await?
        .error_for_status()?;
    if response
        .content_length()
        .is_some_and(|len| len > MAX_PACKAGE_BYTES)
    {
        return Err(addon_install::InstallError::TooLarge.into());
    }
    let mut bytes = Vec::new();
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        bytes.extend_from_slice(&chunk?);
        if bytes.len() as u64 > MAX_PACKAGE_BYTES {
            return Err(addon_install::InstallError::TooLarge.into());
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
) -> Result<Installed, Failure> {
    let flavor = find_flavor(&state, &flavor)?;
    if game_running() {
        tracing::warn!("addon install refused: the game is running");
        return Err(Failure::GameRunning);
    }
    let prereleases = state
        .db
        .lock()
        .map_err(|e| e.to_string())?
        .addon_prereleases()
        .map_err(|e| e.to_string())?;
    let Listing { text, .. } = listing(false).await?;
    let release = addon_install::latest_release(&text, prereleases)
        .ok_or("no release of the addon was found")?;
    tracing::info!(
        "installing addon {} into {}",
        release.version,
        flavor.path.display()
    );
    let bytes = download(&release)
        .await
        .inspect_err(|e| tracing::warn!(?e, "addon download from {} failed", release.url))?;
    tracing::debug!("addon package downloaded: {} bytes", bytes.len());
    let package = addon_install::verify(&bytes, &release.sha256, &release.tag)
        .inspect_err(|e| tracing::warn!("addon package refused: {e}"))?;
    addon_install::install(
        &flavor,
        &package,
        &state.data_dir,
        game_running(),
        takeover(take_over),
        now_unix(),
    )
    .inspect(|_| tracing::info!("addon {} installed", release.version))
    .map_err(|e| {
        tracing::warn!("addon install failed: {e}");
        Failure::from(e)
    })
}

/// Removes the addon folder from a flavor; recorded journals stay.
#[tauri::command]
pub fn remove_addon(
    state: State<'_, AppState>,
    flavor: PathBuf,
    take_over: bool,
) -> Result<(), Failure> {
    let flavor = find_flavor(&state, &flavor)?;
    tracing::info!("removing the addon from {}", flavor.path.display());
    addon_install::remove(
        &flavor,
        &state.data_dir,
        game_running(),
        takeover(take_over),
        now_unix(),
    )
    .map_err(|e| {
        tracing::warn!("addon removal failed: {e}");
        Failure::from(e)
    })
}

/// Puts back the newest copy kept from an earlier install.
#[tauri::command]
pub fn restore_addon(state: State<'_, AppState>, flavor: PathBuf) -> Result<Installed, Failure> {
    let flavor = find_flavor(&state, &flavor)?;
    let rollback = addon_install::rollbacks(&state.data_dir, &flavor)
        .into_iter()
        .next()
        .ok_or("no earlier copy is kept")?;
    tracing::info!(
        "putting back the addon kept at {} into {}",
        rollback.path.display(),
        flavor.path.display()
    );
    let package = addon_install::rollback_package(&rollback.path)?;
    addon_install::install(
        &flavor,
        &package,
        &state.data_dir,
        game_running(),
        Takeover::Allow,
        now_unix(),
    )
    .map_err(|e| {
        tracing::warn!("addon restore failed: {e}");
        Failure::from(e)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    /// A server answering each request with the next status in `statuses`, the body `[]` on 200.
    async fn serve(statuses: Vec<u16>) -> (String, Arc<AtomicUsize>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/releases", listener.local_addr().unwrap());
        let hits = Arc::new(AtomicUsize::new(0));
        let counter = hits.clone();
        tokio::spawn(async move {
            for status in statuses {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut request = [0u8; 4096];
                let _ = socket.read(&mut request).await;
                counter.fetch_add(1, Ordering::SeqCst);
                let body = if status == 200 { "[]" } else { "" };
                let response = format!(
                    "HTTP/1.1 {status} X\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                socket.write_all(response.as_bytes()).await.unwrap();
            }
        });
        (url, hits)
    }

    const NO_PAUSE: Duration = Duration::ZERO;

    #[tokio::test]
    async fn a_gateway_error_is_tried_once_more() {
        let (url, hits) = serve(vec![504, 200]).await;
        let cache = Mutex::new(None);
        let listing = listing_from(&url, true, &cache, NO_PAUSE).await.unwrap();
        assert_eq!(listing.text, "[]");
        assert!(!listing.stale);
        assert_eq!(hits.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn a_lasting_gateway_error_is_server_trouble() {
        let (url, hits) = serve(vec![502, 503]).await;
        let cache = Mutex::new(None);
        let failure = listing_from(&url, true, &cache, NO_PAUSE)
            .await
            .unwrap_err();
        assert_eq!(failure, Failure::ServerTrouble { status: Some(503) });
        assert_eq!(hits.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn a_client_error_is_not_tried_again() {
        let (url, hits) = serve(vec![404]).await;
        let cache = Mutex::new(None);
        let failure = listing_from(&url, true, &cache, NO_PAUSE)
            .await
            .unwrap_err();
        assert!(matches!(failure, Failure::Other { .. }), "{failure:?}");
        assert_eq!(hits.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn the_listing_fetched_earlier_stands_in_when_github_fails() {
        let (url, _) = serve(vec![504, 504]).await;
        let cache = Mutex::new(Some((Instant::now(), 1_700_000_000, "[old]".to_string())));
        let listing = listing_from(&url, true, &cache, NO_PAUSE).await.unwrap();
        assert_eq!(
            listing,
            Listing {
                text: "[old]".into(),
                checked: 1_700_000_000,
                stale: true,
            }
        );
    }

    #[tokio::test]
    async fn no_server_at_all_is_offline() {
        // A port nothing listens on: bound, then released.
        let port = std::net::TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        let cache = Mutex::new(None);
        let failure = listing_from(&format!("http://127.0.0.1:{port}/"), true, &cache, NO_PAUSE)
            .await
            .unwrap_err();
        assert_eq!(failure, Failure::Offline);
    }
}
