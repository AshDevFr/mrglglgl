# Mrglglgl

A journal of your World of Warcraft: Forever character's life, kept by a murloc who writes better
than you would expect.

> Day 12. Slew 23 wolves near the Brackwell Pumpkin Patch. Went fishing. Caught a boot. Kept the boot.

Mrglglgl has two parts:

- **The addon** quietly records your character's day in game: quests, kills, deaths and what
  killed you, fishing (boots included), professions, places discovered, notable loot, the people
  you grouped with. It shows nothing unless you ask (`/mrgl status`), and you choose what it
  records in Options > AddOns > Mrglglgl.
- **The app** (Windows and macOS) finds your WoW install, keeps a full history of every character's
  journal, and has the murloc scribe write each day and week as a short, cozy journal entry. It
  writes with a small language model that runs on your own computer, downloaded from inside the
  app; no account or online service is needed. A remote model you choose (any OpenAI-compatible
  endpoint, such as Ollama, LM Studio or a cloud provider) is optional.

Documentation, with screenshots: **https://mrglglgl.4sh.dev**

## Install

**Addon:** download the latest `Mrglglgl-addon-…-forever.zip` from the
[releases](https://github.com/AshDevFr/mrglglgl/releases), extract it into
`World of Warcraft/_forever_/Interface/AddOns/`, and restart the game. It needs no setup. Or let
the app install and update it: Settings, Addon, Install.

**App:** download the installer for your system from the same releases page. The builds are not
signed with a developer certificate yet, so your system asks once whether to trust them: on macOS,
open the app once, then choose Open Anyway under System Settings > Privacy & Security; on Windows,
choose More info then Run anyway in the SmartScreen prompt.

## Privacy

The addon writes only to the game's own SavedVariables files. The app reads those files and writes
nothing into the World of Warcraft folder except the addon's own folder, when you ask it to
install, update or remove the addon, and never while the game runs. With the built-in local model, nothing leaves
your computer. If you configure a remote endpoint that is not on your own machine, the day's
journal facts, including your character's name, the names of people you grouped with, and quest
descriptions, are sent to that endpoint to write the entry; the app's settings say so before you
use it.

## Repository

| Path | What it is |
|---|---|
| `addon/` | The Lua addon (see `addon/README.md` for development) |
| `companion/` | The Rust and Tauri app (see `companion/README.md`) |
| `docs-site/` | The documentation site (Starlight); its screenshots are taken in CI from the app's demo mode |
| `docs/schema.md` | The data the addon writes: the contract between the two parts |
| `docs/forever-api.md` | What the addon relies on in the Forever client, and how each fact was verified |
| `fixtures/` | Journal files shared by both parts' tests |

## Licence

MIT, see `LICENSE`. World of Warcraft is a trademark of Blizzard Entertainment; Mrglglgl is an
unofficial fan project and ships no Blizzard assets.
