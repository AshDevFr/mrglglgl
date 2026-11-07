# WoW: Forever platform notes

What the Mrglglgl addon and app rely on about World of Warcraft: Forever, and how each fact was
established. Forever launched on 2026-11-04; much of the public documentation predates it, so
nothing here is assumed from other WoW flavors without saying so.

**Client build:** 1.60.1.70790 (verified in game, 2026-11-05)

## Status values

| Status | Meaning |
|---|---|
| `verified-in-game` | Observed on a live Forever client; the evidence cites the check (`S-NN`) that showed it |
| `verified-by-source` | Stated by a source that covers Forever specifically (Blizzard data, the client's own files, Forever-aware tooling or addon code) |
| `unconfirmed` | Not established for Forever; the evidence names the fallback the addon uses |

## Client identity and distribution

| ID | Item | Status | Evidence |
|---|---|---|---|
| ID-interface | `## Interface:` value is `16001` | verified-in-game | S-01: `GetBuildInfo()` returns interface `16001`. Also: Warcraft Wiki [TOC format](https://warcraft.wiki.gg/wiki/TOC_format); installed Forever addons; the BigWigs packager maps `16???` to `forever` |
| ID-toc-suffix | The client reads `<Addon>_Forever.toc` and `<Addon>_Camelot.toc` | verified-by-source | Warcraft Wiki TOC format: "Forever: `AddOnName_Forever.toc` or `AddOnName_Camelot.toc`", and the Forever changelog line "Added Camelot game type, Game variable value and TOC suffix, with Forever game type and TOC suffix alias". Installed Questie ships both. Note: the BigWigs packager recognises only `_Camelot` as a Forever suffix (it does not know `_Forever`); Wago accepts both |
| ID-toc-bare | With no matching suffixed TOC, the client loads the bare `<Addon>.toc` | verified-in-game | S-11: the Mrglglgl addon, shipped with only `Mrglglgl.toc` (`## Interface: 16001`), loaded in game and wrote its SavedVariables. Also Warcraft Wiki TOC format: the game "will default to `AddOnName.toc` without a suffix" when no suffixed file matches |
| ID-toc-multi-interface | `## Interface:` accepts a comma-separated list | verified-by-source | Warcraft Wiki TOC format (supported since patch 10.2.7); installed Forever-compatible addons (Altoholic, Auctionator, Bartender4) ship a single bare TOC listing `120105, 50504, 20506, 11509, 16001` |
| ID-folder | Install subfolder is `_forever_` | verified-in-game | S-10: the running client wrote its screenshot to `World of Warcraft/_forever_/Screenshots/`; the folder also holds the client, `Interface/` and `WTF/`, and its `.flavor.info` names product `wowf` |
| ID-build | Client version 1.60.1, build 70790, built Nov 3 2026 | verified-in-game | S-01: `GetBuildInfo()` returns `"1.60.1", "70790", "Nov  3 2026", 16001`; matches the install metadata `.build.info` |
| ID-curseforge-flavor | CurseForge flavor "Forever", game version "1.60.1" | verified-by-source | CurseForge file listings label Forever files with flavor "Forever" and game version "1.60.1" (e.g. [WoW Forever Suite files](https://www.curseforge.com/wow/addons/wow-forever-suite/files/all)); the packager uploads them with CurseForge game id 88568 |
| ID-wago-flavor | Wago flavor key `forever`, patch `1.60.1` | verified-by-source | [Wago game data API](https://addons.wago.io/api/data/game): `patches.forever = ["1.60.1"]`, `toc_suffixes.forever = ["-Camelot","_Camelot","-Forever","_Forever"]`, `supported_forever_patches = "1.60.1"` |
| ID-packager | BigWigs packager v2.6.0 or later supports Forever; v2.7.0 is current | verified-by-source | [Packager releases](https://github.com/BigWigsMods/packager/releases) (v2.7.0, 2026-11-05: "Don't allow Forever interface versions in `Classic` toc files", "Allow Forever interface versions in `Mainline` toc files"); `release.sh` maps interface `16???` and suffix `_Camelot` to game type `forever`, uploads to CurseForge (game id 88568) and Wago (`supported_forever_patches`), and skips WoWInterface, which has no Forever category |
| ID-camelot | "Camelot" is Forever's internal name: game type `camelot`, `WOW_PROJECT_ID == WOW_PROJECT_CAMELOT == 18`, TOC suffix `_Camelot` | verified-in-game | S-02: `WOW_PROJECT_ID` = 18 = `WOW_PROJECT_CAMELOT` (`WOW_PROJECT_MAINLINE` = 1, `WOW_PROJECT_CLASSIC` = 2). Also the Forever changelog quoted on the Warcraft Wiki TOC format page |

### Further identity findings

These were not on the original question list but change design decisions, so they are tracked
the same way.

| ID | Item | Status | Evidence |
|---|---|---|---|
| ID-client-family | Forever is in the **Mainline** (modern client) family, not the Classic family | verified-by-source | Warcraft Wiki TOC format: the `_Mainline` family suffix covers "standard, forever, camelot", while `_Classic` covers vanilla to mists; packager v2.7.0 allows Forever interfaces in `_Mainline` TOCs and rejects them in `_Classic` ones; Auctionator runs its modern auction house code on Forever and treats `WOW_PROJECT_CAMELOT` like Mainline. Consequence: Classic Era APIs are not a safe prior; modern (`C_*`) APIs may be the primary ones |
| ID-project-id | `WOW_PROJECT_ID` is 18 (`WOW_PROJECT_CAMELOT`) on Forever | verified-in-game | S-02. The addon still detects Forever by interface range as well (Compat.IsForever), since the constant name may change to Forever later |
| ID-surname | Characters have a surname. `UnitFullName("player")` returns first name and surname where other flavors return name and realm; `GetRealmName()` still returns the realm, and `GetRealmID()` matches the server part of player GUIDs | verified-in-game | S-05: `UnitFullName("player")` returned the first name and the surname; `GetRealmName()` returned a realm name of the form `CAMWOW EU4 Realm 1 PvE`; `GetRealmID()` returned 6841, the same number as in the player GUID `Player-6841-...`. Also Leatrix Plus Forever code |
| ID-wtf-layout | Per-character folders are `WTF/Account/<account>/<number>/<FirstName>-<Surname>/`. The number is **not** `GetRealmID()`, so its meaning is unknown and the app must not interpret it; the addon records realm name and surname itself | verified-by-source | A live install's WTF tree has a number (for example `93`) at the realm level, while S-05 showed `GetRealmID()` = 6841 for a character stored there. Character folders read like `Firstname-Surname` |

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
| API-fishing-loot | `LOOT_OPENED(autoLoot, isFromItem)` fires with auto-loot on and off, and `IsFishingLoot()` is true for fishing in both cases | verified-in-game | S-08: `LOOT_OPENED true` / `LOOT_OPENED false` (auto-loot on / off), `IsFishingLoot()` true each time. API docs (Loot); mainline `LootFrame.lua` |
| API-loot-slot | `GetNumLootItems()`, `GetLootSlotLink(slot)` return the catch even with auto-loot on | verified-in-game | S-08: slot 1 listed `[Raw Brilliant Smallfish]` and `[Raw Longjaw Mud Snapper]` with auto-loot on and off |
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
| API-screenshot | `Screenshot()` is callable by addon code | verified-in-game | S-10: `/run Screenshot()` produced a file. API docs (Client): no restriction or secrecy flags |
| API-screenshot-events | `SCREENSHOT_STARTED`, `SCREENSHOT_SUCCEEDED`, `SCREENSHOT_FAILED` | verified-by-source | API docs (Video) |
| API-locale | `GetLocale()` | verified-by-source | API docs (Locale) |
| API-settings-subcategory | `Settings.RegisterVerticalLayoutSubcategory(parent, name)` adds a page under an addon's entry in Options > AddOns | verified-in-game | S-14: the Mrglglgl entry showed its Chat, Screenshots and Forget pages (2026-11-06). Also `Blizzard_Settings_Shared` in the client UI code. Compat.CreateSettingsSubcategory keeps the one-page fallback |
| API-settings-panel | `Settings.RegisterCanvasLayoutCategory` and `Settings.RegisterAddOnCategory`; `InterfaceOptions_AddCategory` is not present | verified-by-source | `Blizzard_Settings_Shared` in the client UI code; no call site or definition of `InterfaceOptions_AddCategory` |
| API-damage-subevents | Damage sub-events (`SWING_DAMAGE`, `SPELL_DAMAGE`, `RANGE_DAMAGE`, `SPELL_PERIODIC_DAMAGE`) are combat log payloads, so they are **not readable by addons** | verified-by-source | See API-cleu-fields |
| API-environmental | Environmental damage is a combat log payload addons cannot read live, but the death recap reports it after a death: `event="ENVIRONMENTAL_DAMAGE"` with `environmentalType` (e.g. `"Drowning"`) | verified-in-game | S-06: death recap events after drowning carry `environmentalType="Drowning"`. Live combat log: see API-cleu-fields |
| API-affiliation-flags | `Enum.CombatLogObject` affiliation constants still exist, but they only describe combat log payloads, which addons cannot read; not usable | verified-by-source | The client's combat log deprecation shims map `COMBATLOG_OBJECT_AFFILIATION_*` to `Enum.CombatLogObject` |
| API-guid-npcid | NPC GUIDs read `Creature-0-<server>-<instance>-<zone>-<npcID>-<spawn>`; the NPC ID is the sixth dash-separated field. Readable in the open world, possibly secret in restricted areas | verified-in-game | S-06: Prairie Wolf GUID `Creature-0-6867-1-2-2958-...` (NPC ID 2958), `issecretvalue` false in and out of combat |
| API-chat-loot | `CHAT_MSG_LOOT(text, playerName, ...)`; `text` is secret during chat messaging lockdown and readable elsewhere. Global strings: `LOOT_ITEM_SELF` = `"You receive loot: %s"`, `LOOT_ITEM_SELF_MULTIPLE` = `"You receive loot: %sx%d"`, `LOOT_ITEM_PUSHED_SELF` = `"You receive item: %s"` (enUS) | verified-in-game | S-07 (strings), S-08 (loot messages readable in the open world). API docs (ChatInfo) |
| API-discovery | Discovery messages use `ERR_ZONE_EXPLORED` = `"Discovered: %s"` and `ERR_ZONE_EXPLORED_XP` = `"Discovered %s: %d experience gained"` (enUS); which event carries them (`UI_INFO_MESSAGE` or `CHAT_MSG_SYSTEM`) was not observed, so the parser accepts both | verified-in-game | S-07 (strings). The addon also records first visits to a subzone itself, so a missed message loses nothing (Compat.IsDiscoveryMessage) |
| API-screenshot-files | Screenshots are saved as `_forever_/Screenshots/WoWScrnShot_MMDDYY_HHMMSS.jpg`, in local time | verified-in-game | S-10: `WoWScrnShot_110526_202955.jpg`, taken 2026-11-05 at 20:29:55 local time |
| API-bit | The `bit` library (`bit.band`, `bit.bor`) is available | verified-by-source | Called from many client UI files |
| API-date-time | Lua `date()` and `time()` are available; also `C_DateAndTime.GetCurrentCalendarTime`, `GetServerTimeLocal` | verified-by-source | Called from client UI files; API docs (DateAndTime) |
| API-timer | `C_Timer.After`, `C_Timer.NewTicker`, `C_Timer.NewTimer` | verified-by-source | API docs (Timer) |
| DATA-junk-boots | Item IDs of junk boots and other junk fishing catches on Forever | unconfirmed | fallback: a fishing catch of poor quality (quality 0) counts as junk, and a junk catch whose name matches the localized boot list counts as a boot (Compat.IsBootCatch) |
| DATA-murloc-npcs | Murlocs are identified by NPC ID from the GUID (readable in the open world); a sourced list of murloc NPC IDs for Forever is still needed | unconfirmed | NPC IDs are readable (S-06). fallback: until the ID list is sourced, and where the GUID is secret, murlocs are recognised by name from a per-locale list (Compat.IsMurlocName) |

## Compat interface

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
function Compat.ParseCreateMessage(text) -- "You create" message: itemLink, quantity; nil otherwise

-- Zones
function Compat.IsDiscoveryMessage(text) -- false if the discovery strings are missing

-- Creatures and professions
function Compat.IsMurlocName(name)       -- per-locale name list; false for secret names
function Compat.IsGatheringSpell(name)   -- mining, herb gathering, skinning, by localized name

-- Screenshots
function Compat.TakeScreenshot()         -- calls Screenshot(); success arrives via SCREENSHOT_SUCCEEDED

-- Creatures and items
function Compat.NpcIdFromGuid(guid)      -- sixth field of a creature GUID; nil for players and secrets
function Compat.GetItemNameAndQuality(link) -- nil until the item is cached (GET_ITEM_INFO_RECEIVED)
function Compat.ItemIdFromLink(link)
function Compat.InChatLockdown()         -- C_ChatInfo.InChatMessagingLockdown

-- Settings panel, on the client's vertical settings layout (Blizzard_Settings_Shared)
function Compat.CreateSettingsCategory(name)  -- Settings.RegisterVerticalLayoutCategory
function Compat.CreateSettingsSubcategory(parent, parentLayout, name)  -- a sub-page, or the parent
function Compat.AddSettingsHeader(layout, name)
function Compat.AddCheckbox(category, variable, name, tooltip, get, set, default)
function Compat.AddDropdown(category, variable, name, tooltip, choices, get, set, default)
function Compat.AddButton(layout, name, buttonText, onClick, tooltip)
function Compat.RegisterSettingsCategory(category)
function Compat.OpenSettings(category)
```

## Achievements and Legacy Challenges

| ID | Item | Status | Evidence |
|---|---|---|---|
| API-achievements | The achievement API is present: `GetAchievementInfo`, `C_AchievementInfo`, `GetCategoryList`, `GetCategoryInfo`, `GetCategoryNumAchievements`, `GetNumCompletedAchievements` | verified-in-game | S-13: `/dump GetAchievementInfo, C_AchievementInfo` returned the function and the table; `GetNumCompletedAchievements()` = 111, 1; `#GetCategoryList()` = 29 |
| API-legacy-are-achievements | Legacy Challenges are achievements worth 0 points, in categories: Classes (one per class), Tradeskills (one per profession), Dungeons, Raids, Player vs. Player (Ranks, Reputations, Season Journey), Adventure (Explorer, Eastern Kingdoms, Kalimdor), Tier 1 Gear. IDs are Forever's own (Wrath's ID 6 does not exist) | verified-in-game | S-13: the one completed achievement is `855 Explore Moonglade 0` (0 points) in category 14778 "Kalimdor" (parent 15596 "Adventure"); `LegacyChallengesUnviewed` = `{ [855] = 14778 }`; `GetTotalAchievementPoints()` = 0; `GetAchievementInfo(6)` is empty. There is no `C_Legacy`/`C_LegacySystem`; the Legacy window is UI code (`LegacySystemFrame_LoadUI`, `ToggleLegacyChallenges`, `ShowLegacyChallenge`) |
| API-achievement-earned | `ACHIEVEMENT_EARNED(achievementID, alreadyEarned)` fires when one is completed | unconfirmed | API docs (Achievement). fallback: registered only when `C_EventUtils.IsEventValid` says the client knows it (Compat.EventExists), so a missing event leaves the addon loading; whether it fires for a Legacy Challenge is the next in-game check |

## Growing up (spells, collections, reputation, gear, travel, duels)

| ID | Item | Status | Evidence |
|---|---|---|---|
| API-progress-events | `LEARNED_SPELL_IN_SKILL_LINE`, `NEW_MOUNT_ADDED`, `NEW_PET_ADDED` and `ACHIEVEMENT_EARNED` exist; `LEARNED_SPELL_IN_TAB` does not | verified-in-game | S-15: `C_EventUtils.IsEventValid` returned true, false (`LEARNED_SPELL_IN_TAB`), true, true, true. That these events fire with the expected arguments is still to be seen |
| API-progress-functions | `C_GameRules.IsHardcoreActive`, `C_Reputation.GetNumFactions`, `C_Item.GetDetailedItemLevelInfo`, `GetBindLocation` exist; `GetFactionInfo` does not (the C_Reputation path is used) | verified-in-game | S-15: the first three dumped as functions, `GetFactionInfo` as nil, `GetBindLocation()` returned "Thunder Bluff" |
| API-hardcore | `C_GameRules.IsHardcoreActive()` answers on Forever; it returns false on a normal realm | verified-in-game | S-16: `/dump C_GameRules.IsHardcoreActive()` returned false on the owner's realm. A hardcore realm's true is still to be seen |
| API-progress-strings | `ERR_NEWTAXIPATH` = "New flight path discovered!" and `DUEL_WINNER_KNOCKOUT` = "%1$s has defeated %2$s in a duel" (numbered placeholders) | verified-in-game | S-15 (enUS) |
| API-item-link-colour | Item links colour by named quality, `\|cnIQ1:`, not `\|cffRRGGBB` | verified-in-game | S-15: `GetInventoryItemLink("player", 16)` returned `\|cnIQ1:\|Hitem:767::::::::10:1484:::::::::\|h[Long Bo Staff]\|h\|r`. Chat.plain strips both forms |

## Professions

| ID | Item | Status | Evidence |
|---|---|---|---|
| API-skill-lines | `C_SkillInfo.GetNumSkillLines()` and `C_SkillInfo.GetSkillLineInfo(index)` return `{ skillID, name, isHeader, rank, maxRank, ... }`; `SKILL_LINES_CHANGED` signals a change | verified-by-source | API docs (SkillInfo, environment All); Forever's own `Camelot/SkillsFrame.lua` reads them |
| API-crafted-result | `TRADE_SKILL_ITEM_CRAFTED_RESULT(data)` with `data.itemID`, `data.quantity`, `data.hyperlink` | verified-by-source | API docs (TradeSkillUI, `CraftingItemResultData`); used by the client's professions output log. Whether it fires from Forever's professions window is an in-game check (S-12) |
| API-spellcast-succeeded | `UNIT_SPELLCAST_SUCCEEDED(unitTarget, castGUID, spellID)`; `C_Spell.GetSpellName(spellID)` | verified-by-source | API docs (Unit, Spell). Gathering spell names as the client reports them are an in-game check (S-12) |
