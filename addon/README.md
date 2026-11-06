# Mrglglgl addon

A quiet data collector for World of Warcraft: Forever. It records what happens in a character's
day into SavedVariables, which the Mrglglgl app reads and turns into a journal. In game it only has
a settings panel and a status command.

## Development

The addon is tested outside the game with [busted](https://lunarmodules.github.io/busted/) against
a thin mock of the client API (`spec/support/wow_mock.lua`), on LuaJIT, which speaks the same
Lua 5.1 dialect as the client. Tools install into a project-local tree (`.rocks/`).

```sh
brew install luajit luarocks   # once
make check                     # lint and run every spec
make spec SPECS="spec/compat_spec.lua"
```

`make lint` runs luacheck and `tools/check-strings.lua`, which rejects string literals passed to
anything that shows text to the player: user-facing text goes through `Locales/` so it can be
translated.

Client behaviour the addon relies on is recorded, with evidence, in `../docs/forever-api.md`.
Anything restricted or unconfirmed there is reached only through `Compat.lua`.

## Installing locally

Link this folder into the client's AddOns folder, then restart the game:

```sh
ln -s "$PWD" "/Applications/World of Warcraft/_forever_/Interface/AddOns/Mrglglgl"
```
