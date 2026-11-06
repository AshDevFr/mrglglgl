-- Every client call that is restricted, unconfirmed or version-sensitive on WoW: Forever goes
-- through here, so the rest of the addon never branches on client details. docs/forever-api.md
-- records the evidence for each function's assumptions and fallbacks.
local _, ns = ...

local Compat = {}
ns.Compat = Compat

local L = ns.L

-- Forever reports interface 16xxx. WOW_PROJECT_ID is WOW_PROJECT_CAMELOT today, but that name may
-- change, so the interface range is the check.
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

function Compat.IsFishingLoot()
	return IsFishingLoot ~= nil and IsFishingLoot() == true
end

local GetItemInfo = (C_Item and C_Item.GetItemInfo) or GetItemInfo

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

function Compat.RegisterSettingsPanel(frame, name)
	local category = Settings.RegisterCanvasLayoutCategory(frame, name)
	Settings.RegisterAddOnCategory(category)
	return category
end
