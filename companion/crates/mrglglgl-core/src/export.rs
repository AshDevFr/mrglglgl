//! Sharing the journal: one entry as plain text for the clipboard, and a whole character as
//! Markdown and as a single self-contained HTML page.
//!
//! Days without a usable entry (not written yet, or set aside by the fact check) appear with
//! their plain notes, so an export is the complete journal.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use chrono::{Datelike, NaiveDate};

use crate::beats::{self, Thresholds};
use crate::milestones::{self, Award};
use crate::prompt;
use crate::schema::Journal;
use crate::stories::{Story, StoryKind, StoryStatus};
use crate::writer::CheckStatus;

/// The one-line footer "Copy entry" adds unless turned off in the settings.
pub const FOOTER: &str = "via Mrglglgl (github.com/AshDevFr/mrglglgl)";

/// An open-licence font (SIL OFL 1.1, see `assets/literata-OFL.txt`), embedded in exported
/// pages so they look the same anywhere.
pub(crate) const LITERATA: &[u8] = include_bytes!("../assets/literata-latin-wght-normal.woff2");

const WEEKDAYS_EN: [&str; 7] = [
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
    "Sunday",
];
const MONTHS_EN: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];
const WEEKDAYS_FR: [&str; 7] = [
    "lundi", "mardi", "mercredi", "jeudi", "vendredi", "samedi", "dimanche",
];
const MONTHS_FR: [&str; 12] = [
    "janvier",
    "février",
    "mars",
    "avril",
    "mai",
    "juin",
    "juillet",
    "août",
    "septembre",
    "octobre",
    "novembre",
    "décembre",
];

fn french(language: &str) -> bool {
    language.starts_with("fr")
}

/// "Sunday, November 8, 2026" or "dimanche 8 novembre 2026".
pub fn long_date(day: &str, language: &str) -> String {
    let Ok(d) = NaiveDate::parse_from_str(day, "%Y-%m-%d") else {
        return day.to_string();
    };
    let weekday = d.weekday().num_days_from_monday() as usize;
    let month = d.month0() as usize;
    if french(language) {
        format!(
            "{} {} {} {}",
            WEEKDAYS_FR[weekday],
            d.day(),
            MONTHS_FR[month],
            d.year()
        )
    } else {
        format!(
            "{}, {} {}, {}",
            WEEKDAYS_EN[weekday],
            MONTHS_EN[month],
            d.day(),
            d.year()
        )
    }
}

fn week_heading(monday: &str, language: &str) -> String {
    if french(language) {
        format!("Semaine du {}", long_date(monday, language))
    } else {
        format!("Week of {}", long_date(monday, language))
    }
}

fn notes_label(language: &str) -> &'static str {
    if french(language) {
        "Notes du jour"
    } else {
        "Notes from the day"
    }
}

fn stamps_label(language: &str) -> &'static str {
    if french(language) {
        "Tampons"
    } else {
        "Stamps"
    }
}

fn written_with(language: &str) -> &'static str {
    if french(language) {
        "Écrit avec Mrglglgl"
    } else {
        "Written with Mrglglgl"
    }
}

/// One entry as plain text for the clipboard: the day as a header, the entry, and the footer.
pub fn copy_entry(day: &str, language: &str, entry: &str, footer: bool) -> String {
    let mut text = format!("{}\n\n{}", long_date(day, language), entry.trim());
    if footer {
        text.push_str("\n\n");
        text.push_str(FOOTER);
    }
    text
}

/// What an export is made from.
pub struct ExportInput<'a> {
    /// The character's name as shown, e.g. "Wren Hollowbrook".
    pub title: &'a str,
    pub journal: &'a Journal,
    /// Every story version; the newest of each day and week is used.
    pub stories: &'a [Story],
    pub awards: &'a [Award],
    /// The language of the headings.
    pub language: &'a str,
}

enum Body {
    Entry(String),
    Notes(Vec<String>),
}

struct DayPart {
    day: String,
    body: Body,
    stamps: Vec<String>,
}

struct WeekPart {
    monday: String,
    chapter: Option<String>,
    days: Vec<DayPart>,
}

fn usable(story: &Story) -> bool {
    story.check != CheckStatus::Failed
}

fn parts(input: &ExportInput) -> Vec<WeekPart> {
    let mut latest: BTreeMap<(StoryKind, &str), &Story> = BTreeMap::new();
    // Only accepted entries are part of the journal; drafts wait for the player.
    for s in input
        .stories
        .iter()
        .filter(|s| s.status == StoryStatus::Accepted)
    {
        let slot = (s.kind, s.start_day.as_str());
        if latest.get(&slot).is_none_or(|old| old.id < s.id) {
            latest.insert(slot, s);
        }
    }
    let thresholds = Thresholds::default();
    let mut weeks: BTreeMap<String, WeekPart> = BTreeMap::new();
    for day in input.journal.days.keys() {
        let Some(monday) = prompt::week_start(day) else {
            continue;
        };
        let body = match latest.get(&(StoryKind::Day, day.as_str())) {
            Some(s) if usable(s) => Body::Entry(s.text.trim().to_string()),
            _ => Body::Notes(beats::extract(input.journal, day, input.awards, &thresholds).texts()),
        };
        let stamps = input
            .awards
            .iter()
            .filter(|a| a.day == *day)
            .map(|a| milestones::name(&a.id).to_string())
            .collect();
        let week = weeks.entry(monday.clone()).or_insert_with(|| WeekPart {
            chapter: latest
                .get(&(StoryKind::Week, monday.as_str()))
                .filter(|s| usable(s))
                .map(|s| s.text.trim().to_string()),
            monday,
            days: Vec::new(),
        });
        week.days.push(DayPart {
            day: day.clone(),
            body,
            stamps,
        });
    }
    weeks.into_values().collect()
}

/// The whole journal as Markdown.
pub fn markdown(input: &ExportInput) -> String {
    let lang = input.language;
    let mut out = format!("# {}\n", input.title);
    for week in parts(input) {
        out.push_str(&format!("\n## {}\n", week_heading(&week.monday, lang)));
        if let Some(chapter) = &week.chapter {
            out.push_str(&format!("\n{chapter}\n"));
        }
        for day in &week.days {
            out.push_str(&format!("\n### {}\n\n", long_date(&day.day, lang)));
            match &day.body {
                Body::Entry(text) => out.push_str(&format!("{text}\n")),
                Body::Notes(notes) => {
                    out.push_str(&format!("*{}*\n\n", notes_label(lang)));
                    for n in notes {
                        out.push_str(&format!("- {n}\n"));
                    }
                }
            }
            if !day.stamps.is_empty() {
                out.push_str(&format!(
                    "\n{}: {}\n",
                    stamps_label(lang),
                    day.stamps.join(", ")
                ));
            }
        }
    }
    out.push_str(&format!("\n---\n\n{}\n", written_with(lang)));
    out
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

pub(crate) fn base64(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(TABLE[((n >> (18 - 6 * i)) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

/// A page the window laid out, as one HTML file with its font inside, for any browser and for
/// printing. The window builds the body from the same components it shows, so an export looks
/// like the journal in the app; `css` is that look.
pub fn page_document(title: &str, lang: &str, css: &str, body: &str) -> String {
    format!(
        "<!doctype html>\n<html lang=\"{lang}\">\n<head>\n<meta charset=\"utf-8\">\n\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
         <title>{title}</title>\n<style>\n@font-face {{ font-family: \"Literata Variable\"; \
         font-weight: 200 900; font-display: swap; \
         src: url(data:font/woff2;base64,{font}) format(\"woff2\"); }}\n{css}\n</style>\n\
         </head>\n<body class=\"export-body\">\n{body}\n</body>\n</html>\n",
        lang = escape(lang),
        title = escape(title),
        font = base64(LITERATA),
    )
}

#[derive(Debug, thiserror::Error)]
pub enum ExportError {
    #[error(
        "exports cannot be saved inside the World of Warcraft folder ({0}); pick another folder"
    )]
    InsideGameFolder(PathBuf),
    #[error("could not write {path}: {source}")]
    Write {
        path: PathBuf,
        source: std::io::Error,
    },
}

fn file_stem(title: &str) -> String {
    let stem: String = title
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect();
    let stem = stem.trim_matches('-').to_string();
    if stem.is_empty() {
        "journal".to_string()
    } else {
        format!("{stem}-journal")
    }
}

fn inside(path: &Path, root: &Path) -> bool {
    let canonical = |p: &Path| p.canonicalize().unwrap_or_else(|_| p.to_path_buf());
    canonical(path).starts_with(canonical(root))
}

/// Writes one export, `<title>-journal.<extension>`, into `folder`, which must not be inside any
/// of the game folders: the app never writes there. Returns the file's path.
pub fn write_file(
    folder: &Path,
    game_folders: &[PathBuf],
    title: &str,
    extension: &str,
    content: &str,
) -> Result<PathBuf, ExportError> {
    if let Some(root) = game_folders.iter().find(|root| inside(folder, root)) {
        return Err(ExportError::InsideGameFolder(root.clone()));
    }
    let path = folder.join(format!("{}.{extension}", file_stem(title)));
    std::fs::write(&path, content).map_err(|source| ExportError::Write {
        path: path.clone(),
        source,
    })?;
    Ok(path)
}
