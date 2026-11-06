-- Fishing catches, junk included. Boots are counted separately: they are the journal's oldest joke.
local _, ns = ...

local Storage, Day, Compat = ns.Storage, ns.Day, ns.Compat

local Fishing = {}
ns.Fishing = Fishing

-- Time of the last fishing loot window, so Loot does not count a rare fish twice.
Fishing.lastCatchAt = nil

local windowOpen = false
-- Catches whose item data was not cached yet, by item ID, completed on GET_ITEM_INFO_RECEIVED.
local waiting = {}

local function complete(event, link)
	local name, quality = Compat.GetItemNameAndQuality(link)
	if name == nil then
		local list = waiting[event.item] or {}
		list[#list + 1] = { event = event, link = link }
		waiting[event.item] = list
		return
	end
	event.name = name
	event.junk = quality == 0
	if event.junk and Compat.IsBootCatch(link) then
		event.boot = true
		local stats = Storage.journal().stats
		stats.boots = stats.boots + 1
		if stats.boots == 1 then
			ns.Emit("firstBoot")
		end
	end
end

ns.On("LOOT_OPENED", function()
	-- The client can report one loot window more than once; a catch is counted once per window.
	if windowOpen or not Compat.IsFishingLoot() then
		return
	end
	windowOpen = true
	Fishing.lastCatchAt = time()
	if not Storage.records("fishing") then
		return
	end
	for slot = 1, GetNumLootItems() do
		local link = GetLootSlotLink(slot)
		if link then
			complete(Day.append("fish", { item = Compat.ItemIdFromLink(link) }), link)
		end
	end
end)

ns.On("LOOT_CLOSED", function()
	windowOpen = false
end)

ns.On("GET_ITEM_INFO_RECEIVED", function(_, itemId, success)
	local list = waiting[itemId]
	if list == nil or not success then
		return
	end
	waiting[itemId] = nil
	for _, entry in ipairs(list) do
		complete(entry.event, entry.link)
	end
end)
