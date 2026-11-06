local Mock = require("spec.support.wow_mock")
local Addon = require("spec.support.addon")

local function events(kind)
	local list = {}
	for _, event in ipairs(_G.MrglglglCharDB.days[os.date("%Y-%m-%d", Mock.now)].ev) do
		if event.k == kind then
			list[#list + 1] = event
		end
	end
	return list
end

local function offer(questId, title, text)
	Mock.questDetail = { id = questId, title = title, text = text }
	Mock.fire("QUEST_DETAIL", 0)
end

describe("Quests", function()
	before_each(function()
		Addon.load(Mock)
		Mock.fire("ADDON_LOADED", "Mrglglgl")
		Mock.fire("PLAYER_LOGIN")
		Mock.fire("PLAYER_ENTERING_WORLD", true, false)
	end)

	it("stores the title and description from the detail frame when the quest is accepted", function()
		offer(749, "The Ravaged Caravan", "Some marauding wolves have attacked a caravan.")
		Mock.fire("QUEST_ACCEPTED", 749)
		assert.same({ title = "The Ravaged Caravan", text = "Some marauding wolves have attacked a caravan.",
			last = os.date("%Y-%m-%d", Mock.now) }, _G.MrglglglCharDB.quests[749])
		assert.equals(749, events("quest_accept")[1].q)
	end)

	it("stores the text once when a quest is abandoned and accepted again", function()
		offer(749, "The Ravaged Caravan", "First text.")
		Mock.fire("QUEST_ACCEPTED", 749)
		offer(749, "The Ravaged Caravan", "Second text.")
		Mock.fire("QUEST_ACCEPTED", 749)
		assert.equals("First text.", _G.MrglglglCharDB.quests[749].text)
		assert.equals(2, #events("quest_accept"))
	end)

	it("never changes the quest log selection", function()
		offer(749, "The Ravaged Caravan", "Text.")
		Mock.fire("QUEST_ACCEPTED", 749)
		assert.equals(0, Mock.questLogSelections)
	end)

	it("falls back to the quest log title when the quest was accepted without its detail frame", function()
		Mock.questTitles[750] = "The Hunt Continues"
		Mock.fire("QUEST_ACCEPTED", 750)
		assert.same({ title = "The Hunt Continues", last = os.date("%Y-%m-%d", Mock.now) },
			_G.MrglglglCharDB.quests[750])
	end)

	it("does not store a detail frame's text under a different quest", function()
		offer(749, "The Ravaged Caravan", "Text.")
		Mock.questTitles[750] = "The Hunt Continues"
		Mock.fire("QUEST_ACCEPTED", 750)
		assert.same({ title = "The Hunt Continues", last = os.date("%Y-%m-%d", Mock.now) },
			_G.MrglglglCharDB.quests[750])
	end)

	it("keeps the accept event but not the description when quest text is off", function()
		_G.MrglglglDB.settings.record.questText = false
		offer(749, "The Ravaged Caravan", "Text.")
		Mock.fire("QUEST_ACCEPTED", 749)
		assert.equals(1, #events("quest_accept"))
		assert.same({ title = "The Ravaged Caravan", last = os.date("%Y-%m-%d", Mock.now) },
			_G.MrglglglCharDB.quests[749])
	end)

	it("records a turn-in with its rewards", function()
		Mock.fire("QUEST_TURNED_IN", 749, 450, 75)
		local turnin = events("quest_turnin")[1]
		assert.same({ 749, 450, 75 }, { turnin.q, turnin.xp, turnin.money })
	end)

	it("remembers the last day a quest was touched, for pruning its text later", function()
		offer(749, "The Ravaged Caravan", "Text.")
		Mock.fire("QUEST_ACCEPTED", 749)
		Mock.advance(3 * 86400)
		Mock.fire("QUEST_TURNED_IN", 749, 450, 75)
		assert.equals(os.date("%Y-%m-%d", Mock.now), _G.MrglglglCharDB.quests[749].last)
	end)

	it("records nothing when quests are off", function()
		_G.MrglglglDB.settings.record.quests = false
		offer(749, "The Ravaged Caravan", "Text.")
		Mock.fire("QUEST_ACCEPTED", 749)
		Mock.fire("QUEST_TURNED_IN", 749, 450, 75)
		assert.same({}, events("quest_accept"))
		assert.same({}, events("quest_turnin"))
		assert.is_nil(_G.MrglglglCharDB.quests[749])
	end)
end)
