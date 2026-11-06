-- Kills by the player and party, aggregated per NPC per day, and boss kills.
--
-- Addons cannot read the combat log on Forever, so kills come from PARTY_KILL, which carries only
-- GUIDs. A creature's name and classification come from a cache of units the client has shown
-- (target, mouseover, nameplates), else from the XP message that follows the kill. In dungeons,
-- raids, encounters and PvP the client may withhold the target GUID; those kills are counted
-- anonymously.
local _, ns = ...

local Storage, Day, Compat = ns.Storage, ns.Day, ns.Compat

-- The cache is wiped when it grows past this, and on every zone change.
local CACHE_LIMIT = 300
-- An XP message names the last unnamed kill if it arrives within this many seconds.
local XP_NAME_WINDOW = 1

local Kills = {}
ns.Kills = Kills

local cache, cacheSize = {}, 0
local groupGuids = {}
-- The kill waiting for its name from the XP message that follows it. When a second unnamed
-- creature type dies within the window, one XP message cannot say which kill it names: neither
-- is named, rather than one taking the other's name (and murloc count).
local unnamed, unnamedNpc, unnamedAt
local ambiguous = false

local function remember(unit)
	local guid = UnitGUID(unit)
	if guid == nil or Compat.IsSecret(guid) or cache[guid] then
		return
	end
	local name = UnitName(unit)
	if name == nil or Compat.IsSecret(name) then
		return
	end
	if cacheSize >= CACHE_LIMIT then
		cache, cacheSize = {}, 0
	end
	cache[guid] = { name = name, cls = UnitClassification(unit) }
	cacheSize = cacheSize + 1
end

ns.On("PLAYER_TARGET_CHANGED", function() remember("target") end)
ns.On("UPDATE_MOUSEOVER_UNIT", function() remember("mouseover") end)
ns.On("NAME_PLATE_UNIT_ADDED", function(_, unit) remember(unit) end)
ns.On("ZONE_CHANGED_NEW_AREA", function() cache, cacheSize = {}, 0 end)

local function refreshGroup()
	groupGuids = {}
	local player = UnitGUID("player")
	if player then
		groupGuids[player] = true
	end
	local prefix = IsInRaid() and "raid" or "party"
	for i = 1, GetNumGroupMembers() do
		local guid = UnitGUID(prefix .. i)
		if guid and not Compat.IsSecret(guid) then
			groupGuids[guid] = true
		end
	end
end

ns.On("PLAYER_LOGIN", refreshGroup)
ns.On("GROUP_ROSTER_UPDATE", refreshGroup)

local function isMurloc(npcId, name)
	return ns.Data.murlocs[npcId] == true or Compat.IsMurlocName(name)
end

function Kills.onPartyKill(attackerGUID, targetGUID)
	-- First, and without allocating: most kills in a crowded zone are other players'.
	if not groupGuids[attackerGUID] or not Storage.records("kills") then
		return
	end
	local day = Day.get(Storage.journal(), Day.key(time()))
	if Compat.IsSecret(targetGUID) then
		day.anonKills = (day.anonKills or 0) + 1
		return
	end
	local npcId = Compat.NpcIdFromGuid(targetGUID)
	if npcId == nil then
		return
	end
	day.kills = day.kills or {}
	local kill = day.kills[npcId]
	if kill == nil then
		local known = cache[targetGUID]
		kill = { n = 0, name = known and known.name, cls = known and known.cls or "unknown" }
		day.kills[npcId] = kill
	end
	kill.n = kill.n + 1
	if kill.name == nil then
		local now = time()
		if unnamed ~= nil and unnamedNpc ~= npcId and now - unnamedAt <= XP_NAME_WINDOW then
			ambiguous = true
		elseif unnamed == nil or now - unnamedAt > XP_NAME_WINDOW then
			ambiguous = false
		end
		unnamed, unnamedNpc, unnamedAt = kill, npcId, now
	end
	kill.murloc = isMurloc(npcId, kill.name)
	if kill.murloc then
		local stats = Storage.journal().stats
		stats.murlocsSlain = stats.murlocsSlain + 1
		if stats.murlocsSlain == 1 then
			ns.Emit("firstMurlocKill")
		end
	end
end

ns.On("PARTY_KILL", function(_, attackerGUID, targetGUID)
	Kills.onPartyKill(attackerGUID, targetGUID)
end)

-- The client's XP messages for a kill: alone, in a group, in a raid. Built once, from the
-- client's own strings, so they follow its language.
local xpPatterns
local function namedByXp(text)
	if xpPatterns == nil then
		xpPatterns = {}
		for _, format in ipairs({ COMBATLOG_XPGAIN_FIRSTPERSON, COMBATLOG_XPGAIN_FIRSTPERSON_GROUP,
			COMBATLOG_XPGAIN_FIRSTPERSON_RAID }) do
			if type(format) == "string" then
				local escaped = format:gsub("[%^%$%(%)%.%[%]%*%+%-%?]", "%%%0")
				xpPatterns[#xpPatterns + 1] = "^" .. escaped:gsub("%%s", "(.-)"):gsub("%%d", "%%d+") .. "$"
			end
		end
	end
	for i = 1, #xpPatterns do
		local name = text:match(xpPatterns[i])
		if name then
			return name
		end
	end
end

ns.On("CHAT_MSG_COMBAT_XP_GAIN", function(_, text)
	if unnamed == nil or time() - unnamedAt > XP_NAME_WINDOW or Compat.IsSecret(text) then
		return
	end
	if ambiguous then
		unnamed, ambiguous = nil, false
		return
	end
	local name = namedByXp(text)
	if name then
		unnamed.name = name
		-- A murloc known only by name was not counted when the kill came in.
		if not unnamed.murloc and isMurloc(unnamedNpc, name) then
			unnamed.murloc = true
			local stats = Storage.journal().stats
			stats.murlocsSlain = stats.murlocsSlain + unnamed.n
			if stats.murlocsSlain == unnamed.n then
				ns.Emit("firstMurlocKill")
			end
		end
		unnamed = nil
	end
end)

ns.On("BOSS_KILL", function(_, encounterId, encounterName)
	if Storage.records("kills") then
		Day.append("boss", { enc = encounterId, name = encounterName })
	end
end)
