# WoW: Forever platform notes

What the Mrglglgl addon and app rely on about World of Warcraft: Forever, and how each fact was
established. Forever launched on 2026-11-04; much of the public documentation predates it, so
nothing here is assumed from other WoW flavors without saying so.

**Client build:** not yet confirmed in game (install metadata reports 1.60.1.70790)

## Status values

| Status | Meaning |
|---|---|
| `verified-in-game` | Observed on a live Forever client; the evidence cites the check (`S-NN`) that showed it |
| `verified-by-source` | Stated by a source that covers Forever specifically (Blizzard data, the client's own files, Forever-aware tooling or addon code) |
| `unconfirmed` | Not established for Forever; the evidence names the fallback the addon uses |

## Client identity and distribution

| ID | Item | Status | Evidence |
|---|---|---|---|
| ID-interface | `## Interface:` value is `16001` | verified-by-source | Warcraft Wiki [TOC format](https://warcraft.wiki.gg/wiki/TOC_format) interface table lists 16001 for Forever; installed Forever builds of Questie, GTFO, Plater, Prat and others declare `## Interface: 16001`; the BigWigs packager maps `16???` to the `forever` game type ([release.sh](https://github.com/BigWigsMods/packager/blob/master/release.sh)) |
| ID-toc-suffix | The client reads `<Addon>_Forever.toc` and `<Addon>_Camelot.toc` | verified-by-source | Warcraft Wiki TOC format: "Forever: `AddOnName_Forever.toc` or `AddOnName_Camelot.toc`", and the Forever changelog line "Added Camelot game type, Game variable value and TOC suffix, with Forever game type and TOC suffix alias". Installed Questie ships both. Note: the BigWigs packager recognises only `_Camelot` as a Forever suffix (it does not know `_Forever`); Wago accepts both |
| ID-toc-bare | With no matching suffixed TOC, the client loads the bare `<Addon>.toc` | verified-by-source | Warcraft Wiki TOC format: the game "will default to `AddOnName.toc` without a suffix" when no suffixed file matches. Family suffix `_Mainline` also applies to Forever and ranks below `_Forever` / `_Camelot` |
| ID-toc-multi-interface | `## Interface:` accepts a comma-separated list | verified-by-source | Warcraft Wiki TOC format (supported since patch 10.2.7); installed Forever-compatible addons (Altoholic, Auctionator, Bartender4) ship a single bare TOC listing `120105, 50504, 20506, 11509, 16001` |
| ID-folder | Install subfolder is `_forever_` | verified-by-source | Present in a live install at `World of Warcraft/_forever_/` alongside its own `Interface/` and `WTF/`; its `.flavor.info` names product `wowf` |
| ID-build | Client version 1.60.1, build 70790 | verified-by-source | Install metadata `.build.info` reports `1.60.1.70790`, product `wowf` |
| ID-curseforge-flavor | CurseForge flavor "Forever", game version "1.60.1" | verified-by-source | CurseForge file listings label Forever files with flavor "Forever" and game version "1.60.1" (e.g. [WoW Forever Suite files](https://www.curseforge.com/wow/addons/wow-forever-suite/files/all)); the packager uploads them with CurseForge game id 88568 |
| ID-wago-flavor | Wago flavor key `forever`, patch `1.60.1` | verified-by-source | [Wago game data API](https://addons.wago.io/api/data/game): `patches.forever = ["1.60.1"]`, `toc_suffixes.forever = ["-Camelot","_Camelot","-Forever","_Forever"]`, `supported_forever_patches = "1.60.1"` |
| ID-packager | BigWigs packager v2.6.0 or later supports Forever; v2.7.0 is current | verified-by-source | [Packager releases](https://github.com/BigWigsMods/packager/releases) (v2.7.0, 2026-11-05: "Don't allow Forever interface versions in `Classic` toc files", "Allow Forever interface versions in `Mainline` toc files"); `release.sh` maps interface `16???` and suffix `_Camelot` to game type `forever`, uploads to CurseForge (game id 88568) and Wago (`supported_forever_patches`), and skips WoWInterface, which has no Forever category |
| ID-camelot | "Camelot" is Forever's internal name: game type `camelot`, constant `WOW_PROJECT_CAMELOT`, TOC suffix `_Camelot` | verified-by-source | Forever changelog quoted on the Warcraft Wiki TOC format page ("The Camelot value may be changed to Forever in the future"); HereBeDragons detects Forever with `WOW_PROJECT_ID == WOW_PROJECT_CAMELOT`; client load screens live under `Camelot160\Main\` |

### Further identity findings

These were not on the original question list but change design decisions, so they are tracked
the same way.

| ID | Item | Status | Evidence |
|---|---|---|---|
| ID-client-family | Forever is in the **Mainline** (modern client) family, not the Classic family | verified-by-source | Warcraft Wiki TOC format: the `_Mainline` family suffix covers "standard, forever, camelot", while `_Classic` covers vanilla to mists; packager v2.7.0 allows Forever interfaces in `_Mainline` TOCs and rejects them in `_Classic` ones; Auctionator runs its modern auction house code on Forever and treats `WOW_PROJECT_CAMELOT` like Mainline. Consequence: Classic Era APIs are not a safe prior; modern (`C_*`) APIs may be the primary ones |
| ID-project-id | Value of `WOW_PROJECT_ID` on Forever | unconfirmed | Third-party notes say `WOW_PROJECT_CAMELOT` (18); TomTom instead detects Forever by interface range 16001 to 19999. fallback: detect Forever by `select(4, GetBuildInfo())` in 16000 to 19999, never by project ID alone (Compat.IsForever) |
| ID-surname | Characters have a surname, and `UnitName` returns it where other flavors return the realm | verified-by-source | Leatrix Plus Forever code: "Forever uses surnames where other games use realm names" and `local surname = isForever and realm or nil` after `UnitName(unit)` |
| ID-wtf-layout | Per-character folders are `WTF/Account/<account>/<number>/<FirstName>-<Surname>/` | verified-by-source | A live install's WTF tree: the realm level is a number (for example `93`) and character folders read like `Firstname-Surname`. Consequence: discovery cannot assume `<realm name>/<character>`, and a character's identity is first name plus surname |
