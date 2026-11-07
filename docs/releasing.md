# Releasing

The addon and the app are released separately, each from its own tag on `main`.

| Part | Tag | Workflow | Publishes |
|---|---|---|---|
| Addon | `addon-vX.Y.Z` (prerelease: `addon-vX.Y.Z-beta.N`) | `addon-release.yml` | A GitHub release with `Mrglglgl-<tag>-forever.zip`; CurseForge and Wago too once their project IDs are in the TOC and `CF_API_KEY` / `WAGO_API_TOKEN` are repository secrets |
| App | `companion-vX.Y.Z` (prerelease: `companion-vX.Y.Z-beta.N`) | `companion-release.yml` | A GitHub release with the macOS `.dmg` and the Windows installer, the signed update files and `latest.json` for the in-app updater, and a changelog of the app's own commits |

Both release workflows also record build provenance: GitHub signs, through Sigstore, that each
published file came from that workflow at that commit. Anyone can check a download with
`gh attestation verify <file> --repo AshDevFr/mrglglgl`.

The documentation site (`docs-site/`, Starlight) is not tagged: `docs.yml` rebuilds it and
uploads it to Cloudflare Pages on every push to `main` that touches it or the app's interface. Its
screenshots are taken in that run from the interface in demo mode (`npm --prefix companion/ui run
demo` shows the same thing locally; `npm --prefix docs-site run screenshots` takes them).

## Before tagging

1. `main` is green: the `addon` and `companion` workflows pass on the commit you will tag.
2. Locally, from the repository root:
   - `make -C addon check package-check`
   - `make -C companion check`
3. If the addon's journal format changed, `docs/schema.md` says so, the shared fixtures are
   regenerated (`make -C addon fixtures`), and the app reads them (its schema tests pass).
4. For the app: bump `version` in `companion/Cargo.toml`, the Tauri config and
   `companion/ui/package.json` to the tag's version.
5. For the app: the `TAURI_SIGNING_PRIVATE_KEY` secret holds the updater's private key (kept
   outside the repository, backed up). Without it the release still publishes, with no in-app
   update; the workflow warns.

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
- App builds are not signed with a developer certificate or notarised yet. macOS builds are signed
  ad hoc (`signingIdentity: "-"`): without it, macOS calls the downloaded app damaged and offers no
  way to open it. The release notes tell players to choose Open Anyway.
  An Apple Developer ID exists, so signing is an additive change to `companion-release.yml`.
- The in-app updater reads `latest.json` from the latest stable release only: prereleases are
  never offered. It verifies each update with the updater key, which is separate from code
  signing, so updates install on unsigned builds too.
