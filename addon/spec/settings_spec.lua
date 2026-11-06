local Mock = require("spec.support.wow_mock")
local Addon = require("spec.support.addon")

describe("Settings panel", function()
	local ns

	before_each(function()
		ns = Addon.load(Mock)
		Mock.fire("ADDON_LOADED", "Mrglglgl")
		Mock.fire("PLAYER_LOGIN")
	end)

	it("registers a panel with a toggle for every recording category", function()
		assert.is_true(Mock.registered)
		for _, label in ipairs({ "SESSIONS", "LEVELS", "QUESTS", "QUEST_TEXT", "ZONES", "KILLS", "DEATHS", "FISHING",
			"LOOT", "GROUP" }) do
			local control = Mock.controls["MRGLGLGL_RECORD_" .. label]
			assert.is_table(control, label)
			assert.equals("checkbox", control.kind)
			assert.is_true(control.setting:GetValue(), label)
		end
		assert.same({ "What gets recorded", "Storage", "Screenshots", "Forget recorded data" }, Mock.headers)
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

	it("offers retention of 7, 14 or 30 days and stores the choice", function()
		local retention = Mock.controls.MRGLGLGL_RETENTION
		assert.same({ 7, 14, 30 }, { retention.options[1].value, retention.options[2].value, retention.options[3].value })
		assert.equals(14, retention.setting:GetValue())
		retention.setting:SetValue(30)
		assert.equals(30, _G.MrglglglDB.settings.retentionDays)
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
