-- Group members met, by name only, so the app can write recurring companions. On Forever the
-- second value of UnitFullName is a surname, which makes "Firstname-Surname" the stable key.
--
-- How long each companion stayed in the group is kept per day, in seconds (`grp`), so the app can
-- tell a long session with friends from a moment grouped to kill a boss. Time is credited to the
-- day it is counted in: when the companion leaves, when the player leaves the group, and at
-- logout or reload.
local _, ns = ...

local Storage, Day, Compat = ns.Storage, ns.Day, ns.Compat

-- Companions in the group now, by key, and the time each was first seen in it.
local present = {}

local function keyOf(unit)
	local name, surname = UnitFullName(unit)
	if name == nil or Compat.IsSecret(name) or Compat.IsSecret(surname) then
		return nil
	end
	return (surname and surname ~= "") and (name .. "-" .. surname) or name
end

local function meet(key, today)
	local companions = Storage.journal().companions
	local companion = companions[key]
	if companion and companion.last == today then
		return
	end
	if companion == nil then
		companion = { first = today, days = 0 }
		companions[key] = companion
	end
	companion.last = today
	companion.days = companion.days + 1
	Day.append("group_join", { who = key })
end

-- Adds the time `key` spent in the group since `since` to today's record.
local function credit(key, since, now)
	local seconds = now - since
	if seconds <= 0 then
		return
	end
	local day = Day.get(Storage.journal(), Day.key(now))
	day.grp = day.grp or {}
	day.grp[key] = (day.grp[key] or 0) + seconds
end

-- Credits everyone still counted and forgets them.
local function creditAll(now)
	for key, since in pairs(present) do
		credit(key, since, now)
	end
	present = {}
end

-- The keys of the other members of the group now.
local function members()
	local keys = {}
	if IsInRaid() then
		for i = 1, GetNumGroupMembers() do
			if not UnitIsUnit("raid" .. i, "player") then
				local key = keyOf("raid" .. i)
				if key then
					keys[key] = true
				end
			end
		end
	else
		-- The party count includes the player; party1 to partyN-1 are the others.
		for i = 1, GetNumGroupMembers() - 1 do
			local key = keyOf("party" .. i)
			if key then
				keys[key] = true
			end
		end
	end
	return keys
end

local function onRoster()
	if not Storage.records("group") then
		present = {}
		return
	end
	local now = time()
	if not IsInGroup() then
		creditAll(now)
		return
	end
	local today = Day.key(now)
	local here = members()
	for key, since in pairs(present) do
		if not here[key] then
			credit(key, since, now)
			present[key] = nil
		end
	end
	for key in pairs(here) do
		meet(key, today)
		if present[key] == nil then
			present[key] = now
		end
	end
end

local function onLogout()
	if Storage.records("group") then
		creditAll(time())
	end
	present = {}
end

ns.On("GROUP_ROSTER_UPDATE", onRoster)
ns.On("PLAYER_LOGIN", onRoster)
ns.On("PLAYER_LOGOUT", onLogout)
