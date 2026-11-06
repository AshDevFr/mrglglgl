# mrglglgl

<!-- Written by sdd-bootstrap. Replace this section with what the project actually is. -->

## Task Workflow

- **Spec Repo**: `.specs/` (a separate, private repository)
- **PRD**: `.specs/docs/PRD.md`
- **Phase Specs**: `.specs/docs/specs/phase-N_<title>.md`
- **Task Directory**: `.specs/docs/tasks/phase-N/`
- **Task File Pattern**: `N.NN-descriptive-name.md`
- **Spec Model**: `flow-back`

## Companion

The desktop app lives in `companion/` (Rust workspace + `ui/` React frontend, Tauri v2).

- `make -C companion check`: build, clippy (`-D warnings`), Rust tests, frontend build, lint and tests
- `make -C companion dev`: run the app with hot reload
- Builds default to `nice -n 19` and two jobs so a game on the same machine stays responsive;
  `JOBS=0 NICE=` for full speed
- Rust tests regenerate the TypeScript bindings in `companion/ui/src/bindings/` (ts-rs); commit them
- Never write under the World of Warcraft folder; never execute Lua
