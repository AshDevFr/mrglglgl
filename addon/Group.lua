-- Group members met, by name only, so the app can write recurring companions. On Forever the
-- second value of UnitFullName is a surname, which makes "Firstname-Surname" the stable key.
local _, ns = ...

local Storage, Day, Compat = ns.Storage, ns.Day, ns.Compat

local function meet(unit, today)
	local name, surname = UnitFullName(unit)
	if name == nil or Compat.IsSecret(name) or Compat.IsSecret(surname) then
		return
	end
	local key = (surname and surname ~= "") and (name .. "-" .. surname) or name
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

local function onRoster()
	if not Storage.records("group") or not IsInGroup() then
		return
	end
	local today = Day.key(time())
	if IsInRaid() then
		for i = 1, GetNumGroupMembers() do
			if not UnitIsUnit("raid" .. i, "player") then
				meet("raid" .. i, today)
			end
		end
	else
		-- The party count includes the player; party1 to partyN-1 are the others.
		for i = 1, GetNumGroupMembers() - 1 do
			meet("party" .. i, today)
		end
	end
end

ns.On("GROUP_ROSTER_UPDATE", onRoster)
ns.On("PLAYER_LOGIN", onRoster)
