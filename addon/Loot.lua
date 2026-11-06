-- Notable loot: items at or above the quality threshold that the player received.
--
-- The "You receive loot" message is the record of what the player actually got. Where the client
-- withholds chat text (dungeons, raids, encounters, PvP), the loot window is read instead.
local _, ns = ...

local Storage, Day, Compat = ns.Storage, ns.Day, ns.Compat

-- A loot message this soon after a fishing window is the catch itself, already recorded.
local FISHING_ECHO_SECONDS = 2

-- Items whose quality was unknown yet, by item ID, judged on GET_ITEM_INFO_RECEIVED.
local waiting = {}

local function consider(link, quantity, snapshot)
	local name, quality = Compat.GetItemNameAndQuality(link)
	local itemId = Compat.ItemIdFromLink(link)
	if name == nil then
		local list = waiting[itemId] or {}
		list[#list + 1] = { link = link, quantity = quantity, snapshot = snapshot }
		waiting[itemId] = list
		return
	end
	if quality < Storage.settings().lootQuality then
		return
	end
	snapshot.item, snapshot.name, snapshot.ql, snapshot.qty = itemId, name, quality, quantity
	Day.append("loot", snapshot)
end

ns.On("CHAT_MSG_LOOT", function(_, text)
	if not Storage.records("loot") then
		return
	end
	local fished = ns.Fishing.lastCatchAt
	if fished and time() - fished <= FISHING_ECHO_SECONDS then
		return
	end
	local link, quantity = Compat.ParseLootMessage(text)
	if link then
		consider(link, quantity, Day.snapshot())
	end
end)

ns.On("LOOT_OPENED", function()
	if not Storage.records("loot") or not Compat.InChatLockdown() or Compat.IsFishingLoot() then
		return
	end
	for slot = 1, GetNumLootItems() do
		local link = GetLootSlotLink(slot)
		if link and not Compat.IsSecret(link) then
			consider(link, nil, Day.snapshot())
		end
	end
end)

ns.On("GET_ITEM_INFO_RECEIVED", function(_, itemId, success)
	local list = waiting[itemId]
	if list == nil or not success then
		return
	end
	waiting[itemId] = nil
	for _, entry in ipairs(list) do
		consider(entry.link, entry.quantity, entry.snapshot)
	end
end)
