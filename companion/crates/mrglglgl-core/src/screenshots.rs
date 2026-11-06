//! Pairing the game's screenshot files with the moments the addon took them.
//!
//! The addon records a `screenshot` event when it asks the game for one; the game writes
//! `Screenshots/WoWScrnShot_MMDDYY_HHMMSS.jpg` (local time) about a second later. A file is shown
//! with the event closest to it within a few seconds. Files are only ever listed and read.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use chrono::{Local, NaiveDateTime, TimeZone};
use serde::{Deserialize, Serialize};

use crate::schema::{EventKind, Journal};
use crate::watch::modified_unix;

/// How far apart, in seconds, an event and a file may be and still belong together.
pub const MATCH_WINDOW_SECONDS: i64 = 10;

/// The folder the game writes screenshots to, inside a flavor folder.
pub const SCREENSHOTS_FOLDER: &str = "Screenshots";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase")]
pub struct ScreenshotFile {
    pub path: PathBuf,
    /// When it was taken, as Unix time.
    pub taken: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase")]
pub struct Screenshot {
    pub path: PathBuf,
    pub taken: i64,
    /// What the addon was capturing: `"levelUp"`, `"death"`, ...
    pub moment: Option<String>,
}

/// The local time a screenshot file name encodes, e.g. `WoWScrnShot_110526_202955.jpg`.
pub fn taken_from_name(name: &str) -> Option<i64> {
    let stem = name.strip_prefix("WoWScrnShot_")?;
    let stamp = stem.split('.').next()?;
    let parsed = NaiveDateTime::parse_from_str(stamp, "%m%d%y_%H%M%S").ok()?;
    Local
        .from_local_datetime(&parsed)
        .earliest()
        .map(|t| t.timestamp())
}

/// True for an image the game writes as a screenshot.
pub fn is_screenshot_file(path: &Path) -> bool {
    path.extension().and_then(|e| e.to_str()).is_some_and(|e| {
        matches!(
            e.to_ascii_lowercase().as_str(),
            "jpg" | "jpeg" | "png" | "tga"
        )
    })
}

/// The screenshot files in a folder, oldest first. Times come from the file names, or from the
/// modification time for a file named some other way.
pub fn list(folder: &Path) -> Vec<ScreenshotFile> {
    let mut files: Vec<ScreenshotFile> = std::fs::read_dir(folder)
        .map(|entries| {
            entries
                .flatten()
                .map(|e| e.path())
                .filter(|p| p.is_file() && is_screenshot_file(p))
                .filter_map(|path| {
                    let name = path.file_name()?.to_str()?;
                    let taken = taken_from_name(name).or_else(|| modified_unix(&path))?;
                    Some(ScreenshotFile { path, taken })
                })
                .collect()
        })
        .unwrap_or_default();
    files.sort_by_key(|f| f.taken);
    files
}

/// The screenshot folder of the flavor a journal lives in: the journal is at
/// `<flavor>/WTF/Account/<account>/<realm>/<character>/SavedVariables/Mrglglgl.lua`.
pub fn folder_for_journal(journal_path: &Path) -> Option<PathBuf> {
    journal_path
        .ancestors()
        .nth(7)
        .map(|flavor| flavor.join(SCREENSHOTS_FOLDER))
}

/// Screenshots matched to the journal's screenshot events, by day. Each file goes to the closest
/// event within the window and is used once; files no event explains are left out.
pub fn match_to_journal(
    journal: &Journal,
    files: &[ScreenshotFile],
) -> BTreeMap<String, Vec<Screenshot>> {
    let mut used = vec![false; files.len()];
    let mut by_day: BTreeMap<String, Vec<Screenshot>> = BTreeMap::new();
    for (date, day) in &journal.days {
        for event in day.events.iter().flatten() {
            let EventKind::Screenshot { moment } = &event.kind else {
                continue;
            };
            let best = files
                .iter()
                .enumerate()
                .filter(|(i, f)| !used[*i] && (f.taken - event.time).abs() <= MATCH_WINDOW_SECONDS)
                .min_by_key(|(_, f)| (f.taken - event.time).abs());
            if let Some((i, file)) = best {
                used[i] = true;
                by_day.entry(date.clone()).or_default().push(Screenshot {
                    path: file.path.clone(),
                    taken: file.taken,
                    moment: moment.clone(),
                });
            }
        }
    }
    by_day
}

/// The widest a screenshot is embedded in an export: a page's width at print resolution.
pub const EMBED_WIDTH: u32 = 1200;
/// JPEG quality for embedded screenshots: small files, no visible loss at page size.
const EMBED_QUALITY: u8 = 82;

/// A screenshot as JPEG bytes no wider than [`EMBED_WIDTH`], never enlarged.
pub fn scaled_jpeg(path: &Path) -> Result<Vec<u8>, image::ImageError> {
    let picture = image::ImageReader::open(path)?
        .with_guessed_format()?
        .decode()?;
    let picture = if picture.width() > EMBED_WIDTH {
        picture.resize(EMBED_WIDTH, u32::MAX, image::imageops::FilterType::Triangle)
    } else {
        picture
    };
    let mut out = Vec::new();
    let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, EMBED_QUALITY);
    picture.to_rgb8().write_with_encoder(encoder)?;
    Ok(out)
}

/// A screenshot as a `data:` URL, so an exported page carries its pictures inside.
pub fn embed(path: &Path) -> Result<String, image::ImageError> {
    Ok(format!(
        "data:image/jpeg;base64,{}",
        crate::export::base64(&scaled_jpeg(path)?)
    ))
}
