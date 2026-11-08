//! The models the app can write with: a curated list shipped as data, downloads into the app's
//! own cache, and the first-run advice on which model fits the machine.
//!
//! The list is a JSON file, so it can be updated without code: a newer `catalogue.json` in the
//! app data folder replaces the built-in one. Models are downloaded on request and are never part
//! of a build.

use std::path::{Path, PathBuf};

use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt;
use tokio_util::sync::CancellationToken;

/// Where model files are fetched from; a test passes a local server instead.
pub const HUGGING_FACE: &str = "https://huggingface.co";
/// Free disk space kept beyond the model itself.
pub const DISK_MARGIN_BYTES: u64 = 1 << 30;
/// Machines report a little less memory than they are sold with; an "8 GB" machine counts.
const RAM_TOLERANCE: f64 = 0.9;
const GIB: f64 = (1u64 << 30) as f64;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelEntry {
    pub id: String,
    pub name: String,
    /// The Hugging Face repository and the file in it.
    pub repo: String,
    pub file: String,
    pub size_bytes: u64,
    pub licence: String,
    /// Prose languages it handles.
    pub languages: Vec<String>,
    /// It has a reasoning mode, which the app turns off.
    #[serde(default)]
    pub reasoning: bool,
    pub min_ram_gb: u32,
    /// The overall recommendation.
    #[serde(default)]
    pub recommended: bool,
    /// Prose languages it is the recommendation for.
    #[serde(default)]
    pub recommended_for: Vec<String>,
    /// A short note on its prose, per language.
    #[serde(default)]
    pub prose_note: std::collections::BTreeMap<String, String>,
}

impl ModelEntry {
    pub fn url(&self, base: &str) -> String {
        format!(
            "{}/{}/resolve/main/{}",
            base.trim_end_matches('/'),
            self.repo,
            self.file
        )
    }

    pub fn path(&self, models_dir: &Path) -> PathBuf {
        models_dir.join(&self.file)
    }

    fn partial_path(&self, models_dir: &Path) -> PathBuf {
        models_dir.join(format!("{}.part", self.file))
    }

    /// Downloaded completely.
    pub fn installed(&self, models_dir: &Path) -> bool {
        std::fs::metadata(self.path(models_dir)).is_ok_and(|m| m.len() == self.size_bytes)
    }

    /// Bytes of an interrupted download that a new one resumes from.
    pub fn partial_bytes(&self, models_dir: &Path) -> u64 {
        std::fs::metadata(self.partial_path(models_dir)).map_or(0, |m| m.len())
    }

    fn fits_ram(&self, ram_bytes: u64) -> bool {
        ram_bytes as f64 >= f64::from(self.min_ram_gb) * GIB * RAM_TOLERANCE
    }

    fn fits_disk(&self, free_disk_bytes: u64) -> bool {
        free_disk_bytes >= self.size_bytes + DISK_MARGIN_BYTES
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Catalogue {
    pub version: u32,
    pub models: Vec<ModelEntry>,
}

#[derive(Debug, thiserror::Error)]
pub enum CatalogueError {
    #[error("the model list is not valid: {0}")]
    Invalid(#[from] serde_json::Error),
}

impl Catalogue {
    pub fn parse(json: &str) -> Result<Self, CatalogueError> {
        Ok(serde_json::from_str(json)?)
    }

    /// The list shipped with the app.
    pub fn built_in() -> Self {
        Self::parse(include_str!("../data/catalogue.json"))
            .expect("the shipped model list is valid")
    }

    /// The built-in list, or the one at `override_path` when it is valid and newer.
    pub fn load(override_path: Option<&Path>) -> Self {
        let built_in = Self::built_in();
        let Some(path) = override_path else {
            return built_in;
        };
        let Ok(text) = std::fs::read_to_string(path) else {
            return built_in;
        };
        match Self::parse(&text) {
            Ok(c) if c.version > built_in.version => c,
            Ok(_) => built_in,
            Err(e) => {
                tracing::warn!("{}: {e}; using the built-in model list", path.display());
                built_in
            }
        }
    }

    pub fn get(&self, id: &str) -> Option<&ModelEntry> {
        self.models.iter().find(|m| m.id == id)
    }

    /// The recommended model for prose in `language`, or the overall recommendation.
    pub fn recommended(&self, language: &str) -> Option<&ModelEntry> {
        self.models
            .iter()
            .find(|m| m.recommended_for.iter().any(|l| l == language))
            .or_else(|| self.models.iter().find(|m| m.recommended))
            .or(self.models.first())
    }

    /// Which model to offer on first run, given the machine.
    pub fn advise(&self, ram_bytes: u64, free_disk_bytes: u64, language: &str) -> Advice {
        let fits = |m: &&ModelEntry| m.fits_ram(ram_bytes) && m.fits_disk(free_disk_bytes);
        if let Some(m) = self.recommended(language).filter(fits) {
            return Advice::Recommend { id: m.id.clone() };
        }
        // Otherwise the smallest model that fits.
        let mut fitting: Vec<&ModelEntry> = self.models.iter().filter(fits).collect();
        fitting.sort_by_key(|m| m.size_bytes);
        if let Some(m) = fitting.first() {
            return Advice::Recommend { id: m.id.clone() };
        }
        let needs_ram = self.models.iter().map(|m| m.min_ram_gb).min().unwrap_or(8);
        if !self.models.iter().any(|m| m.fits_ram(ram_bytes)) {
            Advice::BelowFloor {
                reason: Shortfall::Memory {
                    needed_gb: needs_ram,
                    has_gb: (ram_bytes as f64 / GIB * 10.0).round() / 10.0,
                },
            }
        } else {
            let smallest = self.models.iter().map(|m| m.size_bytes).min().unwrap_or(0);
            Advice::BelowFloor {
                reason: Shortfall::Disk {
                    needed_gb: ((smallest + DISK_MARGIN_BYTES) as f64 / GIB * 10.0).ceil() / 10.0,
                    free_gb: (free_disk_bytes as f64 / GIB * 10.0).floor() / 10.0,
                },
            }
        }
    }
}

/// What the machine can run.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum Advice {
    Recommend {
        id: String,
    },
    /// No model fits: the app says so and offers an endpoint instead.
    BelowFloor {
        reason: Shortfall,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum Shortfall {
    #[serde(rename_all = "camelCase")]
    Memory { needed_gb: u32, has_gb: f64 },
    #[serde(rename_all = "camelCase")]
    Disk { needed_gb: f64, free_gb: f64 },
}

/// How far a download has come.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Progress {
    pub downloaded: u64,
    pub total: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DownloadError {
    #[error("the download was cancelled")]
    Cancelled,
    #[error(
        "not enough free disk space: {} GB needed, {} GB free",
        gigabytes(*needed),
        gigabytes(*free)
    )]
    NoSpace { needed: u64, free: u64 },
    #[error("could not download the model: {0}")]
    Network(String),
    #[error("the server answered {0}")]
    Status(u16),
    #[error("the downloaded file is {got} bytes, expected {expected}")]
    WrongSize { got: u64, expected: u64 },
    #[error("could not write the model file: {0}")]
    Io(String),
}

/// Bytes as decimal gigabytes, the unit the models list shows sizes in.
fn gigabytes(bytes: u64) -> String {
    format!("{:.2}", bytes as f64 / 1e9)
}

fn io(e: std::io::Error) -> DownloadError {
    DownloadError::Io(e.to_string())
}

/// Downloads `entry` into `models_dir`, resuming an interrupted download, reporting progress, and
/// stopping on `cancel` (the partial file is kept for next time). Returns the model file's path.
/// A model already there is not downloaded again.
pub async fn download(
    entry: &ModelEntry,
    models_dir: &Path,
    base_url: &str,
    free_disk_bytes: Option<u64>,
    cancel: &CancellationToken,
    mut on_progress: impl FnMut(Progress) + Send,
) -> Result<PathBuf, DownloadError> {
    let target = entry.path(models_dir);
    if entry.installed(models_dir) {
        on_progress(Progress {
            downloaded: entry.size_bytes,
            total: entry.size_bytes,
        });
        return Ok(target);
    }
    tokio::fs::create_dir_all(models_dir).await.map_err(io)?;
    let partial = entry.partial_path(models_dir);
    let mut have = entry.partial_bytes(models_dir);
    if have > entry.size_bytes {
        tokio::fs::remove_file(&partial).await.map_err(io)?;
        have = 0;
    }
    let remaining = entry.size_bytes - have;
    if let Some(free) = free_disk_bytes
        && free < remaining + DISK_MARGIN_BYTES
    {
        return Err(DownloadError::NoSpace {
            needed: remaining + DISK_MARGIN_BYTES,
            free,
        });
    }

    let client = reqwest::Client::new();
    let mut request = client.get(entry.url(base_url));
    if have > 0 {
        request = request.header(reqwest::header::RANGE, format!("bytes={have}-"));
    }
    let response = tokio::select! {
        r = request.send() => r.map_err(|e| DownloadError::Network(e.to_string()))?,
        () = cancel.cancelled() => return Err(DownloadError::Cancelled),
    };
    let status = response.status();
    let resumed = status == reqwest::StatusCode::PARTIAL_CONTENT;
    if !status.is_success() {
        return Err(DownloadError::Status(status.as_u16()));
    }
    let mut file = if resumed {
        tokio::fs::OpenOptions::new()
            .append(true)
            .open(&partial)
            .await
            .map_err(io)?
    } else {
        // The server sent the whole file: start over.
        have = 0;
        tokio::fs::File::create(&partial).await.map_err(io)?
    };
    on_progress(Progress {
        downloaded: have,
        total: entry.size_bytes,
    });

    let mut stream = response.bytes_stream();
    loop {
        let chunk = tokio::select! {
            c = stream.next() => c,
            () = cancel.cancelled() => {
                file.flush().await.map_err(io)?;
                return Err(DownloadError::Cancelled);
            }
        };
        let Some(chunk) = chunk else { break };
        let chunk = chunk.map_err(|e| DownloadError::Network(e.to_string()))?;
        file.write_all(&chunk).await.map_err(io)?;
        have += chunk.len() as u64;
        on_progress(Progress {
            downloaded: have,
            total: entry.size_bytes,
        });
    }
    file.flush().await.map_err(io)?;
    drop(file);
    if have != entry.size_bytes {
        if have > entry.size_bytes {
            let _ = tokio::fs::remove_file(&partial).await;
        }
        return Err(DownloadError::WrongSize {
            got: have,
            expected: entry.size_bytes,
        });
    }
    tokio::fs::rename(&partial, &target).await.map_err(io)?;
    Ok(target)
}

/// Removes a downloaded model and any partial download of it.
pub fn delete(entry: &ModelEntry, models_dir: &Path) -> std::io::Result<()> {
    for path in [entry.path(models_dir), entry.partial_path(models_dir)] {
        match std::fs::remove_file(&path) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e),
        }
    }
    Ok(())
}
