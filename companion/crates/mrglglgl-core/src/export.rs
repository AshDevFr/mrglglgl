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
use crate::stories::{Story, StoryKind};
use crate::writer::CheckStatus;

/// The one-line footer "Copy entry" adds unless turned off in the settings.
pub const FOOTER: &str = "via Mrglglgl (github.com/AshDevFr/mrglglgl)";

/// An open-licence font (SIL OFL 1.1, see `assets/literata-OFL.txt`), embedded in exported
/// pages so they look the same anywhere.
const LITERATA: &[u8] = include_bytes!("../assets/literata-latin-wght-normal.woff2");

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
    for s in input.stories {
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

fn paragraphs(text: &str) -> String {
    text.split("\n\n")
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .map(|p| format!("<p>{}</p>", escape(p).replace('\n', "<br>")))
        .collect::<Vec<_>>()
        .join("\n")
}

fn base64(bytes: &[u8]) -> String {
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

const STYLE: &str = "
:root { --desk: #cdd3cf; --page: #e3e6de; --edge: #c9d0c8; --ink: #1c2a34;
  --faded: #4f5b61; --stamp: #2a6351; }
@media (prefers-color-scheme: dark) {
  :root { --desk: #121819; --page: #1e2628; --edge: #283335; --ink: #dce4df;
    --faded: #93a39e; --stamp: #78bfa4; }
}
* { box-sizing: border-box; }
body { margin: 0; background: var(--desk); color: var(--ink);
  font-family: Literata, Georgia, serif; line-height: 1.75; }
main { max-width: 44rem; margin: 2rem auto; padding: 0 16px; }
h1 { font-size: 2.4rem; line-height: 1.1; margin: 1rem 0 2rem; }
section.week { margin-bottom: 3rem; }
h2 { font-size: 1.1rem; font-weight: 600; color: var(--faded); margin: 0 0 1rem; }
.chapter { font-style: italic; margin-bottom: 1.5rem; }
article { background: var(--page); border-radius: 2px; padding: 2.5rem 3rem;
  margin-bottom: 1.5rem; box-shadow: 0 1px 0 var(--edge); }
h3 { font-size: 1.6rem; line-height: 1.2; margin: 0 0 1.25rem; }
p { margin: 0 0 1em; max-width: 34em; font-size: 1.1rem; }
.notes { color: var(--faded); font-size: 0.95rem; }
.stamps { color: var(--stamp); font-size: 0.9rem; margin: 1rem 0 0; }
footer { color: var(--faded); font-size: 0.85rem; text-align: center; margin: 3rem 0; }
@media (max-width: 40rem) { article { padding: 1.75rem 1.25rem; } }
";

/// The whole journal as one HTML file with its font and styles inside, for any browser.
pub fn html(input: &ExportInput) -> String {
    let lang = input.language;
    let mut body = String::new();
    for week in parts(input) {
        body.push_str(&format!(
            "<section class=\"week\">\n<h2>{}</h2>\n",
            escape(&week_heading(&week.monday, lang))
        ));
        if let Some(chapter) = &week.chapter {
            body.push_str(&format!(
                "<div class=\"chapter\">{}</div>\n",
                paragraphs(chapter)
            ));
        }
        for day in &week.days {
            body.push_str(&format!(
                "<article>\n<h3>{}</h3>\n",
                escape(&long_date(&day.day, lang))
            ));
            match &day.body {
                Body::Entry(text) => body.push_str(&paragraphs(text)),
                Body::Notes(notes) => {
                    body.push_str(&format!(
                        "<div class=\"notes\"><p>{}</p><ul>{}</ul></div>",
                        escape(notes_label(lang)),
                        notes
                            .iter()
                            .map(|n| format!("<li>{}</li>", escape(n)))
                            .collect::<String>()
                    ));
                }
            }
            if !day.stamps.is_empty() {
                body.push_str(&format!(
                    "\n<p class=\"stamps\">{}: {}</p>",
                    escape(stamps_label(lang)),
                    escape(&day.stamps.join(", "))
                ));
            }
            body.push_str("\n</article>\n");
        }
        body.push_str("</section>\n");
    }
    format!(
        "<!doctype html>\n<html lang=\"{lang}\">\n<head>\n<meta charset=\"utf-8\">\n\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
         <title>{title}</title>\n<style>\n@font-face {{ font-family: Literata; \
         font-weight: 200 900; font-display: swap; \
         src: url(data:font/woff2;base64,{font}) format(\"woff2\"); }}{STYLE}</style>\n\
         </head>\n<body>\n<main>\n<h1>{title}</h1>\n{body}<footer>{footer}</footer>\n\
         </main>\n</body>\n</html>\n",
        lang = escape(lang),
        title = escape(input.title),
        font = base64(LITERATA),
        footer = escape(written_with(lang)),
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

/// The two files written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportFiles {
    pub markdown: PathBuf,
    pub html: PathBuf,
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

/// Writes the Markdown and HTML exports into `folder`, which must not be inside any of the game
/// folders: the app never writes there.
pub fn write(
    folder: &Path,
    game_folders: &[PathBuf],
    input: &ExportInput,
) -> Result<ExportFiles, ExportError> {
    if let Some(root) = game_folders.iter().find(|root| inside(folder, root)) {
        return Err(ExportError::InsideGameFolder(root.clone()));
    }
    let stem = file_stem(input.title);
    let files = ExportFiles {
        markdown: folder.join(format!("{stem}.md")),
        html: folder.join(format!("{stem}.html")),
    };
    for (path, content) in [
        (&files.markdown, markdown(input)),
        (&files.html, html(input)),
    ] {
        std::fs::write(path, content).map_err(|source| ExportError::Write {
            path: path.clone(),
            source,
        })?;
    }
    Ok(files)
}
