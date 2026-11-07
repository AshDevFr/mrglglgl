# Mrglglgl SavedVariables schema

The contract between the Mrglglgl addon, which writes these tables, and the Mrglglgl app, which
reads them. The client writes them to disk on logout and `/reload`:

- `WTF/Account/<account>/SavedVariables/Mrglglgl.lua` holds `MrglglglDB` (account-wide settings)
- `WTF/Account/<account>/<number>/<Firstname>-<Surname>/SavedVariables/Mrglglgl.lua` holds
  `MrglglglCharDB` (one character's journal)

**Current journal schema: 1.** **Current settings schema: 2** (version 1 had a 14-day default
retention and offered 7 days).

The addon's TOC names the journal schema it writes, as `## X-Mrglglgl-Schema: 1`, so the app can
tell from the installed copy alone whether it will be able to read what that addon records.

## Versioning rules

- Adding a field does not change the schema number. Readers must ignore fields they do not know.
- Renaming or removing a field, or changing its meaning or type, increments the schema number and
  comes with a migration in the addon (`addon/Migrate.lua`).
- The addon never rewrites a journal whose schema is newer than its own: it leaves it untouched and
  records nothing.
- The app reads the schema it supports. An older journal is upgraded by the addon the next time
  that character logs in, so the app asks for that instead of reading old shapes; a newer journal
  needs an app update. Both are reported per character and never block the others.

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
| `settings.record.professions` | bool | Record profession skill-ups, crafting and gathering |
| `settings.record.money` | bool | Record the balance at each session's start and end, money earned and spent by source, and auction house events |
| `settings.record.achievements` | bool | Record achievements completed; on Forever these are the Legacy Challenges |
| `settings.record.chat` | bool | Record chat lines in the channels chosen under `settings.chat`; off by default |
| `settings.chat.say`, `settings.chat.emote`, `settings.chat.yell`, `settings.chat.party`, `settings.chat.raid`, `settings.chat.guild`, `settings.chat.officer`, `settings.chat.whisper`, `settings.chat.channel` | bool | Record the player's own lines in that channel while chat is on; `say`, `emote` and `yell` default to `true`, the rest to `false` |
| `settings.chat.sayOthers`, `settings.chat.emoteOthers`, `settings.chat.yellOthers`, `settings.chat.partyOthers`, `settings.chat.raidOthers`, `settings.chat.guildOthers`, `settings.chat.officerOthers`, `settings.chat.whisperOthers`, `settings.chat.channelOthers` | bool | Record other players' lines in that channel while chat is on; all default to `false` |
| `settings.lootQuality` | int | Minimum item quality counted as notable loot (3 = rare) |
| `settings.retentionDays` | int | Days of raw events kept before compaction: 7, 14 or 30 |
| `settings.screenshots.levelUp` | bool | Take a screenshot on level-up |
| `settings.screenshots.death` | bool | Take a screenshot on death |
| `settings.screenshots.bossKill` | bool | Take a screenshot on a boss kill |
| `settings.screenshots.firstBoot` | bool | Take a screenshot on the first boot fished |
| `settings.screenshots.firstMurlocKill` | bool | Take a screenshot on the first murloc slain |
| `settings.screenshots.firstDeathByMurloc` | bool | Take a screenshot on the first death by murloc |

All `record` flags default to `true` except `chat`, and all `screenshots` flags to `false`.

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
| `discovered` | table? | Areas (zone or subzone names) already recorded as discovered, as `name = true`; in the session that creates the journal of a character above level 1, places visited are added here without a `discover` event |
| `stats.boots` | int | Junk boots fished, all time |
| `stats.murlocsSlain` | int | Murlocs slain, all time |
| `stats.deathsByMurloc` | int | Deaths to a murloc, all time |
| `stats.drownings` | int | Deaths by drowning, all time |
| `addon.version` | string? | The addon's packaged version (its release tag), written at each login |
| `addon.build` | string | The client build at the last login, e.g. `70790` |
| `addon.interface` | int | The client's interface number at the last login, e.g. `16001` |
| `addon.login` | int | Unix time of the last login |
| `errors.n` | int | Errors the addon caught in its handlers, all time; recording carried on |
| `errors.last` | table? | The last one: `msg` (up to 300 characters), `event`, `t` (unix time) |

### Day record (`days["YYYY-MM-DD"]`)

A day is a local calendar date. Each event belongs to the day of its own timestamp.

| Field | Type | Meaning |
|---|---|---|
| `ev` | array? | Raw events in time order; once compacted, only the key moments (sessions, levels, deaths, bosses, discoveries, quests taken and handed in, screenshots, group joins, auction events, achievements), absent if there were none |
| `c` | bool? | `true` once the day is compacted: `ev` holds only key moments and `digest` is what the whole day was |
| `kills` | table? | Kill aggregates keyed by NPC ID |
| `anonKills` | int? | Kills whose target the client withheld (dungeons, raids, encounters, PvP) |
| `skills` | table? | Skills that rose that day, keyed by skill ID |
| `crafted` | table? | Items crafted that day, keyed by item ID |
| `gathered` | table? | Items mined, picked or skinned that day, keyed by item ID |
| `money` | table? | Money earned and spent that day by source; see Money |
| `chat` | array? | Chat lines in time order, when chat is recorded; see Chat. Removed when the day is compacted, and capped at 2000 lines a day |
| `digest` | table? | The day's summary, built from the whole day; never rebuilt after compaction |

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
| `session_start` | `bal` int? | Logged in; `bal` is the balance in copper, when money is recorded |
| `session_end` | `bal` int? | Logged out; `bal` is the balance in copper, when money is recorded |
| `level` | | Reached level `lv` |
| `zone` | | Entered zone `z` |
| `subzone` | | Entered subzone `sz` (at most once per subzone per minute) |
| `discover` | `area` string, `src` string | First time this character found `area`: `src` is `"message"` (the client's discovery message) or `"visit"` (first recorded visit) |
| `death` | `killer` table?, `env` string? | The player died. `killer` = `{ name, npc?, murloc }` for a creature (`npc` is its NPC ID when readable); `env` is the environmental cause from the client, e.g. `"Drowning"`, `"Falling"`. Neither is present when the client withheld or had no death recap |
| `boss` | `enc` int, `name` string | A boss encounter was won (`BOSS_KILL`) |
| `quest_accept` | `q` int | Accepted quest `q`; its title and description are in `quests[q]` |
| `quest_turnin` | `q` int, `xp` int, `money` int | Turned in quest `q`, with the XP and copper it rewarded |
| `fish` | `item` int, `name` string?, `junk` bool?, `boot` bool? | A fishing catch. `name`, `junk` (poor quality) and `boot` are filled once the client has the item's data; a catch whose data never arrived has only `item` |
| `loot` | `item` int, `name` string, `ql` int, `qty` int?, `src` string? | An item at or above `settings.lootQuality`; `qty` is absent when read from the loot window rather than the loot message. `src = "window"` marks an item only seen in a loot window, which in a group may have gone to someone else; absent when the character received it |
| `group_join` | `who` string | Grouped with `who` (`"Firstname-Surname"`) for the first time that day; see `companions[who]` |
| `achievement` | `ach` int, `name` string?, `cat` string?, `pts` int? | Completed achievement `ach` (on Forever, a Legacy Challenge, worth 0 points), with its category, e.g. "Kalimdor". One another character on the account earned first is not recorded |
| `auction` | `act` string, `name` string? | At the auction house: `act` is `"listed"`, `"sold"`, `"won"`, `"outbid"`, `"expired"` or `"cancelled"`; `name` is the item, absent for `"listed"` (the client's message does not name it) |
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

### Professions (`days[key].skills`, `.crafted`, `.gathered`)

Aggregated per day, because weapon skills rise every few hits and a crafting session makes dozens
of items. Skill IDs and item IDs do not depend on the client's language; names do.

| Field | Type | Meaning |
|---|---|---|
| `skills[id].name` | string | Skill name |
| `skills[id].cat` | string? | Heading the skill sits under in the game's skills list (e.g. "Professions", "Weapon Skills") |
| `skills[id].from`, `skills[id].to` | int | First and last rank seen that day |
| `crafted[id].n`, `gathered[id].n` | int | Items made, or loot slots gathered |
| `crafted[id].name`, `gathered[id].name` | string? | Item name |
| `gathered[id].via` | string | The gathering skill used, e.g. "Mining", "Herb Gathering", "Skinning" |

### Money (`days[key].money`)

Every change of the player's money, summed by where it came from (`inc`) or went (`out`), in
copper. The source is what the player was doing when the balance changed: `loot`, `quest`
(rewards), `vendor` (sales and purchases), `repair`, `trainer`, `travel` (flights), `trade`,
`mail`, `auction` (deposits and buyouts at the auction house, and sales and refunds taken from
its mail), or `other`.

| Field | Type | Meaning |
|---|---|---|
| `inc` | table? | Copper earned, by source |
| `out` | table? | Copper spent, by source |

### Chat (`days[key].chat[i]`)

A line said in a channel the player chose. Link, colour and texture codes are removed; an item
link reads as its name in brackets.

| Field | Type | Meaning |
|---|---|---|
| `t` | int | Unix time |
| `ch` | string | `say`, `emote` (a custom emote, without the speaker's name), `textEmote` (a built-in emote, a whole sentence such as "You wave."), `yell`, `party`, `raid`, `guild`, `officer`, `whisper` or `channel` |
| `msg` | string | What was said |
| `who` | string? | The speaker, as the client names them; absent for the player's own lines |
| `to` | string? | For a whisper the player sent: who it went to |
| `cn` | string? | For a numbered channel: its name, e.g. "General" |

### Digest (`days[key].digest`)

The day's summary, rebuilt at every login and logout while the day still has raw events. Once a
day is older than `settings.retentionDays`, its raw fields (`ev`, `kills`, `anonKills`, `skills`,
`crafted`, `gathered`) are removed and the digest is all that remains. `counts` are always exact;
lists hold the highlights: `kills` the 10 most killed, `deaths` 5, `loot`, `fish` and
`notableKills` 8, other lists 20. Digests of days older than 90 days are thinned further (see
`thin`).

| Field | Type | Meaning |
|---|---|---|
| `v` | int | Digest version. Days rebuilt from raw events get the current version; compacted days keep theirs |
| `counts` | table | `sessions`, `levels`, `accepted`, `turnedIn`, `deaths`, `fish`, `junk`, `boots`, `loot`, `discoveries`, `bosses`, `group`, `screenshots`, `kills` (including anonymous), `murlocs`, `anonKills`, `skillUps` (ranks gained), `crafted`, `gathered`, `moneyIn`, `moneyOut` (copper), `auctions`, `chat` (lines), `achievements` |
| `levelFrom`, `levelTo` | int? | Lowest and highest level seen that day |
| `zones` | array | Zones in the order first seen |
| `subzones` | array | Subzones in the order first seen |
| `discovered` | array | Areas discovered |
| `accepted`, `turnedIn` | array | Quest IDs |
| `deaths` | array | `{ killer?, env?, z, sz }`, as in `death` events |
| `kills` | array | `{ npc, name?, n, cls, murloc }`, most killed first, ties by NPC ID |
| `notableKills` | array | Kills of elite, rare, rare elite or world boss classification |
| `fish` | array | `{ item, name?, n, junk?, boot? }` per item caught |
| `loot` | array | `{ item, name, ql, qty?, src? }` |
| `companions` | array | `"Firstname-Surname"` of members met that day |
| `bosses` | array | Encounter names won |
| `skills` | array | `{ skill, name, cat?, from, to }` per skill that rose |
| `crafted`, `gathered` | array | `{ item, name?, n, via? }` per item |
| `money` | table | `{ inc = {source = copper}, out = {source = copper} }`, as the day's `money` |
| `auctions` | array | `{ act, name? }` per auction house event |
| `achievements` | array | `{ ach, name?, cat? }` per achievement completed |
| `balStart`, `balEnd` | int? | The day's first and last known balance, in copper |
| `onlyFished` | bool | Caught fish, and no kills, quest activity, crafting or gathering |
| `thin` | bool? | The day is older than 90 days: record lists keep 3 entries, name lists 5, `subzones` is empty; counts unchanged |
| `nothingHappened` | bool | No kills, deaths, quests, fishing, loot, levels, discoveries, bosses, companions or profession activity |

Fields added by later work are documented here as they land.

### Quest record (`quests[questID]`)

| Field | Type | Meaning |
|---|---|---|
| `title` | string | Quest title |
| `text` | string? | Quest description; absent when `record.questText` was off, and removed once the quest was last touched before the retention window (the title stays) |
| `last` | string? | Day the quest was last accepted or turned in, `"YYYY-MM-DD"` |
| `lv` | int? | The quest's level, from the quest log when it was accepted; absent when the client did not give it |
| `kind` | string? | `"elite"`, `"dungeon"`, `"raid"` or `"group"`, from the quest log's tag; absent for an ordinary quest or when the client did not give it |

### Companion record (`companions["Firstname-Surname"]`)

| Field | Type | Meaning |
|---|---|---|
| `first` | string | Day first met, `"YYYY-MM-DD"` |
| `last` | string | Day last met |
| `days` | int | Number of distinct days met |
