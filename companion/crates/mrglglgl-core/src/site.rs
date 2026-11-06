//! The journal website: a finished static site, rendered through a theme the player can override.
//!
//! The window prepares the content as [`SiteData`] (posts, days, entries, stamps, captions, with
//! stand-in names already applied); templates only ever see that. Templates are Jinja-style
//! (MiniJinja). A theme folder's file wins over the default theme's file of the same name, so a
//! player can override one template, or add `custom.css`, and keep everything else.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use minijinja::{Environment, Value, context};
use serde::{Deserialize, Serialize};

/// Posts on one index page.
pub const POSTS_PER_PAGE: usize = 10;
/// Posts in a feed.
const FEED_POSTS: usize = 20;

/// The default theme, shipped inside the app.
pub const DEFAULT_THEME: &[(&str, &str)] = &[
    ("base.html", include_str!("../site-theme/base.html")),
    ("index.html", include_str!("../site-theme/index.html")),
    (
        "character.html",
        include_str!("../site-theme/character.html"),
    ),
    ("post.html", include_str!("../site-theme/post.html")),
    ("archive.html", include_str!("../site-theme/archive.html")),
    (
        "milestones.html",
        include_str!("../site-theme/milestones.html"),
    ),
    ("feed.xml", include_str!("../site-theme/feed.xml")),
    (
        "assets/site.css",
        include_str!("../site-theme/assets/site.css"),
    ),
    (
        "assets/site.js",
        include_str!("../site-theme/assets/site.js"),
    ),
];

/// Everything a site shows, prepared by the window. Field names are what templates use.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct SiteData {
    pub title: String,
    /// The language of the pages, e.g. `"en"`.
    pub language: String,
    /// Where the site will live, e.g. `"https://wren.pages.dev/"`; feeds need full links.
    pub base_url: Option<String>,
    /// The paper the site starts on: `"tide"` or `"parchment"`.
    pub paper: String,
    /// The theme's words in the site's language ("posts", "archive", ...); templates fall back to
    /// English for any that are missing.
    pub labels: BTreeMap<String, String>,
    pub characters: Vec<SiteCharacter>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct SiteCharacter {
    /// The character's folder in the site, e.g. `"wren"`.
    pub slug: String,
    pub name: String,
    /// One line about the character, e.g. "Tauren Hunter, level 10".
    pub summary: String,
    pub posts: Vec<SitePost>,
    pub milestones: Vec<SiteStamp>,
}

/// One week, as a post.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct SitePost {
    /// The post's folder, e.g. `"2026-w45"`.
    pub slug: String,
    pub title: String,
    /// The week's Monday, `YYYY-MM-DD`.
    pub date: String,
    /// `YYYY-MM`, for the archive.
    pub month: String,
    pub month_label: String,
    /// A line or two for index pages.
    pub excerpt: String,
    /// The week's chapter as HTML, when one was written.
    pub chapter_html: Option<String>,
    pub days: Vec<SiteDay>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct SiteDay {
    /// `YYYY-MM-DD`.
    pub date: String,
    pub title: String,
    /// The entry, or what happened, as HTML.
    pub html: String,
    /// The narrator wrote this day; otherwise `html` holds its facts.
    pub written: bool,
    pub stamps: Vec<SiteStamp>,
    pub photos: Vec<SitePhoto>,
}

/// A milestone, with its ink stamp drawn as SVG.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct SiteStamp {
    pub name: String,
    pub description: String,
    pub day: String,
    pub svg: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct SitePhoto {
    /// The screenshot on this computer; never put in the site itself.
    #[serde(skip_serializing)]
    pub source: String,
    /// The picture's file name in `images/<character>/`.
    pub file: String,
    pub caption: String,
}

/// One file of the site.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SiteFile {
    /// Relative to the site folder, with `/`.
    pub path: String,
    pub content: SiteContent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SiteContent {
    Text(String),
    Bytes(Vec<u8>),
    /// A screenshot, scaled to page width when the site is written.
    Image(PathBuf),
}

#[derive(Debug, thiserror::Error)]
pub enum SiteError {
    /// A template could not be rendered; the message names the file and line.
    #[error("{0}")]
    Template(String),
    #[error("could not read the theme: {0}")]
    Theme(String),
}

fn default_file(name: &str) -> Option<&'static str> {
    DEFAULT_THEME
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, text)| *text)
}

/// A theme file: the theme folder's, else the default theme's. Names that climb out of the
/// folder are refused.
fn theme_file(theme: Option<&Path>, name: &str) -> Result<Option<String>, SiteError> {
    let safe = !name.is_empty()
        && !name.contains('\\')
        && !name.starts_with('/')
        && name.split('/').all(|part| !part.is_empty() && part != "..");
    if !safe {
        return Ok(None);
    }
    if let Some(dir) = theme {
        let path = dir.join(name);
        if path.is_file() {
            return std::fs::read_to_string(&path)
                .map(Some)
                .map_err(|e| SiteError::Theme(format!("{}: {e}", path.display())));
        }
    }
    Ok(default_file(name).map(str::to_string))
}

/// "Mon, 02 Nov 2026 00:00:00 +0000", for feeds.
fn rfc822(date: String) -> String {
    chrono::NaiveDate::parse_from_str(&date, "%Y-%m-%d")
        .map(|d| d.format("%a, %d %b %Y 00:00:00 +0000").to_string())
        .unwrap_or(date)
}

fn template_error(e: &minijinja::Error) -> SiteError {
    let place = match (e.name(), e.line()) {
        (Some(name), Some(line)) => format!("{name}, line {line}: "),
        (Some(name), None) => format!("{name}: "),
        _ => String::new(),
    };
    let detail = e.detail().map(|d| format!(": {d}")).unwrap_or_default();
    SiteError::Template(format!("{place}{}{detail}", e.kind()))
}

/// A path or address the core built itself, given to templates as it is: HTML escaping would
/// turn its slashes into entities.
fn link(text: &str) -> Value {
    Value::from_safe_string(text.to_string())
}

/// The site's address with the characters that could break out of an attribute encoded.
fn clean_url(url: &str) -> String {
    let url = url.trim();
    let mut out = String::with_capacity(url.len() + 1);
    for c in url.chars() {
        match c {
            '"' => out.push_str("%22"),
            '\'' => out.push_str("%27"),
            '<' => out.push_str("%3C"),
            '>' => out.push_str("%3E"),
            '&' => out.push_str("%26"),
            c if c.is_whitespace() => out.push_str("%20"),
            c => out.push(c),
        }
    }
    if !out.ends_with('/') {
        out.push('/');
    }
    out
}

/// Every file of the site, rendered through `theme` over the default theme. Nothing is written.
pub fn render(data: &SiteData, theme: Option<&Path>) -> Result<Vec<SiteFile>, SiteError> {
    let mut env = Environment::new();
    let dir = theme.map(Path::to_path_buf);
    env.set_loader(move |name| {
        theme_file(dir.as_deref(), name).map_err(|e| {
            minijinja::Error::new(minijinja::ErrorKind::InvalidOperation, e.to_string())
        })
    });
    env.add_filter("rfc822", rfc822);

    let custom_css = match theme.map(|t| t.join("custom.css")) {
        Some(path) if path.is_file() => Some(
            std::fs::read_to_string(&path)
                .map_err(|e| SiteError::Theme(format!("{}: {e}", path.display())))?,
        ),
        _ => None,
    };
    let has_custom_css = custom_css.is_some();
    let base_url = data
        .base_url
        .as_deref()
        .filter(|u| !u.trim().is_empty())
        .map(clean_url);

    let mut files = Vec::new();
    let mut page = |template: &str, path: String, root: &str, extra: Value| {
        let text = env
            .get_template(template)
            .and_then(|t| {
                t.render(context! {
                    site => data,
                    root => link(root),
                    has_custom_css => has_custom_css,
                    labels => &data.labels,
                    ..extra
                })
            })
            .map_err(|e| template_error(&e))?;
        files.push(SiteFile {
            path,
            content: SiteContent::Text(text),
        });
        Ok::<(), SiteError>(())
    };

    page("index.html", "index.html".into(), "", context! {})?;

    let mut images = Vec::new();
    for character in &data.characters {
        let slug = &character.slug;
        let mut posts: Vec<&SitePost> = character.posts.iter().collect();
        posts.sort_by(|a, b| b.date.cmp(&a.date));

        let pages: Vec<&[&SitePost]> = posts.chunks(POSTS_PER_PAGE).collect();
        let total = pages.len().max(1);
        for number in 1..=total {
            let (path, root) = if number == 1 {
                (format!("{slug}/index.html"), "../")
            } else {
                (format!("{slug}/page/{number}/index.html"), "../../../")
            };
            let page_link = |n: usize| {
                if n == 1 {
                    format!("{root}{slug}/")
                } else {
                    format!("{root}{slug}/page/{n}/")
                }
            };
            page(
                "character.html",
                path,
                root,
                context! {
                    character => character,
                    posts => pages.get(number - 1).copied().unwrap_or(&[]),
                    page => context! {
                        number => number,
                        total => total,
                        newer_url => (number > 1).then(|| link(&page_link(number - 1))),
                        older_url => (number < total).then(|| link(&page_link(number + 1))),
                    },
                },
            )?;
        }

        for (i, post) in posts.iter().enumerate() {
            page(
                "post.html",
                format!("{slug}/{}/index.html", post.slug),
                "../../",
                context! {
                    character => character,
                    post => post,
                    newer_post => i.checked_sub(1).and_then(|j| posts.get(j)),
                    older_post => posts.get(i + 1),
                },
            )?;
            for day in &post.days {
                for photo in &day.photos {
                    images.push(SiteFile {
                        path: format!("images/{slug}/{}", photo.file),
                        content: SiteContent::Image(PathBuf::from(&photo.source)),
                    });
                }
            }
        }

        let mut months: Vec<(String, String, Vec<&SitePost>)> = Vec::new();
        for post in &posts {
            match months.last_mut() {
                Some((month, _, list)) if *month == post.month => list.push(post),
                _ => months.push((post.month.clone(), post.month_label.clone(), vec![post])),
            }
        }
        let months: Vec<Value> = months
            .into_iter()
            .map(
                |(month, label, posts)| context! { month => month, label => label, posts => posts },
            )
            .collect();
        page(
            "archive.html",
            format!("{slug}/archive/index.html"),
            "../../",
            context! { character => character, months => months },
        )?;
        page(
            "milestones.html",
            format!("{slug}/milestones/index.html"),
            "../../",
            context! { character => character },
        )?;
        page(
            "feed.xml",
            format!("{slug}/feed.xml"),
            "../",
            context! {
                character => character,
                posts => posts.iter().take(FEED_POSTS).collect::<Vec<_>>(),
                base => link(base_url.as_deref().unwrap_or("../")),
            },
        )?;
    }

    for asset in ["assets/site.css", "assets/site.js"] {
        let text = theme_file(theme, asset)?.unwrap_or_default();
        files.push(SiteFile {
            path: asset.to_string(),
            content: SiteContent::Text(text),
        });
    }
    files.push(SiteFile {
        path: "assets/literata.woff2".into(),
        content: SiteContent::Bytes(crate::export::LITERATA.to_vec()),
    });
    if let Some(css) = custom_css {
        files.push(SiteFile {
            path: "assets/custom.css".into(),
            content: SiteContent::Text(css),
        });
    }
    files.extend(images);
    Ok(files)
}
