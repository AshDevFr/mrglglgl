-- Zone and subzone changes, and discoveries.
local _, ns = ...

local Storage, Day, Compat = ns.Storage, ns.Day, ns.Compat

-- A subzone recorded within this many seconds is not recorded again, so walking along a border
-- does not write an event per step.
local SUBZONE_QUIET_SECONDS = 60

local lastZone
local subzoneSeen = {}

-- A character already played before the addon was installed has been to many places; visiting
-- them in the first recorded session is not discovering them. In that session a visit marks the
-- place as known without an event. The client's discovery message is always a real discovery.
-- A level 1 character with a new journal is new, so its first visits count.
local seeding = false

-- One discovery per area per character, whichever comes first: the client's discovery message or
-- the character's first recorded visit (the fallback when the message is missed or absent).
local function discover(area, source)
	if area == nil or area == "" then
		return
	end
	local journal = Storage.journal()
	journal.discovered = journal.discovered or {}
	if journal.discovered[area] then
		return
	end
	journal.discovered[area] = true
	if seeding and source == "visit" then
		return
	end
	Day.append("discover", { area = area, src = source })
end

-- Zones flown over on a flight path were not visited: a druid flying home from Moonglade crossed
-- four zones and fifty places, and the journal told them all. Nothing is recorded in flight, and
-- the place landed in is recorded when the flight ends.
local skippedInFlight = false

local function onZoneChanged()
	if not Storage.records("zones") then
		return
	end
	if Compat.OnTaxi() then
		skippedInFlight = true
		return
	end
	local zone, subzone, now = GetZoneText(), GetSubZoneText(), time()
	if zone ~= lastZone then
		lastZone = zone
		Day.append("zone")
		discover(zone, "visit")
	end
	if subzone ~= "" then
		local seen = subzoneSeen[subzone]
		if seen == nil or now - seen >= SUBZONE_QUIET_SECONDS then
			Day.append("subzone")
		end
		subzoneSeen[subzone] = now
		discover(subzone, "visit")
	end
end

ns.On("PLAYER_LOGIN", function()
	seeding = Storage.fresh and UnitLevel("player") > 1
end)

ns.On("ZONE_CHANGED_NEW_AREA", onZoneChanged)
ns.On("ZONE_CHANGED", onZoneChanged)
ns.On("ZONE_CHANGED_INDOORS", onZoneChanged)
-- Fires when a flight path lands, and also after a stun or a fear, which must not record the
-- subzone again.
ns.On("PLAYER_CONTROL_GAINED", function()
	if skippedInFlight then
		skippedInFlight = false
		onZoneChanged()
	end
end)

-- The discovery message arrives as a UI info message or a system chat line; which one was not
-- observed on Forever, so both are handled.
ns.On("UI_INFO_MESSAGE", function(_, _, message)
	if Storage.records("zones") then
		discover(Compat.IsDiscoveryMessage(message), "message")
	end
end)

ns.On("CHAT_MSG_SYSTEM", function(_, text)
	if Storage.records("zones") then
		discover(Compat.IsDiscoveryMessage(text), "message")
	end
end)
