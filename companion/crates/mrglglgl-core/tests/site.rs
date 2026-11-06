use std::path::{Path, PathBuf};

use mrglglgl_core::site::{
    self, SiteCharacter, SiteContent, SiteData, SiteDay, SiteError, SiteFile, SitePhoto, SitePost,
    SiteStamp,
};

const STAMP_SVG: &str = "<svg class=\"ink-stamp\"><text>KEPT THE BOOT</text></svg>";

fn day(date: &str, photo: Option<&Path>) -> SiteDay {
    SiteDay {
        date: date.into(),
        title: format!("Day {date}"),
        html: "<p class=\"entry\">The wolves of Mulgore kept me busy.</p>".into(),
        written: true,
        stamps: vec![SiteStamp {
            name: "Kept the Boot".into(),
            description: "Fished up a boot and kept it.".into(),
            day: date.into(),
            svg: STAMP_SVG.into(),
        }],
        photos: photo
            .map(|p| SitePhoto {
                source: p.to_string_lossy().into_owned(),
                file: format!("{date}-1.jpg"),
                caption: "Level up in Red Rocks, 9:41 PM".into(),
            })
            .into_iter()
            .collect(),
    }
}

/// Twelve weekly posts, the newest the week of 2026-11-02, the first day with a screenshot.
fn sample(photo: &Path) -> SiteData {
    let posts = (0..12)
        .map(|i| {
            let monday =
                chrono::NaiveDate::from_ymd_opt(2026, 11, 2).unwrap() - chrono::Days::new(7 * i);
            let date = monday.format("%Y-%m-%d").to_string();
            let week = monday.format("%G-w%V").to_string();
            SitePost {
                slug: week,
                title: format!("Week of <b>{date}</b>"),
                date: date.clone(),
                month: monday.format("%Y-%m").to_string(),
                month_label: monday.format("%B %Y").to_string(),
                excerpt: "A week of wolves.".into(),
                chapter_html: (i == 0).then(|| "<p class=\"entry\">A chapter.</p>".into()),
                days: vec![day(&date, (i == 0).then_some(photo))],
            }
        })
        .collect();
    SiteData {
        title: "Wren's travels".into(),
        language: "en".into(),
        base_url: Some("https://example.com/".into()),
        paper: "parchment".into(),
        labels: [("posts".to_string(), "Posts".to_string())].into(),
        characters: vec![SiteCharacter {
            slug: "wren".into(),
            name: "Wren Hollowbrook".into(),
            summary: "Tauren Hunter, level 10".into(),
            posts,
            milestones: vec![SiteStamp {
                name: "Kept the Boot".into(),
                description: "Fished up a boot and kept it.".into(),
                day: "2026-11-02".into(),
                svg: STAMP_SVG.into(),
            }],
        }],
    }
}

fn photo(dir: &Path) -> PathBuf {
    let path = dir.join("WoWScrnShot_110226_214100.png");
    image::RgbImage::from_pixel(64, 32, image::Rgb([40, 90, 80]))
        .save(&path)
        .unwrap();
    path
}

fn text<'a>(files: &'a [SiteFile], path: &str) -> &'a str {
    match &files
        .iter()
        .find(|f| f.path == path)
        .unwrap_or_else(|| panic!("no {path}"))
        .content
    {
        SiteContent::Text(t) => t,
        other => panic!("{path} is not text: {other:?}"),
    }
}

#[test]
fn a_site_has_a_home_posts_in_pages_of_ten_an_archive_milestones_and_a_feed() {
    let dir = tempfile::tempdir().unwrap();
    let source = photo(dir.path());
    let files = site::render(&sample(&source), None).unwrap();
    let paths: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();
    for expected in [
        "index.html",
        "wren/index.html",
        "wren/page/2/index.html",
        "wren/2026-w45/index.html",
        "wren/archive/index.html",
        "wren/milestones/index.html",
        "wren/feed.xml",
        "assets/site.css",
        "assets/site.js",
        "assets/literata.woff2",
        "images/wren/2026-11-02-1.jpg",
    ] {
        assert!(paths.contains(&expected), "missing {expected}: {paths:?}");
    }
    assert!(!paths.contains(&"wren/page/3/index.html"));
    assert!(!paths.contains(&"assets/custom.css"));

    let home = text(&files, "index.html");
    assert!(
        home.contains("data-paper=\"parchment\""),
        "the site starts on the chosen paper"
    );
    assert!(home.contains("Wren&#x27;s travels") || home.contains("Wren's travels"));
    assert!(home.contains("href=\"wren/\""));
    assert!(home.contains("Tauren Hunter, level 10"));

    let post = text(&files, "wren/2026-w45/index.html");
    assert!(
        post.contains("href=\"../../assets/site.css\""),
        "relative links"
    );
    assert!(post.contains("<p class=\"entry\">The wolves of Mulgore kept me busy.</p>"));
    assert!(post.contains("<p class=\"entry\">A chapter.</p>"));
    assert!(post.contains(STAMP_SVG), "stamps are drawn");
    assert!(post.contains("src=\"../../images/wren/2026-11-02-1.jpg\""));
    assert!(post.contains("Level up in Red Rocks, 9:41 PM"));
    assert!(
        post.contains("Week of &lt;b&gt;2026-11-02") && !post.contains("<b>"),
        "text from the journal is escaped"
    );
    assert!(!post.contains("custom.css"));

    let first = text(&files, "wren/index.html");
    assert_eq!(first.matches("class=\"post-card\"").count(), 10);
    assert!(
        first.contains("href=\"../wren/page/2/\""),
        "on to the older posts"
    );
    let second = text(&files, "wren/page/2/index.html");
    assert_eq!(second.matches("class=\"post-card\"").count(), 2);
    assert!(
        second.contains("href=\"../../../wren/\""),
        "back to the newer posts"
    );

    let archive = text(&files, "wren/archive/index.html");
    assert!(archive.contains("November 2026"));
    assert!(archive.contains("August 2026"));

    let feed = text(&files, "wren/feed.xml");
    assert!(feed.starts_with("<?xml"));
    assert!(feed.contains("<link>https://example.com/wren/2026-w45/</link>"));
    assert!(feed.contains("<pubDate>Mon, 02 Nov 2026 00:00:00 +0000</pubDate>"));

    match &files
        .iter()
        .find(|f| f.path == "images/wren/2026-11-02-1.jpg")
        .unwrap()
        .content
    {
        SiteContent::Image(from) => assert_eq!(from, &source),
        other => panic!("not an image: {other:?}"),
    }
}

#[test]
fn a_theme_folder_overrides_one_template_and_adds_custom_css() {
    let dir = tempfile::tempdir().unwrap();
    let source = photo(dir.path());
    let theme = dir.path().join("theme");
    std::fs::create_dir_all(&theme).unwrap();
    std::fs::write(
        theme.join("post.html"),
        "{% extends \"base.html\" %}{% block content %}<h1>MINE {{ post.slug }}</h1>{% endblock %}",
    )
    .unwrap();
    std::fs::write(theme.join("custom.css"), "body { color: hotpink; }").unwrap();

    let files = site::render(&sample(&source), Some(&theme)).unwrap();
    let post = text(&files, "wren/2026-w45/index.html");
    assert!(
        post.contains("<h1>MINE 2026-w45</h1>"),
        "the override is used"
    );
    assert!(
        post.contains("href=\"../../assets/site.css\""),
        "inside the default frame"
    );
    assert!(post.contains("href=\"../../assets/custom.css\""));
    assert!(
        text(&files, "wren/index.html").contains("class=\"post-card\""),
        "other templates still come from the default theme"
    );
    assert_eq!(
        text(&files, "assets/custom.css"),
        "body { color: hotpink; }"
    );
}

#[test]
fn a_template_error_names_the_file_and_line_and_builds_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let source = photo(dir.path());
    let theme = dir.path().join("theme");
    std::fs::create_dir_all(&theme).unwrap();
    std::fs::write(
        theme.join("post.html"),
        "{% extends \"base.html\" %}\n{% block content %}\n{{ post.title | no_such_filter }}\n{% endblock %}",
    )
    .unwrap();
    match site::render(&sample(&source), Some(&theme)) {
        Err(SiteError::Template(message)) => {
            assert!(message.contains("post.html"), "{message}");
            assert!(message.contains("line 3"), "{message}");
        }
        other => panic!("expected a template error, got {other:?}"),
    }
}

#[test]
fn a_theme_cannot_reach_outside_its_folder() {
    let dir = tempfile::tempdir().unwrap();
    let source = photo(dir.path());
    let theme = dir.path().join("theme");
    std::fs::create_dir_all(&theme).unwrap();
    std::fs::write(dir.path().join("secret.html"), "SECRET").unwrap();
    std::fs::write(
        theme.join("post.html"),
        "{% extends \"base.html\" %}{% block content %}{% include \"../secret.html\" ignore missing %}{% endblock %}",
    )
    .unwrap();
    let files = site::render(&sample(&source), Some(&theme)).unwrap();
    assert!(!text(&files, "wren/2026-w45/index.html").contains("SECRET"));
}
