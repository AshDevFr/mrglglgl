-- Every client call that is restricted, unconfirmed or version-sensitive on WoW: Forever goes
-- through here, so the rest of the addon never branches on client details. docs/forever-api.md
-- records the evidence for each function's assumptions and fallbacks.
local _, ns = ...

local Compat = {}
ns.Compat = Compat

local L = ns.L

-- Forever reports interface 16xxx. WOW_PROJECT_ID is WOW_PROJECT_CAMELOT today, but that name may
-- change, so the interface range is the check.
-- The addon's own version as packaged (the release tag). Classic clients moved this into
-- C_AddOns; older ones have the global. Unverified on Forever (in-game check pending).
function Compat.AddonVersion()
	local get = (C_AddOns and C_AddOns.GetAddOnMetadata) or GetAddOnMetadata
	if not get then
		return nil
	end
	return get(ns.name, "Version")
end

function Compat.IsForever()
	local interface = select(4, GetBuildInfo())
	return type(interface) == "number" and interface >= 16000 and interface < 20000
end

-- Values the client withholds from addons (in dungeons, raids, encounters and PvP) are secret:
-- they can be passed back to the client but not read, compared or stored.
function Compat.IsSecret(value)
	return issecretvalue ~= nil and issecretvalue(value) == true
end

-- Creature GUIDs read "Creature-0-<server>-<instance>-<zone>-<npcID>-<spawn>". Returns nil for
-- players, secret GUIDs and anything else that is not a creature.
function Compat.NpcIdFromGuid(guid)
	if guid == nil or Compat.IsSecret(guid) then
		return nil
	end
	local kind, npcId = guid:match("^(%a+)%-0%-%d+%-%d+%-%d+%-(%d+)%-")
	if kind ~= "Creature" and kind ~= "Vehicle" then
		return nil
	end
	return tonumber(npcId)
end

-- Only valid while the quest detail frame is open (QUEST_DETAIL). Reading the open frame changes
-- no quest log selection.
function Compat.GetQuestDetailText()
	return GetQuestID(), GetTitleText(), GetQuestText()
end

-- Tag IDs the quest log uses for the kinds of quest players mark in brackets ([9D], [9R], [9G]).
local QUEST_TAGS = { [1] = "group", [62] = "raid", [81] = "dungeon" }

-- The level and kind of a quest in the player's quest log: level, then "elite", "dungeon", "raid",
-- "group" or nil. Read while the quest is in the log (at QUEST_ACCEPTED); reading changes no
-- selection. Every call is checked first: none of them is confirmed on Forever yet, and a missing
-- one gives nothing rather than an error.
function Compat.GetQuestLevelAndKind(questId)
	local log = C_QuestLog
	if not log then
		return nil, nil
	end
	local level
	local index = log.GetLogIndexForQuestID and log.GetLogIndexForQuestID(questId)
	if index and log.GetInfo then
		local info = log.GetInfo(index)
		level = info and tonumber(info.level) or nil
		if level and level <= 0 then
			level = nil
		end
	end
	local kind
	local tag = log.GetQuestTagInfo and log.GetQuestTagInfo(questId)
	if tag then
		kind = QUEST_TAGS[tag.tagID]
		if kind == nil and (tag.isElite or tag.tagName == "Elite") then
			kind = "elite"
		end
	end
	return level, kind
end

function Compat.IsFishingLoot()
	return IsFishingLoot ~= nil and IsFishingLoot() == true
end

local GetItemInfo = (C_Item and C_Item.GetItemInfo) or GetItemInfo

-- Returns name, quality for an item link, or nil while the item is not cached yet; the client then
-- sends GET_ITEM_INFO_RECEIVED with the item ID.
function Compat.GetItemNameAndQuality(itemLink)
	local name, _, quality = GetItemInfo(itemLink)
	if name == nil then
		return nil
	end
	return name, quality
end

function Compat.ItemIdFromLink(itemLink)
	return tonumber(itemLink:match("item:(%d+)"))
end

-- True while the client withholds chat text from addons (dungeons, raids, encounters, PvP).
function Compat.InChatLockdown()
	return C_ChatInfo ~= nil and C_ChatInfo.InChatMessagingLockdown ~= nil
		and C_ChatInfo.InChatMessagingLockdown() == true
end

local function hints(key)
	local list = {}
	for hint in L[key]:gmatch("[^|]+") do
		list[#list + 1] = hint
	end
	return list
end

local function nameMatches(name, key)
	local lower = name:lower()
	for _, hint in ipairs(hints(key)) do
		if lower:find(hint, 1, true) then
			return true
		end
	end
	return false
end

-- True for a junk boot, false for anything else, nil while the item is not cached yet (the caller
-- retries after GET_ITEM_INFO_RECEIVED). Junk item IDs on Forever are unconfirmed, so a known ID
-- list wins and the localized name is the fallback.
function Compat.IsBootCatch(itemLink)
	local itemId = tonumber(itemLink:match("item:(%d+)"))
	local known = ns.Data and ns.Data.junkBoots
	if known and itemId and known[itemId] then
		return true
	end
	local name, _, quality = GetItemInfo(itemLink)
	if name == nil then
		return nil
	end
	return quality == 0 and nameMatches(name, "BOOT_NAME_HINTS")
end

-- Turns a client format string such as "You receive loot: %sx%d" into an anchored Lua pattern
-- capturing each %s and %d.
local function patternFromGlobalString(format)
	local escaped = format:gsub("[%^%$%(%)%.%[%]%*%+%-%?]", "%%%0")
	return "^" .. escaped:gsub("%%s", "(.-)"):gsub("%%d", "(%%d+)") .. "$"
end

local lootPatterns
local function getLootPatterns()
	if lootPatterns == nil then
		lootPatterns = {}
		-- The multiple-item forms first: the single-item pattern would also match them. Looked up by
		-- name because any of them may be missing on a client, and a nil would end an ipairs list.
		for _, name in ipairs({ "LOOT_ITEM_SELF_MULTIPLE", "LOOT_ITEM_PUSHED_SELF_MULTIPLE", "LOOT_ITEM_SELF",
			"LOOT_ITEM_PUSHED_SELF" }) do
			local format = _G[name]
			if type(format) == "string" then
				lootPatterns[#lootPatterns + 1] = patternFromGlobalString(format)
			end
		end
	end
	return lootPatterns
end

-- Returns itemLink, quantity for a loot message about the player, or nil when the text is secret,
-- is about someone else, or the format strings are missing.
function Compat.ParseLootMessage(text)
	if text == nil or Compat.IsSecret(text) then
		return nil
	end
	for _, pattern in ipairs(getLootPatterns()) do
		local link, count = text:match(pattern)
		if link then
			return link, tonumber(count) or 1
		end
	end
	return nil
end

-- Returns itemLink, quantity for a "You create" message about the player, or nil.
-- The player's money in copper, or nil if the client does not give it.
function Compat.GetMoney()
	local get = _G.GetMoney
	return get and get() or nil
end

-- Runs after a client function each time it is called, when that function exists: repairs and
-- taking money from mail are actions, not events.
function Compat.After(name, callback)
	if type(_G[name]) == "function" and _G.hooksecurefunc then
		_G.hooksecurefunc(name, callback)
		return true
	end
	return false
end

-- Whether this client knows an event. Registering one it does not know is an error on clients that
-- check, so an event not yet seen on Forever is registered only when the client says it exists.
function Compat.EventExists(event)
	local utils = _G.C_EventUtils
	if utils and type(utils.IsEventValid) == "function" then
		return utils.IsEventValid(event) == true
	end
	return true
end

-- An achievement's name, points and category name. On Forever the Legacy Challenges are
-- achievements worth 0 points, sorted into categories such as "Kalimdor" or "Druid".
function Compat.GetAchievement(id)
	if type(_G.GetAchievementInfo) ~= "function" then
		return nil
	end
	local _, name, points = _G.GetAchievementInfo(id)
	if type(name) ~= "string" then
		return nil
	end
	local category
	if type(_G.GetAchievementCategory) == "function" and type(_G.GetCategoryInfo) == "function" then
		local categoryId = _G.GetAchievementCategory(id)
		category = categoryId and _G.GetCategoryInfo(categoryId)
	end
	return { name = name, points = points, category = type(category) == "string" and category or nil }
end

-- The auction house's chat messages, by what happened. Looked up by name: a message missing on
-- this client is skipped.
local AUCTION_MESSAGES = {
	{ "ERR_AUCTION_SOLD_S", "sold" },
	{ "ERR_AUCTION_WON_S", "won" },
	{ "ERR_AUCTION_OUTBID_S", "outbid" },
	{ "ERR_AUCTION_EXPIRED_S", "expired" },
	{ "ERR_AUCTION_REMOVED_S", "cancelled" },
}

-- Returns what happened at the auction house ("listed", "sold", "won", "outbid", "expired",
-- "cancelled") and the item's name, or nil for any other message.
function Compat.ParseAuctionMessage(text)
	if type(text) ~= "string" then
		return nil
	end
	if _G.ERR_AUCTION_STARTED and text == _G.ERR_AUCTION_STARTED then
		return "listed", nil
	end
	for _, entry in ipairs(AUCTION_MESSAGES) do
		local format = _G[entry[1]]
		if type(format) == "string" then
			local name = text:match(patternFromGlobalString(format))
			if name then
				return entry[2], name
			end
		end
	end
	return nil
end

-- The mail subjects the auction house sends; money taken from such a mail is auction money.
local AUCTION_MAIL_SUBJECTS = {
	"AUCTION_SOLD_MAIL_SUBJECT", "AUCTION_OUTBID_MAIL_SUBJECT", "AUCTION_REMOVED_MAIL_SUBJECT",
	"AUCTION_EXPIRED_MAIL_SUBJECT", "AUCTION_WON_MAIL_SUBJECT",
}

function Compat.IsAuctionMail(index)
	local header = _G.GetInboxHeaderInfo
	if not header then
		return false
	end
	local _, _, _, subject = header(index)
	if type(subject) ~= "string" then
		return false
	end
	for _, name in ipairs(AUCTION_MAIL_SUBJECTS) do
		local format = _G[name]
		if type(format) == "string" and subject:match(patternFromGlobalString(format)) then
			return true
		end
	end
	return false
end

function Compat.ParseCreateMessage(text)
	if text == nil or Compat.IsSecret(text) then
		return nil
	end
	for _, name in ipairs({ "LOOT_ITEM_CREATED_SELF_MULTIPLE", "LOOT_ITEM_CREATED_SELF" }) do
		local format = _G[name]
		if type(format) == "string" then
			local link, count = text:match(patternFromGlobalString(format))
			if link then
				return link, tonumber(count) or 1
			end
		end
	end
	return nil
end

-- Gathering spells are recognised by localized name: their spell IDs on Forever are unconfirmed.
function Compat.IsGatheringSpell(name)
	if name == nil or Compat.IsSecret(name) then
		return false
	end
	return nameMatches(name, "GATHER_SPELL_HINTS")
end

-- Returns the discovered area's name for a discovery message, or nil.
function Compat.IsDiscoveryMessage(text)
	if text == nil or Compat.IsSecret(text) then
		return nil
	end
	for _, name in ipairs({ "ERR_ZONE_EXPLORED_XP", "ERR_ZONE_EXPLORED" }) do
		local format = _G[name]
		if type(format) == "string" then
			local area = text:match(patternFromGlobalString(format))
			if area then
				return area
			end
		end
	end
	return nil
end

-- Murlocs are identified by NPC ID where the GUID is readable; this is the fallback by name.
function Compat.IsMurlocName(name)
	if name == nil or Compat.IsSecret(name) then
		return false
	end
	return nameMatches(name, "MURLOC_NAME_HINTS")
end

-- Success is reported later by SCREENSHOT_SUCCEEDED.
function Compat.TakeScreenshot()
	if Screenshot == nil then
		return false
	end
	Screenshot()
	return true
end

-- The settings panel uses the client's own vertical settings layout, so it looks and searches like
-- every other options page. Signatures are those of this client's Blizzard_Settings_Shared.

function Compat.CreateSettingsCategory(name)
	return Settings.RegisterVerticalLayoutCategory(name)
end

-- A page under the addon's entry in Options > AddOns. A client without sub-pages gets everything on
-- the addon's one page.
function Compat.CreateSettingsSubcategory(parent, parentLayout, name)
	if type(Settings.RegisterVerticalLayoutSubcategory) == "function" then
		return Settings.RegisterVerticalLayoutSubcategory(parent, name)
	end
	return parent, parentLayout
end

function Compat.AddSettingsHeader(layout, name)
	layout:AddInitializer(CreateSettingsListSectionHeaderInitializer(name))
end

function Compat.AddCheckbox(category, variable, name, tooltip, getValue, setValue, default)
	local setting = Settings.RegisterProxySetting(category, variable, Settings.VarType.Boolean, name, default,
		getValue, setValue)
	Settings.CreateCheckbox(category, setting, tooltip)
	return setting
end

-- choices: list of { value, label }.
function Compat.AddDropdown(category, variable, name, tooltip, choices, getValue, setValue, default)
	local setting = Settings.RegisterProxySetting(category, variable, Settings.VarType.Number, name, default,
		getValue, setValue)
	local function options()
		local container = Settings.CreateControlTextContainer()
		for _, choice in ipairs(choices) do
			container:Add(choice[1], choice[2])
		end
		return container:GetData()
	end
	Settings.CreateDropdown(category, setting, options, tooltip)
	return setting
end

function Compat.AddButton(layout, name, buttonText, onClick, tooltip)
	layout:AddInitializer(CreateSettingsButtonInitializer(name, buttonText, onClick, tooltip, true))
end

function Compat.RegisterSettingsCategory(category)
	Settings.RegisterAddOnCategory(category)
end

function Compat.OpenSettings(category)
	Settings.OpenToCategory(category:GetID())
end
