# Lint and test the addon outside the game. Tools live in a project-local LuaRocks tree on LuaJIT,
# which speaks the same Lua 5.1 dialect as the WoW client.
LUA      ?= luajit
LUA_DIR  ?= $(shell brew --prefix luajit 2>/dev/null)
ROCKS    := .rocks
BUSTED   := $(ROCKS)/bin/busted
LUACHECK := $(ROCKS)/bin/luacheck
SPECS    ?=

.PHONY: setup lint test spec check

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
