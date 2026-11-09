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
			"Storage",
			"Chat channels (recorded while Chat is on, on the main page)",
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

	it("opens from /mrgl settings", function()
		_G.SlashCmdList.MRGLGLGL("settings")
		assert.equals(42, Mock.opened)
		assert.is_table(ns.Settings)
	end)
end)
