# Releasing

The addon and the app are released separately, each from its own tag on `main`.

| Part | Tag | Workflow | Publishes |
|---|---|---|---|
| Addon | `addon-vX.Y.Z` (prerelease: `addon-vX.Y.Z-beta.N`) | `addon-release.yml` | A GitHub release with `Mrglglgl-<tag>-forever.zip`; CurseForge and Wago too once their project IDs are in the TOC and `CF_API_KEY` / `WAGO_API_TOKEN` are repository secrets |
| App | `companion-vX.Y.Z` (prerelease: `companion-vX.Y.Z-beta.N`) | `companion-release.yml` | A GitHub release with the macOS `.dmg` and the Windows installer, and a changelog of the app's own commits |

The documentation site (`docs-site/`, Starlight) is not tagged: `docs.yml` rebuilds it and
publishes it to GitHub Pages on every push to `main` that touches it or the app's interface. Its
screenshots are taken in that run from the interface in demo mode (`npm --prefix companion/ui run
demo` shows the same thing locally; `npm --prefix docs-site run screenshots` takes them).

## Before tagging

1. `main` is green: the `addon` and `companion` workflows pass on the commit you will tag.
2. Locally, from the repository root:
   - `make -C addon check package-check`
   - `make -C companion check`
3. If the addon's journal format changed, `docs/schema.md` says so, the shared fixtures are
   regenerated (`make -C addon fixtures`), and the app reads them (its schema tests pass).
4. For the app: bump `version` in `companion/Cargo.toml` and in the Tauri config to the tag's
   version.

## Tagging

```sh
git tag -a addon-v0.2.0 -m "Mrglglgl addon 0.2.0"
git push origin addon-v0.2.0
```

Use `companion-v…` for the app. Watch the run with `gh run watch`; the release appears on the
repository's releases page when it finishes.

## Known issues

- The addon's in-game version reads the whole tag (for example `addon-v0.1.0-beta.2`), because the
  packager fills the TOC version from the tag name.
- App builds are not signed or notarised yet; the release notes tell players how to open them.
  An Apple Developer ID exists, so signing is an additive change to `companion-release.yml`.
- There is no auto-updater until builds are signed.
