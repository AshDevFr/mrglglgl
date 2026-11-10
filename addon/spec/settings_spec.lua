local Mock = require("spec.support.wow_mock")
local Addon = require("spec.support.addon")

describe("Settings panel", function()
	local ns

	before_each(function()
		ns = Addon.load(Mock)
		Mock.fire("ADDON_LOADED", "Mrglglgl")
		Mock.fire("PLAYER_LOGIN")
		Mock.fire("PLAYER_ENTERING_WORLD", true, false)
	end)

	it("registers a panel with a toggle for every recording category", function()
		assert.is_true(Mock.registered)
		for _, label in ipairs({ "SESSIONS", "LEVELS", "QUESTS", "QUEST_TEXT", "ZONES", "KILLS", "DEATHS", "FISHING",
			"PROFESSIONS", "LOOT", "GROUP" }) do
			local control = Mock.controls["MRGLGLGL_RECORD_" .. label]
			assert.is_table(control, label)
			assert.equals("checkbox", control.kind)
			assert.is_true(control.setting:GetValue(), label)
		end
		local chat = Mock.controls.MRGLGLGL_RECORD_CHAT
		assert.is_false(chat.setting:GetValue(), "chat is opt-in")
		assert.is_true(Mock.controls.MRGLGLGL_CHAT_SAY.setting:GetValue())
		assert.is_false(Mock.controls.MRGLGLGL_CHAT_SAY_OTHERS.setting:GetValue())
		assert.same({
			"What gets recorded",
			"Your adventure",
			"Combat",
			"Gathering and crafting",
			"Items and gold",
			"Growth",
			"People",
			"Storage",
			"Messages",
			"Chat (the channels below only matter while Chat is on)",
			"Screenshots",
			"Forget recorded data (press a button twice within 10 seconds)",
		}, Mock.headers)
	end)

	it("splits the panel into pages, so the long chat list has its own", function()
		assert.is_table(Mock.pages["Chat"])
		assert.is_table(Mock.pages["Screenshots"])
		assert.is_table(Mock.pages["Forget"])
		assert.equals("Chat", Mock.controls.MRGLGLGL_CHAT_SAY_OTHERS.page)
		assert.equals("Mrglglgl", Mock.controls.MRGLGLGL_RECORD_LEVELS.page)
	end)

	it("stops recording a category the moment it is unticked, without a reload", function()
		Mock.controls.MRGLGLGL_RECORD_LEVELS.setting:SetValue(false)
		assert.is_false(_G.MrglglglDB.settings.record.levels)
		Mock.fire("PLAYER_LEVEL_UP", 6)
		local today = _G.MrglglglCharDB.days[os.date("%Y-%m-%d", Mock.now)]
		for _, event in ipairs(today.ev) do
			assert.not_equals("level", event.k)
		end
	end)

	it("offers retention of 14, 30, 60 or 90 days, 30 by default, and stores the choice", function()
		local retention = Mock.controls.MRGLGLGL_RETENTION
		local values = {}
		for i, option in ipairs(retention.options) do
			values[i] = option.value
		end
		assert.same({ 14, 30, 60, 90 }, values)
		assert.equals(30, retention.setting:GetValue())
		retention.setting:SetValue(60)
		assert.equals(60, _G.MrglglglDB.settings.retentionDays)
	end)

	it("offers every screenshot moment, all off by default", function()
		for _, label in ipairs({ "LEVEL_UP", "DEATH", "BOSS_KILL", "FIRST_BOOT", "FIRST_MURLOC_KILL",
			"FIRST_DEATH_BY_MURLOC" }) do
			assert.is_false(Mock.controls["MRGLGLGL_SCREENSHOT_" .. label].setting:GetValue(), label)
		end
	end)

	it("forgets a category only on a second press within the confirmation window", function()
		Mock.fire("PLAYER_LEVEL_UP", 6)
		local forget = Mock.buttons["Levels"].click
		forget()
		local function levels()
			local n = 0
			for _, event in ipairs(_G.MrglglglCharDB.days[os.date("%Y-%m-%d", Mock.now)].ev) do
				n = n + (event.k == "level" and 1 or 0)
			end
			return n
		end
		assert.equals(1, levels())
		Mock.advance(3)
		forget()
		assert.equals(0, levels())
	end)

	it("does not forget when the second press comes too late", function()
		Mock.fire("PLAYER_LEVEL_UP", 6)
		local forget = Mock.buttons["Levels"].click
		forget()
		Mock.advance(30)
		forget()
		local n = 0
		for _, event in ipairs(_G.MrglglglCharDB.days[os.date("%Y-%m-%d", Mock.now)].ev) do
			n = n + (event.k == "level" and 1 or 0)
		end
		assert.equals(1, n)
	end)

	it("puts the Chat switch on the Chat page, off by default, and not on the main page", function()
		local chat = Mock.controls.MRGLGLGL_RECORD_CHAT
		assert.equals("Chat", chat.page)
		assert.is_false(chat.setting:GetValue())
		chat.setting:SetValue(true)
		assert.is_true(_G.MrglglglDB.settings.record.chat)
	end)

	it("groups the recording toggles under headers, with the loot quality after Notable loot", function()
		assert.equals("Mrglglgl", Mock.controls.MRGLGLGL_RECORD_GROUP.page)
		assert.is_table(Mock.controls.MRGLGLGL_LOOT_QUALITY)
	end)

	it("offers the login summary, off by default", function()
		local control = Mock.controls.MRGLGLGL_LOGIN_SUMMARY
		assert.equals("Mrglglgl", control.page)
		assert.is_false(control.setting:GetValue())
		control.setting:SetValue(true)
		assert.is_true(_G.MrglglglDB.settings.loginSummary)
	end)

	it("forgets everything only on a second press, and warns in colour on the first", function()
		Mock.fire("PLAYER_LEVEL_UP", 6)
		Mock.recap = { { sourceName = "Prairie Wolf", sourceGUID = "Creature-0-1-1-1-2958-3" } }
		Mock.fire("PLAYER_DEAD")
		Mock.advance(1)
		local function count(kind)
			local n = 0
			for _, event in ipairs(_G.MrglglglCharDB.days[os.date("%Y-%m-%d", Mock.now)].ev) do
				n = n + (event.k == kind and 1 or 0)
			end
			return n
		end
		local forgetAll = Mock.buttons["Everything"].click
		Mock.printed = {}
		forgetAll()
		assert.matches("|cff", Mock.printed[1], 1, true)
		assert.equals(1, count("level"))
		assert.equals(1, count("death"))
		Mock.advance(3)
		forgetAll()
		assert.equals(0, count("level"))
		assert.equals(0, count("death"))
		assert.matches("Everything recorded", Mock.printed[2], 1, true)
	end)

	it("does not forget everything when the second press comes too late", function()
		Mock.fire("PLAYER_LEVEL_UP", 6)
		local forgetAll = Mock.buttons["Everything"].click
		forgetAll()
		Mock.advance(30)
		forgetAll()
		local n = 0
		for _, event in ipairs(_G.MrglglglCharDB.days[os.date("%Y-%m-%d", Mock.now)].ev) do
			n = n + (event.k == "level" and 1 or 0)
		end
		assert.equals(1, n)
	end)

	it("opens from /mrgl settings", function()
		_G.SlashCmdList.MRGLGLGL("settings")
		assert.equals(42, Mock.opened)
		assert.is_table(ns.Settings)
	end)
end)
