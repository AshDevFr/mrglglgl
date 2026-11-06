local Mock = require("spec.support.wow_mock")
local Addon = require("spec.support.addon")

local function shots()
	local list = {}
	local day = _G.MrglglglCharDB.days[os.date("%Y-%m-%d", Mock.now)]
	for _, event in ipairs(day and day.ev or {}) do
		if event.k == "screenshot" then
			list[#list + 1] = event.moment
		end
	end
	return list
end

describe("Screenshots", function()
	before_each(function()
		Addon.load(Mock)
		Mock.fire("ADDON_LOADED", "Mrglglgl")
		Mock.fire("PLAYER_LOGIN")
		Mock.fire("PLAYER_ENTERING_WORLD", true, false)
	end)

	it("takes none by default", function()
		Mock.fire("PLAYER_LEVEL_UP", 6)
		Mock.fire("PLAYER_DEAD")
		Mock.fire("BOSS_KILL", 1144, "Hogger")
		Mock.advance(30)
		assert.equals(0, Mock.screenshots)
		assert.same({}, shots())
	end)

	it("takes exactly one on level up when enabled, a moment later, and records it", function()
		_G.MrglglglDB.settings.screenshots.levelUp = true
		Mock.fire("PLAYER_LEVEL_UP", 6)
		assert.equals(0, Mock.screenshots)
		Mock.advance(1)
		assert.equals(1, Mock.screenshots)
		Mock.fire("SCREENSHOT_SUCCEEDED")
		assert.same({ "levelUp" }, shots())
	end)

	it("takes one, not two, for deaths three seconds apart", function()
		_G.MrglglglDB.settings.screenshots.death = true
		Mock.fire("PLAYER_DEAD")
		Mock.advance(3)
		Mock.fire("PLAYER_DEAD")
		Mock.advance(5)
		assert.equals(1, Mock.screenshots)
	end)

	it("takes one on the first boot fished", function()
		_G.MrglglglDB.settings.screenshots.firstBoot = true
		Mock.items[6150] = { name = "Old Boot", quality = 0 }
		Mock.fishing, Mock.loot = true, { "|Hitem:6150::|h[Old Boot]|h" }
		Mock.fire("LOOT_OPENED", true, false)
		Mock.fire("LOOT_CLOSED")
		Mock.advance(1)
		assert.equals(1, Mock.screenshots)
		Mock.advance(20)
		Mock.fire("LOOT_OPENED", true, false)
		Mock.fire("LOOT_CLOSED")
		Mock.advance(1)
		assert.equals(1, Mock.screenshots)
	end)

	it("does not record the player's own screenshots", function()
		Mock.fire("SCREENSHOT_SUCCEEDED")
		assert.same({}, shots())
	end)

	it("records nothing when the client reports a failure", function()
		_G.MrglglglDB.settings.screenshots.levelUp = true
		Mock.fire("PLAYER_LEVEL_UP", 6)
		Mock.advance(1)
		Mock.fire("SCREENSHOT_FAILED")
		Mock.fire("SCREENSHOT_SUCCEEDED")
		assert.same({}, shots())
	end)
end)
