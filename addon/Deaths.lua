-- The player's deaths, with what killed them.
--
-- The combat log is closed to addons on Forever; the death recap is not. Its events are newest
-- first: the first one is the killing blow, either a creature (sourceName, sourceGUID) or the
-- environment (environmentalType, e.g. "Drowning"). The recap can fill a moment after
-- PLAYER_DEAD, so reading it waits briefly and retries.
local _, ns = ...

local Storage, Day, Compat = ns.Storage, ns.Day, ns.Compat

local READ_DELAY = 0.5
local READ_ATTEMPTS = 4

local function readable(value)
	if value == nil or Compat.IsSecret(value) then
		return nil
	end
	return value
end

local function killerFrom(event)
	local env = readable(event.environmentalType)
	if env then
		return { env = env }
	end
	local name = readable(event.sourceName)
	if name == nil then
		return {}
	end
	local npcId = Compat.NpcIdFromGuid(readable(event.sourceGUID))
	local murloc = (npcId ~= nil and ns.Data.murlocs[npcId] == true) or Compat.IsMurlocName(name)
	return { killer = { name = name, npc = npcId, murloc = murloc } }
end

local function record(attempt)
	local events = C_DeathRecap.HasRecapEvents() and C_DeathRecap.GetRecapEvents()
	if (not events or events[1] == nil) and attempt < READ_ATTEMPTS then
		C_Timer.After(READ_DELAY, function() record(attempt + 1) end)
		return
	end
	local fields = events and events[1] and killerFrom(events[1]) or {}
	-- On a hardcore realm a death is the character's last: the journal says so, for a eulogy.
	if Compat.IsHardcore() then
		fields.final = true
	end
	Day.append("death", fields)
	local stats = Storage.journal().stats
	if fields.env == "Drowning" then
		stats.drownings = stats.drownings + 1
	elseif fields.killer and fields.killer.murloc then
		stats.deathsByMurloc = stats.deathsByMurloc + 1
		if stats.deathsByMurloc == 1 then
			ns.Emit("firstDeathByMurloc")
		end
	end
end

ns.On("PLAYER_DEAD", function()
	if Storage.records("deaths") then
		C_Timer.After(READ_DELAY, function() record(1) end)
	end
end)
