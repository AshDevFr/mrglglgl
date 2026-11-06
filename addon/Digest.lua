-- A day's digest: the compact summary that survives once the day's raw events are dropped, and
-- the app's main input for writing the day. A pure function of the day record.
local _, ns = ...

local Digest = {}
ns.Digest = Digest

-- Increment when the digest's shape or meaning changes. Days that still have raw events are
-- rebuilt; compacted days keep the version they were built with.
Digest.VERSION = 2

-- Caps keep a busy day's digest small: a summary needs the highlights, and counts stay exact.
-- Record lists cost far more per entry than name lists, so they get tighter caps.
-- Raised well above a real day's numbers: a compacted day's digest is what the app tells the day
-- from, possibly weeks later.
local TOP_KILLS = 40
local LIST_CAP = 60
local RECORD_CAPS = { deaths = 20, loot = 30, fish = 30, notableKills = 30 }

local NOTABLE_CLASSIFICATIONS = { elite = true, rare = true, rareelite = true, worldboss = true }

local function push(list, value, cap)
	if #list < (cap or LIST_CAP) then
		list[#list + 1] = value
	end
end

local function addOnce(list, seen, value)
	if value and value ~= "" and not seen[value] then
		seen[value] = true
		push(list, value)
	end
end

-- Old days keep only their highlights: exact counts, and the first few entries of each list. The
-- app has archived the full day long before; in game nothing reads old digests in detail.
local THIN_RECORDS, THIN_NAMES = 3, 5
local RECORD_LISTS = { "deaths", "kills", "notableKills", "fish", "loot", "skills", "crafted", "gathered" }
local NAME_LISTS = { "zones", "discovered", "accepted", "turnedIn", "companions", "bosses" }

function Digest.thin(digest)
	if digest.thin then
		return
	end
	local function cut(list, cap)
		for i = #list, cap + 1, -1 do
			list[i] = nil
		end
	end
	for _, name in ipairs(RECORD_LISTS) do
		cut(digest[name] or {}, THIN_RECORDS)
	end
	for _, name in ipairs(NAME_LISTS) do
		cut(digest[name] or {}, THIN_NAMES)
	end
	digest.subzones = {}
	digest.thin = true
end

function Digest.build(day)
	local counts = {
		sessions = 0, levels = 0, accepted = 0, turnedIn = 0, deaths = 0, fish = 0, junk = 0, boots = 0,
		loot = 0, discoveries = 0, bosses = 0, group = 0, screenshots = 0,
		kills = 0, murlocs = 0, anonKills = day.anonKills or 0, skillUps = 0, crafted = 0, gathered = 0,
	}
	local digest = {
		v = Digest.VERSION, counts = counts,
		zones = {}, subzones = {}, discovered = {}, deaths = {}, accepted = {}, turnedIn = {},
		fish = {}, loot = {}, companions = {}, bosses = {}, kills = {}, notableKills = {},
		skills = {}, crafted = {}, gathered = {},
	}
	local zonesSeen, subzonesSeen, fishByItem = {}, {}, {}
	local lowLevel, highLevel

	for _, e in ipairs(day.ev or {}) do
		if e.lv then
			lowLevel = (lowLevel == nil or e.lv < lowLevel) and e.lv or lowLevel
			highLevel = (highLevel == nil or e.lv > highLevel) and e.lv or highLevel
		end
		addOnce(digest.zones, zonesSeen, e.z)
		local k = e.k
		if k == "session_start" then
			counts.sessions = counts.sessions + 1
		elseif k == "level" then
			counts.levels = counts.levels + 1
		elseif k == "subzone" then
			addOnce(digest.subzones, subzonesSeen, e.sz)
		elseif k == "discover" then
			counts.discoveries = counts.discoveries + 1
			push(digest.discovered, e.area)
		elseif k == "quest_accept" then
			counts.accepted = counts.accepted + 1
			push(digest.accepted, e.q)
		elseif k == "quest_turnin" then
			counts.turnedIn = counts.turnedIn + 1
			push(digest.turnedIn, e.q)
		elseif k == "death" then
			counts.deaths = counts.deaths + 1
			push(digest.deaths, { killer = e.killer, env = e.env, z = e.z, sz = e.sz }, RECORD_CAPS.deaths)
		elseif k == "fish" then
			counts.fish = counts.fish + 1
			counts.junk = counts.junk + (e.junk and 1 or 0)
			counts.boots = counts.boots + (e.boot and 1 or 0)
			local entry = fishByItem[e.item]
			if entry == nil then
				entry = { item = e.item, name = e.name, n = 0, junk = e.junk, boot = e.boot }
				fishByItem[e.item] = entry
				push(digest.fish, entry, RECORD_CAPS.fish)
			end
			entry.n = entry.n + 1
		elseif k == "loot" then
			counts.loot = counts.loot + 1
			push(digest.loot, { item = e.item, name = e.name, ql = e.ql, qty = e.qty }, RECORD_CAPS.loot)
		elseif k == "group_join" then
			counts.group = counts.group + 1
			push(digest.companions, e.who)
		elseif k == "boss" then
			counts.bosses = counts.bosses + 1
			push(digest.bosses, e.name)
		elseif k == "screenshot" then
			counts.screenshots = counts.screenshots + 1
		end
	end

	local kills = {}
	for npc, kill in pairs(day.kills or {}) do
		counts.kills = counts.kills + kill.n
		if kill.murloc then
			counts.murlocs = counts.murlocs + kill.n
		end
		local entry = { npc = npc, name = kill.name, n = kill.n, cls = kill.cls, murloc = kill.murloc }
		kills[#kills + 1] = entry
		if NOTABLE_CLASSIFICATIONS[kill.cls] then
			push(digest.notableKills, entry, RECORD_CAPS.notableKills)
		end
	end
	-- Most killed first; ties by NPC ID, so the same day always gives the same digest.
	table.sort(kills, function(a, b)
		if a.n ~= b.n then
			return a.n > b.n
		end
		return a.npc < b.npc
	end)
	table.sort(digest.notableKills, function(a, b) return a.npc < b.npc end)
	for i = 1, math.min(TOP_KILLS, #kills) do
		digest.kills[i] = kills[i]
	end
	counts.kills = counts.kills + counts.anonKills

	-- Professions, ordered by ID so the same day always gives the same digest.
	local function sortedIds(t)
		local ids = {}
		for id in pairs(t or {}) do
			ids[#ids + 1] = id
		end
		table.sort(ids)
		return ids
	end
	for _, id in ipairs(sortedIds(day.skills)) do
		local s = day.skills[id]
		counts.skillUps = counts.skillUps + (s.to - s.from)
		push(digest.skills, { skill = id, name = s.name, cat = s.cat, from = s.from, to = s.to })
	end
	for _, id in ipairs(sortedIds(day.crafted)) do
		local c = day.crafted[id]
		counts.crafted = counts.crafted + c.n
		push(digest.crafted, { item = id, name = c.name, n = c.n })
	end
	for _, id in ipairs(sortedIds(day.gathered)) do
		local g = day.gathered[id]
		counts.gathered = counts.gathered + g.n
		push(digest.gathered, { item = id, name = g.name, n = g.n, via = g.via })
	end

	digest.levelFrom, digest.levelTo = lowLevel, highLevel
	local busy = counts.kills + counts.deaths + counts.accepted + counts.turnedIn + counts.loot
		+ counts.levels + counts.discoveries + counts.bosses + counts.group
		+ counts.skillUps + counts.crafted + counts.gathered
	digest.onlyFished = counts.fish > 0 and counts.kills == 0 and counts.accepted == 0 and counts.turnedIn == 0
		and counts.crafted == 0 and counts.gathered == 0
	digest.nothingHappened = busy + counts.fish == 0
	return digest
end
