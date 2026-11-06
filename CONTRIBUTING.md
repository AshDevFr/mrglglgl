# Contributing

Thanks for helping the murloc. A few things keep the two halves of Mrglglgl working together.

## Layout

- `addon/`: the Lua addon. `make -C addon check` lints and runs every spec (busted on LuaJIT,
  against a mock of the game's API). See `addon/README.md`.
- `companion/`: the Rust and Tauri app. `make -C companion check` builds, lints and runs every test.
  See `companion/README.md`.
- `docs/schema.md`: the journal format, the contract between the two. A change to what the addon
  stores goes here first; the app's tests fail until it reads the documented fields.
- `fixtures/`: journal files both halves test against. Regenerate them with
  `make -C addon fixtures` after changing the addon; its specs fail while they are stale.
- `docs/forever-api.md`: what the addon relies on in the WoW: Forever client, and how each fact was
  verified. Add a row, with its evidence, before relying on a new client function.

## Rules that matter

- The addon never calls protected functions and never automates gameplay.
- The app never writes into the World of Warcraft folder and never runs Lua.
- No Blizzard art, sound or other assets in the repository.
- Player-facing text goes through the translation files (`addon/Locales/`,
  `companion/ui/src/i18n/locales/`), never as literal strings in code; the addon's lint enforces it.
- Fixtures and tests use fictional names only.

## Narrators and translations

New journal voices are data files; see the narrator files reference (`docs-site/src/content/docs/reference/narrator-files.md`). A translation of the addon is a new file
in `addon/Locales/`; a translation of the app is a new catalogue next to `en.json`.

## Commits

Conventional commit messages (`feat:`, `fix:`, `docs:`...), with a body that says why. Releases are
described in `docs/releasing.md`.
