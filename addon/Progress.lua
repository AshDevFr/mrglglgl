-- How the character grows beyond levels and quests: spells learned (druid forms among them),
-- mounts and pets, reputation standings, gear upgrades, where the hearthstone is set and flight
-- paths learned, duels, and dungeon and raid runs. Every event here is registered only when the
-- client knows it, and every client call is guarded: none of them was seen on Forever yet.
local _, ns = ...

local Storage, Day, Compat = ns.Storage, ns.Day, ns.Compat

local Progress = {}
ns.Progress = Progress

local function on(event, fn)
	if Compat.EventExists(event) then
		ns.On(event, fn)
	end
end

-- Spells ---------------------------------------------------------------------------------------

local function learned(_, spellId)
	if Storage.records("spells") and type(spellId) == "number" then
		Day.append("spell", { spell = spellId, name = Compat.GetSpellName(spellId) })
	end
end
-- The newer clients renamed the event; whichever this one has is used, never both.
if Compat.EventExists("LEARNED_SPELL_IN_SKILL_LINE") then
	ns.On("LEARNED_SPELL_IN_SKILL_LINE", learned)
else
	on("LEARNED_SPELL_IN_TAB", learned)
end

-- Mounts and pets ------------------------------------------------------------------------------

on("NEW_MOUNT_ADDED", function(_, mountId)
	if not Storage.records("collections") then
		return
	end
	local journal = _G.C_MountJournal
	local name = journal and type(journal.GetMountInfoByID) == "function" and journal.GetMountInfoByID(mountId)
	Day.append("mount", { name = type(name) == "string" and name or nil })
end)

on("NEW_PET_ADDED", function(_, petGuid)
	if not Storage.records("collections") then
		return
	end
	local journal = _G.C_PetJournal
	local name
	if journal and type(journal.GetPetInfoByPetID) == "function" then
		local _, customName, _, _, _, _, _, speciesName = journal.GetPetInfoByPetID(petGuid)
		name = customName or speciesName
	end
	Day.append("pet", { name = type(name) == "string" and name or nil })
end)

-- Reputation -----------------------------------------------------------------------------------

local standings

ns.On("PLAYER_ENTERING_WORLD", function()
	standings = Compat.GetStandings()
end)

-- Only a standing that rises is told: "Became Honored with Thunder Bluff". A faction met for the
-- first time starts at its standing, which is not news.
on("UPDATE_FACTION", function()
	if standings == nil then
		return
	end
	local now = Compat.GetStandings()
	for faction, standing in pairs(now) do
		local before = standings[faction]
		if before and standing and standing > before and Storage.records("reputation") then
			Day.append("rep", { faction = faction, standing = Compat.StandingName(standing) or tostring(standing) })
		end
	end
	standings = now
end)

-- Gear -----------------------------------------------------------------------------------------

-- Head to tabard, as the client numbers them; shirt (4) and tabard (19) are not upgrades.
local SLOTS = { 1, 2, 3, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18 }
local equipped = {}

-- An empty slot counts as item level 0, so a first ring is an upgrade; a slot whose item is not
-- cached yet stays unknown, and its first change is learnt rather than told.
ns.On("PLAYER_ENTERING_WORLD", function()
	for _, slot in ipairs(SLOTS) do
		local item = Compat.GetEquipped(slot)
		if item then
			equipped[slot] = item.level
		elseif type(_G.GetInventoryItemLink) == "function" and not _G.GetInventoryItemLink("player", slot) then
			equipped[slot] = 0
		end
	end
end)

-- An item put on that beats what the slot held: a better item level. Swapping back and forth
-- between known pieces tells nothing new, since the slot remembers its best.
on("PLAYER_EQUIPMENT_CHANGED", function(_, slot)
	local item = Compat.GetEquipped(slot)
	if not item then
		return
	end
	local best = equipped[slot]
	if best and item.level > best and Storage.records("gear") then
		Day.append("gear", { item = item.item, name = item.name, slot = slot, ilvl = item.level, ql = item.quality })
	end
	if best == nil or item.level > best then
		equipped[slot] = item.level
	end
end)

-- Hearthstone and flight paths -----------------------------------------------------------------

on("PLAYER_BINDLOCATION_CHANGED", function()
	if Storage.records("travel") and type(_G.GetBindLocation) == "function" then
		Day.append("bind", { area = _G.GetBindLocation() })
	end
end)

local function flightPath(_, ...)
	for i = 1, select("#", ...) do
		if Compat.IsNewFlightPathMessage((select(i, ...))) and Storage.records("travel") then
			local area = GetSubZoneText()
			Day.append("flight_path", { area = area ~= "" and area or GetZoneText() })
			return
		end
	end
end
-- Which of the two carries the message was not observed; it is the same text in either.
on("UI_INFO_MESSAGE", flightPath)
ns.On("CHAT_MSG_SYSTEM", flightPath)

-- Duels ----------------------------------------------------------------------------------------

ns.On("CHAT_MSG_SYSTEM", function(_, text)
	local duel = Compat.ParseDuelMessage(text)
	if not duel or not Storage.records("duels") then
		return
	end
	local me = UnitName("player")
	if duel.winner == me then
		Day.append("duel", { won = true, who = duel.loser })
	elseif duel.loser == me then
		Day.append("duel", { won = false, who = duel.winner })
	end
end)

-- Dungeons and raids ---------------------------------------------------------------------------

local inside, enteredAt

ns.On("PLAYER_ENTERING_WORLD", function()
	local instance = Compat.GetInstance()
	if inside and (not instance or instance.name ~= inside.name) then
		if Storage.records("dungeons") then
			Day.append("instance", { act = "leave", name = inside.name, kind = inside.kind,
				dur = enteredAt and (time() - enteredAt) or nil })
		end
		inside, enteredAt = nil, nil
	end
	if instance and not inside then
		inside, enteredAt = instance, time()
		if Storage.records("dungeons") then
			Day.append("instance", { act = "enter", name = instance.name, kind = instance.kind })
		end
	end
end)

return Progress
