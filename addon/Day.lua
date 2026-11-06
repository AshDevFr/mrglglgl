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

-- The session's last event, whose place and level stand for the session's end: at logout the
-- client may answer with an empty place and a stale level.
local lastEvent

-- Where the session last was: the last event's zone, subzone, map and level, for a stamp taken
-- when the client can no longer be trusted to say. Nil before the first event.
function Day.lastPlace()
	if lastEvent == nil then
		return nil
	end
	return { z = lastEvent.z, sz = lastEvent.sz, m = lastEvent.m, lv = lastEvent.lv }
end

-- The highest level seen this session. A level never goes down, but at logout the client can
-- report the level from before the session's level-ups.
local highestLevel = 0

-- Where and when the player is now: the stamp every event carries. Taken early for an event that
-- can only be appended later, so it records where it happened rather than where the player is
-- by then.
function Day.snapshot()
	local level = UnitLevel("player")
	if level and level < highestLevel then
		level = highestLevel
	end
	return {
		t = time(),
		z = GetZoneText(),
		sz = GetSubZoneText(),
		m = C_Map.GetBestMapForUnit("player"),
		lv = level,
	}
end

-- Appends an event of kind k to the day it happens in, stamped with time, zone, subzone, map ID
-- and level. fields may add or override any of them; an event completed later passes the
-- snapshot taken when it happened, and lands in that day. Callers check the recording gate.
function Day.append(kind, fields)
	local event = Day.snapshot()
	event.k = kind
	if fields then
		for key, value in pairs(fields) do
			event[key] = value
		end
	end
	if type(event.lv) == "number" and event.lv > highestLevel then
		highestLevel = event.lv
	end
	lastEvent = event
	local events = Day.get(ns.Storage.journal(), Day.key(event.t)).ev
	events[#events + 1] = event
	return event
end
