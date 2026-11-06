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
	quest.last = ns.Day.key(time())
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
		local quests = Storage.journal().quests
		local quest = quests[questId]
		if not quest then
			-- Accepted before the addon was installed: learn the title now, so the turn-in can be
			-- told by name. Without a title there is nothing worth keeping.
			local title = C_QuestLog.GetTitleForQuestID(questId)
			if title then
				quest = { title = title }
				quests[questId] = quest
			end
		end
		if quest then
			quest.last = Day.key(time())
		end
	end
end)
