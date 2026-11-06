.PHONY: help \
	bindings ui-install ui-build ui-lint ui-lint-fix ui-test \
	fmt lint test check ci dev build \
	changelog changelog-unreleased setup-hooks clean

# The game may be running on the same machine: builds run at low priority and
# with a bounded number of jobs by default. Override with JOBS=0 NICE= for a
# full-speed build.
JOBS ?= 2
NICE ?= nice -n 19
CARGO := $(NICE) cargo
CARGO_JOBS := $(if $(filter 0,$(JOBS)),,-j $(JOBS))
NPM := $(NICE) npm --prefix ui

# Colors
BLUE := \033[0;34m
GREEN := \033[0;32m
NC := \033[0m

help: ## Show this help message
	@echo "$(BLUE)Mrglglgl app development commands$(NC)"
	@echo ""
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  $(GREEN)%-22s$(NC) %s\n", $$1, $$2}'

# ── Frontend ─────────────────────────────────────────────────────────────────

ui/node_modules: ui/package.json ui/package-lock.json
	$(NPM) ci
	@touch ui/node_modules

ui-install: ui/node_modules ## Install frontend dependencies

# The frontend type-checks against these, so they are written before it builds. The app crate's
# own command types are written by `make test` (it embeds ui/dist, so it builds after the UI).
bindings: ## Regenerate the TypeScript types of the core data the app sends to the frontend
	$(CARGO) test -p mrglglgl-core --features ts --lib $(CARGO_JOBS) export_bindings

ui-build: ui-install bindings ## Build the frontend into ui/dist (the app embeds it)
	$(NPM) run build

ui-lint: ui-install ## Lint the frontend (biome)
	$(NPM) run lint

ui-lint-fix: ui-install ## Lint and auto-fix the frontend
	$(NPM) run lint:fix

ui-test: ui-install ## Run frontend tests (Vitest)
	$(NPM) run test:run

# ── Rust ─────────────────────────────────────────────────────────────────────

fmt: ## Format Rust code
	cargo fmt --all

# The app crate embeds ui/dist at compile time, so the frontend is built first.
lint: ui-build ## Run clippy with warnings as errors
	$(CARGO) clippy --workspace --all-targets $(CARGO_JOBS) -- -D warnings

test: ui-build ## Run Rust tests (also regenerates the TypeScript bindings)
	$(CARGO) test --workspace $(CARGO_JOBS)

core-test: ## Run one mrglglgl-core integration test target: make core-test T=sv
	@test -n "$(T)" || { echo "T is empty"; exit 1; }
	$(CARGO) test -p mrglglgl-core --test $(T) $(CARGO_JOBS)

app-test: ## Run the app crate's tests (also writes its TypeScript command types)
	$(CARGO) test -p mrglglgl-app $(CARGO_JOBS) $(ARGS)

llm-test: ## Run one mrglglgl-llm integration test target: make llm-test T=provider
	@test -n "$(T)" || { echo "T is empty"; exit 1; }
	$(CARGO) test -p mrglglgl-llm --test $(T) $(CARGO_JOBS)

check: ui-build ui-lint ## Build, lint and test everything
	cargo fmt --all -- --check
	$(CARGO) build --workspace $(CARGO_JOBS)
	$(CARGO) clippy --workspace --all-targets $(CARGO_JOBS) -- -D warnings
	$(CARGO) test --workspace $(CARGO_JOBS)
	$(NPM) run test:run

ci: check ## What CI runs

# ── App ──────────────────────────────────────────────────────────────────────

dev: ui-install ## Run the app with hot reload (Vite + Tauri)
	cd crates/mrglglgl-app && $(NICE) ../../ui/node_modules/.bin/tauri dev

build: ui-install ## Build the app and its installers
	cd crates/mrglglgl-app && $(NICE) ../../ui/node_modules/.bin/tauri build

# ── Changelog (git-cliff) ────────────────────────────────────────────────────

changelog: ## Regenerate CHANGELOG.md from the app's commits
	git-cliff --config cliff.toml --include-path 'companion/**' --repository .. -o CHANGELOG.md

changelog-unreleased: ## Print unreleased app changes
	git-cliff --config cliff.toml --include-path 'companion/**' --repository .. --unreleased

# ── Misc ─────────────────────────────────────────────────────────────────────

setup-hooks: ## Install the pre-commit hooks
	./scripts/setup-hooks.sh

clean: ## Remove build output
	cargo clean
	rm -rf ui/dist

# Checks the user documentation (every guide exists; the privacy text matches the app).
docs-check:
	scripts/check-docs.sh
