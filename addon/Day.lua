-- Days and the event log. A day is a local calendar date; every event belongs to the day of its
-- own timestamp, so a session that crosses midnight splits across both days.
local _, ns = ...

local Day = {}
ns.Day = Day

function Day.key(t)
	return date("%Y-%m-%d", t)
end

-- Returns the day record for key, creating it.
function Day.get(journal, key)
	local day = journal.days[key]
	if day == nil then
		day = { ev = {} }
		journal.days[key] = day
	end
	day.ev = day.ev or {}
	return day
end

-- Appends an event of kind k to the day it happens in, stamped with time, zone, subzone, map ID
-- and level. fields may add or override any of them. Callers check the recording gate first.
function Day.append(kind, fields)
	local t = time()
	local event = {
		t = t,
		k = kind,
		z = GetZoneText(),
		sz = GetSubZoneText(),
		m = C_Map.GetBestMapForUnit("player"),
		lv = UnitLevel("player"),
	}
	if fields then
		for key, value in pairs(fields) do
			event[key] = value
		end
	end
	local events = Day.get(ns.Storage.journal(), Day.key(t)).ev
	events[#events + 1] = event
	return event
end
