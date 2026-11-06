# Lint and test the addon outside the game. Tools live in a project-local LuaRocks tree on LuaJIT,
# which speaks the same Lua 5.1 dialect as the WoW client.
LUA      ?= luajit
LUA_DIR  ?= $(shell brew --prefix luajit 2>/dev/null)
ROCKS    := .rocks
BUSTED   := $(ROCKS)/bin/busted
LUACHECK := $(ROCKS)/bin/luacheck
SPECS    ?=

# Day keys and timestamps come from local time; one fixed zone makes specs and fixtures identical
# on every machine and in CI.
export TZ := UTC

.PHONY: setup lint test spec check fixtures package-check

$(BUSTED) $(LUACHECK):
	luarocks --lua-version 5.1 $(if $(LUA_DIR),--lua-dir $(LUA_DIR)) --tree $(ROCKS) install busted
	luarocks --lua-version 5.1 $(if $(LUA_DIR),--lua-dir $(LUA_DIR)) --tree $(ROCKS) install luacheck

setup: $(BUSTED) $(LUACHECK)

lint: setup
	$(LUACHECK) .
	$(LUA) tools/check-strings.lua

test: setup
	$(BUSTED)

# Runs only the named spec files, failing if any of them does not exist:
#   make spec SPECS="spec/storage_spec.lua spec/migrate_spec.lua"
spec: setup
	@test -n "$(SPECS)" || { echo "SPECS is empty"; exit 1; }
	@for f in $(SPECS); do test -f "$$f" || { echo "missing $$f"; exit 1; }; done
	$(BUSTED) $(SPECS)

check: lint test

# Regenerates fixtures/savedvariables/ from the addon's own code.
fixtures:
	$(LUA) tools/fixtures.lua

# Checks the package definition (TOC and .pkgmeta) without building the zip; the zip itself is
# built by the BigWigs packager in the release workflow.
package-check:
	$(LUA) tools/check-package.lua
