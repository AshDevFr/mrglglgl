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
| `discovered` | table? | Areas (zone or subzone names) already recorded as discovered, as `name = true` |
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

### Event (`days[key].ev[i]`)

Every event has these fields:

| Field | Type | Meaning |
|---|---|---|
| `t` | int | Unix time of the event |
| `k` | string | Event kind, below |
| `z` | string | Zone name, in the client's language |
| `sz` | string | Subzone name, empty when none |
| `m` | int? | Map ID from `C_Map.GetBestMapForUnit` |
| `lv` | int | Player level; for `level` events, the new level |

Event kinds and their extra fields:

| Kind | Extra fields | Meaning |
|---|---|---|
| `session_start` | | Logged in |
| `session_end` | | Logged out |
| `level` | | Reached level `lv` |
| `zone` | | Entered zone `z` |
| `subzone` | | Entered subzone `sz` (at most once per subzone per minute) |
| `discover` | `area` string, `src` string | First time this character found `area`: `src` is `"message"` (the client's discovery message) or `"visit"` (first recorded visit) |
| `death` | `killer` table?, `env` string? | The player died. `killer` = `{ name, npc?, murloc }` for a creature (`npc` is its NPC ID when readable); `env` is the environmental cause from the client, e.g. `"Drowning"`, `"Falling"`. Neither is present when the client withheld or had no death recap |
| `boss` | `enc` int, `name` string | A boss encounter was won (`BOSS_KILL`) |
| `quest_accept` | `q` int | Accepted quest `q`; its title and description are in `quests[q]` |
| `quest_turnin` | `q` int, `xp` int, `money` int | Turned in quest `q`, with the XP and copper it rewarded |
| `fish` | `item` int, `name` string?, `junk` bool?, `boot` bool? | A fishing catch. `name`, `junk` (poor quality) and `boot` are filled once the client has the item's data; a catch whose data never arrived has only `item` |
| `loot` | `item` int, `name` string, `ql` int, `qty` int? | Received an item at or above `settings.lootQuality`; `qty` is absent when read from the loot window rather than the loot message |
| `group_join` | `who` string | Grouped with `who` (`"Firstname-Surname"`) for the first time that day; see `companions[who]` |
| `screenshot` | `moment` string | The addon took a screenshot: `"levelUp"`, `"death"`, `"bossKill"`, `"firstBoot"`, `"firstMurlocKill"` or `"firstDeathByMurloc"`. The file is `_forever_/Screenshots/WoWScrnShot_MMDDYY_HHMMSS.jpg` taken about a second after `t` |

### Kill aggregate (`days[key].kills[npcID]`)

Kills by the player or a party member, one record per creature type per day, keyed by NPC ID (the
sixth field of the creature GUID), so it does not depend on the client's language.

| Field | Type | Meaning |
|---|---|---|
| `n` | int | Kills that day |
| `name` | string? | Creature name, in the client's language; absent when never seen and no XP message named it |
| `cls` | string | `"normal"`, `"elite"`, `"rare"`, `"rareelite"`, `"worldboss"`, `"trivial"`, `"minus"` or `"unknown"` |
| `murloc` | bool | Identified as a murloc, by NPC ID or by name |

`anonKills` counts kills whose target the client withheld from addons; they have no aggregate.

### Digest (`days[key].digest`)

The day's summary, rebuilt at every login and logout while the day still has raw events. Once a
day is older than `settings.retentionDays`, its `ev`, `kills` and `anonKills` are removed and the
digest is all that remains. Lists are capped at 20 entries and `kills` at the 15 most killed;
`counts` are always exact.

| Field | Type | Meaning |
|---|---|---|
| `v` | int | Digest version. Days rebuilt from raw events get the current version; compacted days keep theirs |
| `counts` | table | `sessions`, `levels`, `accepted`, `turnedIn`, `deaths`, `fish`, `junk`, `boots`, `loot`, `discoveries`, `bosses`, `group`, `screenshots`, `kills` (including anonymous), `murlocs`, `anonKills` |
| `levelFrom`, `levelTo` | int? | Lowest and highest level seen that day |
| `zones` | array | Zones in the order first seen |
| `subzones` | array | Subzones in the order first seen |
| `discovered` | array | Areas discovered |
| `accepted`, `turnedIn` | array | Quest IDs |
| `deaths` | array | `{ killer?, env?, z, sz }`, as in `death` events |
| `kills` | array | `{ npc, name?, n, cls, murloc }`, most killed first, ties by NPC ID |
| `notableKills` | array | Kills of elite, rare, rare elite or world boss classification |
| `fish` | array | `{ item, name?, n, junk?, boot? }` per item caught |
| `loot` | array | `{ item, name, ql, qty? }` |
| `companions` | array | `"Firstname-Surname"` of members met that day |
| `bosses` | array | Encounter names won |
| `onlyFished` | bool | Caught fish, and no kills or quest activity |
| `nothingHappened` | bool | No kills, deaths, quests, fishing, loot, levels, discoveries, bosses or companions |

Fields added by later work are documented here as they land.

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
