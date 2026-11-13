-- Quests accepted and turned in, with each quest's title, description and hand-in text stored
-- once.
--
-- The description is read from the quest detail frame while it is open (QUEST_DETAIL), which
-- changes nothing in the player's quest log, and kept until QUEST_ACCEPTED confirms the quest.
-- The hand-in text (what the quest giver says when it is done) is read the same way from the
-- reward frame (QUEST_COMPLETE) and kept until QUEST_TURNED_IN confirms it.
local _, ns = ...

local Storage, Day, Compat = ns.Storage, ns.Day, ns.Compat

local pending, pendingDone

ns.On("QUEST_DETAIL", function()
	if not Storage.records("quests") then
		return
	end
	local questId, title, text = Compat.GetQuestDetailText()
	pending = { id = questId, title = title, text = text }
end)

ns.On("QUEST_COMPLETE", function()
	if not Storage.records("quests") then
		return
	end
	local questId, text = Compat.GetQuestRewardText()
	pendingDone = { id = questId, text = text }
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
	-- Shown as [9], [9+] (elite), [9D] (dungeon), [9R] (raid) or [9G] (group) beside the title.
	local level, kind = Compat.GetQuestLevelAndKind(questId)
	quest.lv = level or quest.lv
	quest.kind = kind or quest.kind
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
			local done = pendingDone and pendingDone.id == questId and pendingDone.text or nil
			if quest.done == nil and done and done ~= "" and Storage.records("questText") then
				quest.done = done
			end
		end
		pendingDone = nil
	end
end)
