# WoW: Forever platform notes

What the Mrglglgl addon and app rely on about World of Warcraft: Forever, and how each fact was
established. Forever launched on 2026-11-04; much of the public documentation predates it, so
nothing here is assumed from other WoW flavors without saying so.

**Client build:** not yet confirmed in game (install metadata reports 1.60.1.70790)

## Status values

| Status | Meaning |
|---|---|
| `verified-in-game` | Observed on a live Forever client; the evidence cites the check (`S-NN`) that showed it |
| `verified-by-source` | Stated by a source that covers Forever specifically (Blizzard data, the client's own files, Forever-aware tooling or addon code) |
| `unconfirmed` | Not established for Forever; the evidence names the fallback the addon uses |

## Client identity and distribution

| ID | Item | Status | Evidence |
|---|---|---|---|
| ID-interface | `## Interface:` value is `16001` | verified-by-source | Warcraft Wiki [TOC format](https://warcraft.wiki.gg/wiki/TOC_format) interface table lists 16001 for Forever; installed Forever builds of Questie, GTFO, Plater, Prat and others declare `## Interface: 16001`; the BigWigs packager maps `16???` to the `forever` game type ([release.sh](https://github.com/BigWigsMods/packager/blob/master/release.sh)) |
| ID-toc-suffix | The client reads `<Addon>_Forever.toc` and `<Addon>_Camelot.toc` | verified-by-source | Warcraft Wiki TOC format: "Forever: `AddOnName_Forever.toc` or `AddOnName_Camelot.toc`", and the Forever changelog line "Added Camelot game type, Game variable value and TOC suffix, with Forever game type and TOC suffix alias". Installed Questie ships both. Note: the BigWigs packager recognises only `_Camelot` as a Forever suffix (it does not know `_Forever`); Wago accepts both |
| ID-toc-bare | With no matching suffixed TOC, the client loads the bare `<Addon>.toc` | verified-by-source | Warcraft Wiki TOC format: the game "will default to `AddOnName.toc` without a suffix" when no suffixed file matches. Family suffix `_Mainline` also applies to Forever and ranks below `_Forever` / `_Camelot` |
| ID-toc-multi-interface | `## Interface:` accepts a comma-separated list | verified-by-source | Warcraft Wiki TOC format (supported since patch 10.2.7); installed Forever-compatible addons (Altoholic, Auctionator, Bartender4) ship a single bare TOC listing `120105, 50504, 20506, 11509, 16001` |
| ID-folder | Install subfolder is `_forever_` | verified-by-source | Present in a live install at `World of Warcraft/_forever_/` alongside its own `Interface/` and `WTF/`; its `.flavor.info` names product `wowf` |
| ID-build | Client version 1.60.1, build 70790 | verified-by-source | Install metadata `.build.info` reports `1.60.1.70790`, product `wowf` |
| ID-curseforge-flavor | CurseForge flavor "Forever", game version "1.60.1" | verified-by-source | CurseForge file listings label Forever files with flavor "Forever" and game version "1.60.1" (e.g. [WoW Forever Suite files](https://www.curseforge.com/wow/addons/wow-forever-suite/files/all)); the packager uploads them with CurseForge game id 88568 |
| ID-wago-flavor | Wago flavor key `forever`, patch `1.60.1` | verified-by-source | [Wago game data API](https://addons.wago.io/api/data/game): `patches.forever = ["1.60.1"]`, `toc_suffixes.forever = ["-Camelot","_Camelot","-Forever","_Forever"]`, `supported_forever_patches = "1.60.1"` |
| ID-packager | BigWigs packager v2.6.0 or later supports Forever; v2.7.0 is current | verified-by-source | [Packager releases](https://github.com/BigWigsMods/packager/releases) (v2.7.0, 2026-11-05: "Don't allow Forever interface versions in `Classic` toc files", "Allow Forever interface versions in `Mainline` toc files"); `release.sh` maps interface `16???` and suffix `_Camelot` to game type `forever`, uploads to CurseForge (game id 88568) and Wago (`supported_forever_patches`), and skips WoWInterface, which has no Forever category |
| ID-camelot | "Camelot" is Forever's internal name: game type `camelot`, constant `WOW_PROJECT_CAMELOT`, TOC suffix `_Camelot` | verified-by-source | Forever changelog quoted on the Warcraft Wiki TOC format page ("The Camelot value may be changed to Forever in the future"); HereBeDragons detects Forever with `WOW_PROJECT_ID == WOW_PROJECT_CAMELOT`; client load screens live under `Camelot160\Main\` |

### Further identity findings

These were not on the original question list but change design decisions, so they are tracked
the same way.

| ID | Item | Status | Evidence |
|---|---|---|---|
| ID-client-family | Forever is in the **Mainline** (modern client) family, not the Classic family | verified-by-source | Warcraft Wiki TOC format: the `_Mainline` family suffix covers "standard, forever, camelot", while `_Classic` covers vanilla to mists; packager v2.7.0 allows Forever interfaces in `_Mainline` TOCs and rejects them in `_Classic` ones; Auctionator runs its modern auction house code on Forever and treats `WOW_PROJECT_CAMELOT` like Mainline. Consequence: Classic Era APIs are not a safe prior; modern (`C_*`) APIs may be the primary ones |
| ID-project-id | Value of `WOW_PROJECT_ID` on Forever | unconfirmed | Third-party notes say `WOW_PROJECT_CAMELOT` (18); TomTom instead detects Forever by interface range 16001 to 19999. fallback: detect Forever by `select(4, GetBuildInfo())` in 16000 to 19999, never by project ID alone (Compat.IsForever) |
| ID-surname | Characters have a surname, and `UnitName` returns it where other flavors return the realm | verified-by-source | Leatrix Plus Forever code: "Forever uses surnames where other games use realm names" and `local surname = isForever and realm or nil` after `UnitName(unit)` |
| ID-wtf-layout | Per-character folders are `WTF/Account/<account>/<number>/<FirstName>-<Surname>/` | verified-by-source | A live install's WTF tree: the realm level is a number (for example `93`) and character folders read like `Firstname-Surname`. Consequence: discovery cannot assume `<realm name>/<character>`, and a character's identity is first name plus surname |

## Source for API facts

API rows marked `verified-by-source` cite the client's own interface code for build
1.60.1.70790: the generated API documentation (`Blizzard_APIDocumentationGenerated`) and the
Blizzard UI files the client loads for its game type, as mirrored in the `forever` branch of
[wow-ui-source](https://github.com/Gethe/wow-ui-source/tree/forever). Forever loads the
`mainline` family variants and its own `camelot` variants of the UI, not the Vanilla ones.

## Addon restrictions ("secret values")

Forever inherits the modern client's addon restrictions. Some APIs return *secret values*:
addon code can pass them back to the client but cannot read, compare or print them.

- **Combat log:** the function that reads a combat log event exists only in `C_CombatLogSecure`,
  whose environment is `SecureOnly`. The client's deprecation shims say these functions "have been
  relocated to the secure environment, for which no deprecation is (intentionally) provided".
  `COMBAT_LOG_EVENT_UNFILTERED` still fires, but addons cannot read its payload.
- **Unit identity:** the client documents values as secret "when the unit isn't player-controlled
  or in the party/raid". ~~Read literally, any NPC's GUID and name are secret to addons.~~
  **Correction (S-06, in game):** in the open world, in and out of combat, `UnitGUID` and
  `UnitName` on NPCs, and the GUIDs in `PARTY_KILL`, are readable (`issecretvalue` is false). The
  rule applies only while addon restrictions are in effect, which the client ties to dungeons,
  raids, encounters, challenge modes and PvP. Code must still check `issecretvalue` before using
  these values.
- **Chat messaging lockdown:** chat event text is secret "when encounter, challenge mode, or PvP
  match addon restrictions are in effect, and when the player is on a communication-restricted map
  such as a dungeon or raid". Elsewhere it is readable (S-06: XP gain text was readable).

## Collector APIs

| ID | Item | Status | Evidence |
|---|---|---|---|
| API-quest-detail | `QUEST_DETAIL(questStartItemID)`; while the detail frame is open, `GetTitleText()` and `GetQuestText()` return the title and description, `GetQuestID()` the quest | verified-by-source | API docs (QuestLog); the mainline `QuestInfo.lua` calls `GetTitleText()` and `GetQuestText()` for the detail panel |
| API-quest-accepted | `QUEST_ACCEPTED(questId)`: one argument, no quest log index (unlike Classic Era) | verified-by-source | API docs (QuestLog) |
| API-quest-turnin | `QUEST_TURNED_IN(questID, xpReward, moneyReward)` | verified-by-source | API docs (QuestLog) |
| API-questlog-fallback | Quest titles by ID via `C_QuestLog.GetTitleForQuestID(questID)`; quest log entries via `C_QuestLog.GetInfo` and `C_QuestLog.GetQuestIDForLogIndex`. The legacy `GetQuestLogQuestText` is only called from Classic-family UI code | verified-by-source | API docs (QuestLog); UI call sites |
| API-fishing-loot | `IsFishingLoot()` while a loot window is open; `LOOT_OPENED(autoLoot, isFromItem)` and `LOOT_READY(autoloot)` | verified-by-source | API docs (Loot); the mainline `LootFrame.lua` calls `IsFishingLoot()`. Whether auto-loot leaves a window to read is an in-game check |
| API-loot-slot | `GetNumLootItems()`, `GetLootSlotLink(slot)` | verified-by-source | The mainline `LootFrame.lua` calls both |
| API-cleu-fields | Combat log event payloads are **not readable by addons** | verified-by-source | API docs: `GetCurrentEventInfo` exists only in `C_CombatLogSecure` (`Environment = "SecureOnly"`); see Addon restrictions |
| API-party-kill | `PARTY_KILL(attackerGUID, targetGUID)` fires once per kill by the player or a party member; in the open world both GUIDs are readable | verified-in-game | S-06: `PARTY_KILL Player-... Creature-0-...-2958-...` with `issecretvalue` false. API docs (Unit) mark it `SecretWhenUnitIdentityRestricted`, so in restricted areas the target GUID can be secret |
| API-unit-died | `UNIT_DIED(unitGUID)` exists as a standalone event; same secrecy as `PARTY_KILL` | verified-by-source | API docs (Unit) |
| API-death-recap | `C_DeathRecap.GetRecapEvents()` after a death returns the last damage events with `event`, `sourceName`, `sourceGUID`, `environmentalType`, `amount`, `overkill`, `currentHP`, `timestamp`; the killer is the newest event | verified-in-game | S-06: after drowning, events 1 to 7 were `ENVIRONMENTAL_DAMAGE` / `Drowning` and earlier ones `SWING_DAMAGE` from `sourceName="Prairie Wolf"` with its GUID. API docs (DeathRecap): environment `All`, no secrecy predicate |
| API-classification | `UnitClassification(unit)` returns e.g. `"normal"`; no secrecy predicate | verified-in-game | S-06: `"normal"` for a wolf and a named NPC; API docs (Unit) |
| API-item-info-async | `GET_ITEM_INFO_RECEIVED(itemID, success)` and `ITEM_DATA_LOAD_RESULT(itemID, success)`; `C_Item.RequestLoadItemDataByID`, `C_Item.GetItemQualityByID` | verified-by-source | API docs (Item) |
| API-zone-events | `ZONE_CHANGED`, `ZONE_CHANGED_INDOORS`, `ZONE_CHANGED_NEW_AREA`, no payload | verified-by-source | API docs (Map) |
| API-zone-text | `GetZoneText()`, `GetSubZoneText()`, `GetRealZoneText()`, `GetMinimapZoneText()` | verified-by-source | API docs (ZoneScript) |
| API-map-id | `C_Map.GetBestMapForUnit(unit)`, `C_Map.GetMapInfo(mapID)` | verified-by-source | API docs (Map) |
| API-group-roster | `GROUP_ROSTER_UPDATE`; `GetNumGroupMembers()`, `IsInGroup()` | verified-by-source | API docs (PartyInfo); widely called by the client UI. Party members are player-controlled, so their names are not secret |
| API-screenshot | `Screenshot()` is a plain global with no restriction or secrecy flags | verified-by-source | API docs (Client) |
| API-screenshot-events | `SCREENSHOT_STARTED`, `SCREENSHOT_SUCCEEDED`, `SCREENSHOT_FAILED` | verified-by-source | API docs (Video) |
| API-locale | `GetLocale()` | verified-by-source | API docs (Locale) |
| API-settings-panel | `Settings.RegisterCanvasLayoutCategory` and `Settings.RegisterAddOnCategory`; `InterfaceOptions_AddCategory` is not present | verified-by-source | `Blizzard_Settings_Shared` in the client UI code; no call site or definition of `InterfaceOptions_AddCategory` |
| API-damage-subevents | Damage sub-events (`SWING_DAMAGE`, `SPELL_DAMAGE`, `RANGE_DAMAGE`, `SPELL_PERIODIC_DAMAGE`) are combat log payloads, so they are **not readable by addons** | verified-by-source | See API-cleu-fields |
| API-environmental | Environmental damage is a combat log payload addons cannot read live, but the death recap reports it after a death: `event="ENVIRONMENTAL_DAMAGE"` with `environmentalType` (e.g. `"Drowning"`) | verified-in-game | S-06: death recap events after drowning carry `environmentalType="Drowning"`. Live combat log: see API-cleu-fields |
| API-affiliation-flags | `Enum.CombatLogObject` affiliation constants still exist, but they only describe combat log payloads, which addons cannot read; not usable | verified-by-source | The client's combat log deprecation shims map `COMBATLOG_OBJECT_AFFILIATION_*` to `Enum.CombatLogObject` |
| API-guid-npcid | NPC GUIDs read `Creature-0-<server>-<instance>-<zone>-<npcID>-<spawn>`; the NPC ID is the sixth dash-separated field. Readable in the open world, possibly secret in restricted areas | verified-in-game | S-06: Prairie Wolf GUID `Creature-0-6867-1-2-2958-...` (NPC ID 2958), `issecretvalue` false in and out of combat |
| API-chat-loot | `CHAT_MSG_LOOT(text, playerName, ...)`; `text` is secret during chat messaging lockdown (dungeons, raids, encounters, PvP) and readable elsewhere. The `LOOT_ITEM_SELF*` global strings used to parse it are not referenced by the client UI code, so their presence and format are unchecked | unconfirmed | API docs (ChatInfo). fallback: notable loot is read from the loot window (`GetLootSlotLink` on `LOOT_OPENED`), which needs no string parsing; chat parsing only adds items pushed straight to the bags (Compat.ParseLootMessage returns nil when the pattern strings are missing) |
| API-discovery | Zone discovery messages (`ERR_ZONE_EXPLORED*` global strings via `UI_INFO_MESSAGE(errorType, message)` or `CHAT_MSG_SYSTEM`) are not referenced by the client UI code | unconfirmed | fallback: a discovery is the first time this character enters a subzone, tracked by the addon itself (Compat.IsDiscoveryMessage returns false when the strings are missing) |
| API-screenshot-files | Screenshot folder and file name format on Forever | unconfirmed | No `Screenshots/` folder exists until a first screenshot is taken. fallback: the app matches screenshot files to events by file modification time, not by name (Compat.TakeScreenshot only reports success via `SCREENSHOT_SUCCEEDED`) |
| API-bit | The `bit` library (`bit.band`, `bit.bor`) is available | verified-by-source | Called from many client UI files |
| API-date-time | Lua `date()` and `time()` are available; also `C_DateAndTime.GetCurrentCalendarTime`, `GetServerTimeLocal` | verified-by-source | Called from client UI files; API docs (DateAndTime) |
| API-timer | `C_Timer.After`, `C_Timer.NewTicker`, `C_Timer.NewTimer` | verified-by-source | API docs (Timer) |
| DATA-junk-boots | Item IDs of junk boots and other junk fishing catches on Forever | unconfirmed | fallback: a fishing catch of poor quality (quality 0) counts as junk, and a junk catch whose name matches the localized boot list counts as a boot (Compat.IsBootCatch) |
| DATA-murloc-npcs | Murlocs are identified by NPC ID from the GUID (readable in the open world); a sourced list of murloc NPC IDs for Forever is still needed | unconfirmed | NPC IDs are readable (S-06). fallback: until the ID list is sourced, and where the GUID is secret, murlocs are recognised by name from a per-locale list (Compat.IsMurlocName) |

## Compat interface (draft)

The addon calls the client only through this module for anything that is unconfirmed, restricted
or version-sensitive, so the rest of the code never branches on client details.

```lua
-- Client identity
function Compat.IsForever()              -- interface in 16000..19999; never relies on WOW_PROJECT_ID
function Compat.IsSecret(value)          -- issecretvalue(value) where it exists, else false

-- Quests (QUEST_DETAIL frame is open)
function Compat.GetQuestDetailText()     -- questID, title, description

-- Fishing and loot (loot window is open)
function Compat.IsFishingLoot()          -- false if the API is missing
function Compat.IsBootCatch(itemLink)    -- poor quality and name in the localized boot list
function Compat.ParseLootMessage(text)   -- itemLink, quantity; nil if secret or patterns missing

-- Zones
function Compat.IsDiscoveryMessage(text) -- false if the discovery strings are missing

-- Creatures
function Compat.IsMurlocName(name)       -- per-locale name list; false for secret names

-- Screenshots
function Compat.TakeScreenshot()         -- calls Screenshot(); success arrives via SCREENSHOT_SUCCEEDED

-- Settings
function Compat.RegisterSettingsPanel(frame, name) -- Settings.RegisterCanvasLayoutCategory + RegisterAddOnCategory
```
