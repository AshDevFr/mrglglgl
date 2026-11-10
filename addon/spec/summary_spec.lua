local Mock = require("spec.support.wow_mock")
local Addon = require("spec.support.addon")

local DAY = 86400

describe("Login summary", function()
	local function load()
		Addon.load(Mock)
		Mock.fire("ADDON_LOADED", "Mrglglgl")
		Mock.fire("PLAYER_LOGIN")
	end

	-- A finished earlier day with a quest turned in, kills and a boot.
	local function previousDay()
		Mock.advance(-DAY)
		Mock.fire("PLAYER_ENTERING_WORLD", true, false)
		Mock.fire("PLAYER_LEVEL_UP", 6)
		Mock.advance(DAY)
	end

	before_each(function()
		load()
	end)

	it("says nothing by default", function()
		previousDay()
		Mock.printed = {}
		Mock.fire("PLAYER_ENTERING_WORLD", true, false)
		assert.same({}, Mock.printed)
	end)

	it("summarises the last day with data at login when turned on", function()
		previousDay()
		_G.MrglglglDB.settings.loginSummary = true
		Mock.printed = {}
		Mock.fire("PLAYER_ENTERING_WORLD", true, false)
		assert.equals(1, #Mock.printed)
		local expected = "Last time (" .. os.date("%Y-%m-%d", Mock.now - DAY) .. "): 1 level gained. Saved when"
		assert.matches(expected, Mock.printed[1], 1, true)
	end)

	it("stays quiet on a reload and on a loading screen", function()
		previousDay()
		_G.MrglglglDB.settings.loginSummary = true
		Mock.printed = {}
		Mock.fire("PLAYER_ENTERING_WORLD", false, true)
		Mock.fire("PLAYER_ENTERING_WORLD", false, false)
		assert.same({}, Mock.printed)
	end)

	it("prints nothing when there is no earlier day with data", function()
		_G.MrglglglDB.settings.loginSummary = true
		Mock.fire("PLAYER_LEVEL_UP", 6)
		Mock.printed = {}
		Mock.fire("PLAYER_ENTERING_WORLD", true, false)
		assert.same({}, Mock.printed)
	end)
end)
