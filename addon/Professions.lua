-- Professions: skill-ups, crafting and gathering, each aggregated per day.
--
-- Skill-ups come from comparing skill ranks (C_SkillInfo, which Forever's own skills panel reads)
-- whenever the client says they changed: no chat parsing, and weapon skills that rise every few
-- hits collapse into one record per skill per day. Crafts come from the crafting result event,
-- with the "You create" message as a fallback. Gathering is the loot window that opens right after
-- a successful mining, herb gathering or skinning cast.
local _, ns = ...

local Storage, Day, Compat = ns.Storage, ns.Day, ns.Compat

local Professions = {}
ns.Professions = Professions

-- A loot window this soon after a gathering cast holds what was gathered.
local GATHER_WINDOW = 5
-- The crafting event and the crafting message report the same craft; within this many seconds of
-- each other for the same item they count once.
local CRAFT_ECHO_SECONDS = 2

local ranks
local pendingGather
local windowOpen = false
local lastCraft = {}

local function today()
	return Day.get(Storage.journal(), Day.key(time()))
end

-- Skill rank by skill ID, with each skill's name and the header it sits under in the skills list.
-- Skills under a collapsed header are not listed by the client; they are simply not compared.
local function readSkills()
	local skills, header = {}, nil
	for index = 1, C_SkillInfo.GetNumSkillLines() do
		local info = C_SkillInfo.GetSkillLineInfo(index)
		if info then
			if info.isHeader then
				header = info.name
			elseif info.skillID then
				skills[info.skillID] = { name = info.name, rank = info.rank, cat = header }
			end
		end
	end
	return skills
end

function Professions.compareSkills()
	local now = readSkills()
	local before = ranks
	ranks = now
	if before == nil or not Storage.records("professions") then
		return
	end
	for skillId, skill in pairs(now) do
		local old = before[skillId]
		if old and skill.rank > old.rank then
			local day = today()
			day.skills = day.skills or {}
			local record = day.skills[skillId]
			if record == nil then
				record = { name = skill.name, cat = skill.cat, from = old.rank }
				day.skills[skillId] = record
			end
			record.to = skill.rank
		end
	end
end

ns.On("PLAYER_ENTERING_WORLD", function()
	if ranks == nil then
		ranks = readSkills()
	end
end)
ns.On("SKILL_LINES_CHANGED", Professions.compareSkills)

local function itemName(link)
	local name = Compat.GetItemNameAndQuality(link)
	return name or link:match("%[(.-)%]")
end

local function addCraft(itemId, quantity, link, source)
	local now = time()
	local last = lastCraft[itemId]
	if last and last.source ~= source and now - last.at <= CRAFT_ECHO_SECONDS then
		lastCraft[itemId] = nil
		return
	end
	lastCraft[itemId] = { at = now, source = source }
	local day = today()
	day.crafted = day.crafted or {}
	local record = day.crafted[itemId]
	if record == nil then
		record = { n = 0, name = itemName(link) }
		day.crafted[itemId] = record
	end
	record.n = record.n + quantity
end

ns.On("TRADE_SKILL_ITEM_CRAFTED_RESULT", function(_, data)
	if Storage.records("professions") and data and data.itemID and data.itemID > 0 then
		addCraft(data.itemID, data.quantity or 1, data.hyperlink or "", "event")
	end
end)

ns.On("CHAT_MSG_LOOT", function(_, text)
	if not Storage.records("professions") then
		return
	end
	local link, quantity = Compat.ParseCreateMessage(text)
	local itemId = link and Compat.ItemIdFromLink(link)
	if itemId then
		addCraft(itemId, quantity, link, "message")
	end
end)

ns.On("UNIT_SPELLCAST_SUCCEEDED", function(_, unit, _, spellId)
	if unit ~= "player" or not Storage.records("professions") then
		return
	end
	local name = C_Spell.GetSpellName(spellId)
	if Compat.IsGatheringSpell(name) then
		pendingGather = { via = name, at = time() }
	end
end)

ns.On("LOOT_OPENED", function()
	if windowOpen then
		return
	end
	windowOpen = true
	local gather = pendingGather
	pendingGather = nil
	if gather == nil or time() - gather.at > GATHER_WINDOW or Compat.IsFishingLoot()
		or not Storage.records("professions") then
		return
	end
	local day = today()
	for slot = 1, GetNumLootItems() do
		local link = GetLootSlotLink(slot)
		local itemId = link and Compat.ItemIdFromLink(link)
		if itemId then
			day.gathered = day.gathered or {}
			local record = day.gathered[itemId]
			if record == nil then
				record = { n = 0, name = itemName(link), via = gather.via }
				day.gathered[itemId] = record
			end
			record.n = record.n + 1
		end
	end
end)

ns.On("LOOT_CLOSED", function()
	windowOpen = false
end)
