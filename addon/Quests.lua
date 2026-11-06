-- Quests accepted and turned in, with each quest's title and description stored once.
--
-- The description is read from the quest detail frame while it is open (QUEST_DETAIL), which
-- changes nothing in the player's quest log, and kept until QUEST_ACCEPTED confirms the quest.
local _, ns = ...

local Storage, Day, Compat = ns.Storage, ns.Day, ns.Compat

local pending

ns.On("QUEST_DETAIL", function()
	if not Storage.records("quests") then
		return
	end
	local questId, title, text = Compat.GetQuestDetailText()
	pending = { id = questId, title = title, text = text }
end)

local function storeQuest(questId)
	local quests = Storage.journal().quests
	local quest = quests[questId] or {}
	quests[questId] = quest
	local detail = pending and pending.id == questId and pending or nil
	quest.title = quest.title or (detail and detail.title) or C_QuestLog.GetTitleForQuestID(questId)
	if quest.text == nil and detail and Storage.records("questText") then
		quest.text = detail.text
	end
end

ns.On("QUEST_ACCEPTED", function(_, questId)
	if not Storage.records("quests") then
		return
	end
	storeQuest(questId)
	Day.append("quest_accept", { q = questId })
	pending = nil
end)

ns.On("QUEST_TURNED_IN", function(_, questId, xp, money)
	if Storage.records("quests") then
		Day.append("quest_turnin", { q = questId, xp = xp, money = money })
	end
end)
