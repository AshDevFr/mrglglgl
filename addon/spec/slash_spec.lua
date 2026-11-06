local Mock = require("spec.support.wow_mock")
local Addon = require("spec.support.addon")

local function run(command)
	Mock.printed = {}
	_G.SlashCmdList.MRGLGLGL(command)
	return table.concat(Mock.printed, "\n")
end

describe("Slash commands", function()
	before_each(function()
		Addon.load(Mock)
		Mock.fire("ADDON_LOADED", "Mrglglgl")
		Mock.fire("PLAYER_LOGIN")
		Mock.fire("PLAYER_ENTERING_WORLD", true, false)
	end)

	it("registers /mrgl and /mrglglgl", function()
		assert.equals("/mrgl", _G.SLASH_MRGLGLGL1)
		assert.equals("/mrglglgl", _G.SLASH_MRGLGLGL2)
		assert.is_function(_G.SlashCmdList.MRGLGLGL)
	end)

	it("reports today's counts per category, matching what happened", function()
		Mock.fire("QUEST_DETAIL")
		Mock.fire("QUEST_ACCEPTED", 749)
		Mock.setUnit("target", { guid = "Creature-0-1-1-1-2958-1", name = "Prairie Wolf" })
		Mock.fire("PLAYER_TARGET_CHANGED")
		Mock.fire("PARTY_KILL", "Player-1-0001", "Creature-0-1-1-1-2958-1")
		Mock.fire("PARTY_KILL", "Player-1-0001", "Creature-0-1-1-1-2958-2")
		Mock.items[6150] = { name = "Old Boot", quality = 0 }
		Mock.fishing, Mock.loot = true, { "|Hitem:6150::|h[Old Boot]|h" }
		Mock.fire("LOOT_OPENED", true, false)
		Mock.fire("LOOT_CLOSED")
		Mock.recap = { { sourceName = "Prairie Wolf", sourceGUID = "Creature-0-1-1-1-2958-3" } }
		Mock.fire("PLAYER_DEAD")
		Mock.advance(1)

		local out = run("status")
		assert.matches("Quests: 1 accepted, 0 turned in", out, 1, true)
		assert.matches("Kills: 2 (murlocs: 0, unnamed: 0)", out, 1, true)
		assert.matches("Deaths: 1", out, 1, true)
		assert.matches("Fishing: 1 catches, 1 junk, 1 boots", out, 1, true)
		assert.matches("Boots kept, all time: 1", out, 1, true)
	end)

	it("says which categories are not being recorded", function()
		_G.MrglglglDB.settings.record.kills = false
		assert.matches("Kills: not recorded", run("status"), 1, true)
	end)

	it("treats a bare /mrgl as status, and anything unknown as help", function()
		assert.matches("Recorded today", run(""), 1, true)
		assert.matches("Commands:", run("frobnicate"), 1, true)
	end)

	it("prints raw events for debugging", function()
		Mock.fire("PLAYER_LEVEL_UP", 6)
		assert.matches("k=level", run("debug raw"), 1, true)
	end)

	it("prints nothing unless a command is used", function()
		Mock.printed = {}
		Mock.fire("PLAYER_LEVEL_UP", 6)
		Mock.fire("PLAYER_LOGOUT")
		assert.same({}, Mock.printed)
	end)
end)
