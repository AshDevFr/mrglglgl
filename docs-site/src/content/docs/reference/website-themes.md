---
title: Website themes
description: Restyle the journal website, override a template, or write a whole theme.
---

The app can keep a website of your journals up to date in a folder (Settings, Website). The site is
finished HTML, ready for Cloudflare Pages, Netlify or GitHub Pages, with nothing to build. How it
looks comes from a theme: Jinja-style templates (rendered by
[MiniJinja](https://docs.rs/minijinja/2)) and a stylesheet.

The default theme ships inside the app. Pick a **theme folder** in Settings to change it; there
are three levels, and you can stop at the first that does what you need.

## 1. Restyle with `custom.css`

Put a `custom.css` in the theme folder. It is copied to `assets/custom.css` and loaded after the
theme's own stylesheet on every page. Colours are custom properties, so a few lines go a long way:

```css
:root { --stamp: #7a2fb0; --page: #fbf8f2; }
.journal-page h2 { font-variant: small-caps; }
```

The properties are `--desk`, `--page`, `--page-edge`, `--ink`, `--ink-faded`, `--stamp`, `--tide`,
`--photo-frame`, `--reading-font` and `--ui-font`. The two papers and light and dark set them
under `[data-paper="parchment"]` and `[data-scheme="dark"]` on the `<html>` element; override
those selectors to change one combination.

## 2. Override one template

Any file in the theme folder replaces the default theme's file of the same name; every other file
still comes from the default. Templates extend `base.html` and replace its blocks:

```jinja
{% extends "base.html" %}
{% block footer %}Written in Mulgore, between quests.{% endblock %}
```

Blocks in `base.html`: `title`, `head` (add your own tags: analytics, fonts, meta), `menu`,
`content`, `footer`.

## 3. Start a whole theme

"Copy the default theme here" in Settings writes every template and asset into the theme folder
(files already there are kept). Edit what you like, and delete what you did not change so it
keeps following the default theme in later versions.

## Templates and what they receive

Every template receives `site`, `root`, `labels`, `has_custom_css` and `canonical` (the page's full address; only when the site has a `base_url`). Use `{{ root }}` in front
of every link: it leads back to the site's top folder from wherever the page is, so the site works
at any address or sub-path.

| Template | Page | Also receives |
|---|---|---|
| `index.html` | `index.html` | (with one character, the default theme sends the reader straight to their journal) |
| `character.html` | `<character>/` and `<character>/page/N/` | `character`, `section` (`posts`), `share_image`, `posts` (this page's, newest first), `page` (`number`, `total`, `newer_url`, `older_url`) |
| `post.html` | `<character>/<year>-w<week>/` | `character`, `section` (`post`), `share_image`, `post`, `newer_post`, `older_post` |
| `archive.html` | `<character>/archive/` | `character`, `section` (`archive`), `months` (each `month`, `label`, `posts`) |
| `milestones.html` | `<character>/milestones/` | `character`, `section` (`milestones`), `milestones` (the character's, each with `day_title` and `url` added) |
| `feed.xml` | `<character>/feed.xml` | `character`, `posts` (the newest twenty), `base` (the site address) |
| `assets/site.css` | the stylesheet | |
| `assets/site.js` | the light/dark and paper switches | |
| `assets/favicon.svg` | the site's icon (a small murloc); replace it with your own file of the same name | |

### The data

- `site`: `title`, `language`, `base_url`, `paper` (`tide` or `parchment`, what the site starts
  on), `characters`.
- `character`: `slug` (its folder), `name`, `summary` ("Tauren Hunter, level 10"), `posts`,
  `milestones`.
- `post` (one week): `slug`, `title`, `date` (the Monday, `YYYY-MM-DD`), `month`, `month_label`,
  `excerpt`, `chapter_html` (the week's chapter, when written), `days`. In `character.html` and
  `post.html` a post also has `cover` (its first photo: `path` from the site's top folder, and
  `caption`; empty when the week has none) and `stamps` (every stamp of its days).
- `day`: `date`, `title`, `html` (the entry, or what happened, as HTML), `written` (whether the
  narrator wrote it), `stamps`, `photos`.
- `stamp` (a milestone): `name`, `description`, `day`, `svg` (the ink stamp). In the
  `milestones` list of `milestones.html` a stamp also has `day_title` (the day it was earned, as
  the day's title) and `url` (a link to that day on its post, with `{{ stamp.url | safe }}`; empty
  when no post holds the day).
- `share_image`: a full address of a photo for link previews: a post's first photo, or the
  newest photo-bearing post's on a character page. Only set when the site has a `base_url` and
  there is a photo.
- `photo`: `file` (in `images/<character>/`), `caption`.
- `labels`: the theme's words in the app's language (`posts`, `archive`, `milestones`, `feed`,
  `feed_help`, `newer`, `older`, `previous_week`, `next_week`, `notes`, ...). The default theme falls back to English
  with `{{ labels.posts | default('Posts') }}`.

`html`, `chapter_html` and `svg` are markup the app made: print them with `| safe`. Everything
else is escaped for you.

The default `base.html` is one compact header row: the site title, the character's sections
(Posts, Archive, Milestones, the open one marked with `aria-current` from `section`) and two quiet
icon buttons for light/dark and paper. A character switcher menu (`site-menu`) appears only when
the site has more than one character. The feed link lives in the footer block, with the
character's feed address. Every page gets a description, an icon and Open Graph tags (a post's
`excerpt`, else the character's `summary`), so a link shared in a chat shows a preview; the
address (`og:url`) and photo (`og:image`, which makes the card `summary_large_image`) are added
only when the site has a `base_url`, because previews need full addresses. Each photo links to
its full-size image. A day the narrator has not
written is labelled with `labels.notes` above its notes.

Filters beyond MiniJinja's built-ins: `rfc822` turns a `YYYY-MM-DD` date into a feed date.

## What the site never shows

Only finished days and accepted entries; never today or an entry told so far. Other players
appear under stand-in names unless you turn that off, and realm names are left out. Templates
only see the data above, never the journal itself.

## When something is wrong

A template error stops the build, nothing is written, and Settings shows the file and line, e.g.
`post.html, line 3: unknown filter`. Themes cannot include files from outside their folder.
