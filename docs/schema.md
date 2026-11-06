# Mrglglgl SavedVariables schema

The contract between the Mrglglgl addon, which writes these tables, and the Mrglglgl app, which
reads them. The client writes them to disk on logout and `/reload`:

- `WTF/Account/<account>/SavedVariables/Mrglglgl.lua` holds `MrglglglDB` (account-wide settings)
- `WTF/Account/<account>/<number>/<Firstname>-<Surname>/SavedVariables/Mrglglgl.lua` holds
  `MrglglglCharDB` (one character's journal)

**Current journal schema: 1.** **Current settings schema: 1.**

## Versioning rules

- Adding a field does not change the schema number. Readers must ignore fields they do not know.
- Renaming or removing a field, or changing its meaning or type, increments the schema number and
  comes with a migration in the addon (`addon/Migrate.lua`).
- The addon never rewrites a journal whose schema is newer than its own: it leaves it untouched and
  records nothing.
- The app reads every schema up to the one it supports and refuses a newer one with a message,
  per character.

Keys are short because SavedVariables size counts. Types: `int` and `number` are Lua numbers,
`string` is a Lua string, `bool` is `true` / `false`, `?` marks a field that may be absent.

## `MrglglglDB` (account-wide)

| Field | Type | Meaning |
|---|---|---|
| `schema` | int | Settings schema version |
| `settings.record.sessions` | bool | Record logins and logouts |
| `settings.record.levels` | bool | Record level-ups |
| `settings.record.quests` | bool | Record quests accepted and turned in |
| `settings.record.questText` | bool | Also store each quest's description |
| `settings.record.zones` | bool | Record zone and subzone changes and discoveries |
| `settings.record.kills` | bool | Record kills by the player and party |
| `settings.record.deaths` | bool | Record the player's deaths |
| `settings.record.fishing` | bool | Record fishing catches |
| `settings.record.loot` | bool | Record notable loot |
| `settings.record.group` | bool | Record group members met |
| `settings.lootQuality` | int | Minimum item quality counted as notable loot (3 = rare) |
| `settings.retentionDays` | int | Days of raw events kept before compaction: 7, 14 or 30 |
| `settings.screenshots.levelUp` | bool | Take a screenshot on level-up |
| `settings.screenshots.death` | bool | Take a screenshot on death |
| `settings.screenshots.bossKill` | bool | Take a screenshot on a boss kill |
| `settings.screenshots.firstBoot` | bool | Take a screenshot on the first boot fished |
| `settings.screenshots.firstMurlocKill` | bool | Take a screenshot on the first murloc slain |
| `settings.screenshots.firstDeathByMurloc` | bool | Take a screenshot on the first death by murloc |

All `record` flags default to `true` and all `screenshots` flags to `false`.

## `MrglglglCharDB` (per character)

| Field | Type | Meaning |
|---|---|---|
| `schema` | int | Journal schema version |
| `char.name` | string | First name |
| `char.surname` | string? | Surname (Forever characters have one) |
| `char.realm` | string | Realm name from `GetRealmName()` |
| `char.realmId` | int | Realm ID from `GetRealmID()`; not the number in the WTF folder path |
| `char.race` | string | Race file name, e.g. `"Tauren"` |
| `char.class` | string | Class file name, e.g. `"HUNTER"` |
| `char.faction` | string | `"Alliance"` or `"Horde"` |
| `char.locale` | string | Client locale, e.g. `"enUS"`: the language of stored names and quest text |
| `days` | table | Day records keyed by local calendar date `"YYYY-MM-DD"` |
| `quests` | table | Quest text keyed by quest ID, stored once per quest |
| `companions` | table | Group members met, keyed by `"Firstname-Surname"` |
| `stats.boots` | int | Junk boots fished, all time |
| `stats.murlocsSlain` | int | Murlocs slain, all time |
| `stats.deathsByMurloc` | int | Deaths to a murloc, all time |
| `stats.drownings` | int | Deaths by drowning, all time |

### Day record (`days["YYYY-MM-DD"]`)

A day is a local calendar date. Each event belongs to the day of its own timestamp.

| Field | Type | Meaning |
|---|---|---|
| `ev` | array? | Raw events in time order; absent once the day is compacted |
| `kills` | table? | Kill aggregates keyed by NPC ID |
| `anonKills` | int? | Kills whose target the client withheld (dungeons, raids, encounters, PvP) |
| `digest` | table? | The day's summary; kept after compaction |

Fields added by later work (digest contents, per-kind event fields) are documented here as they
land.

### Quest record (`quests[questID]`)

| Field | Type | Meaning |
|---|---|---|
| `title` | string | Quest title |
| `text` | string? | Quest description; absent when `record.questText` was off |

### Companion record (`companions["Firstname-Surname"]`)

| Field | Type | Meaning |
|---|---|---|
| `first` | string | Day first met, `"YYYY-MM-DD"` |
| `last` | string | Day last met |
| `days` | int | Number of distinct days met |
