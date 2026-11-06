local Mock = require("spec.support.wow_mock")
local Addon = require("spec.support.addon")

local CATEGORIES = {
	"sessions", "levels", "quests", "questText", "zones", "kills", "deaths", "fishing", "loot", "group",
}

describe("Storage", function()
	local ns

	local function start()
		Mock.fire("ADDON_LOADED", "Mrglglgl")
		Mock.fire("PLAYER_LOGIN")
	end

	before_each(function()
		ns = Addon.load(Mock)
	end)

	it("creates both databases with defaults on a fresh character, silently", function()
		start()
		assert.equals(1, _G.MrglglglDB.schema)
		assert.equals(14, _G.MrglglglDB.settings.retentionDays)
		assert.equals(3, _G.MrglglglDB.settings.lootQuality)
		assert.is_false(_G.MrglglglDB.settings.screenshots.levelUp)
		assert.equals(1, _G.MrglglglCharDB.schema)
		local today = _G.MrglglglCharDB.days[os.date("%Y-%m-%d", Mock.now)]
		assert.same({ "session_start" }, { today.ev[1].k, today.ev[2] and today.ev[2].k })
		assert.same({ boots = 0, murlocsSlain = 0, deathsByMurloc = 0, drownings = 0 }, _G.MrglglglCharDB.stats)
		assert.same({}, Mock.printed)
	end)

	it("records the character's first name, surname and realm at login", function()
		start()
		local char = _G.MrglglglCharDB.char
		assert.equals("Wren", char.name)
		assert.equals("Hollowbrook", char.surname)
		assert.equals("Test Realm", char.realm)
		assert.equals("Tauren", char.race)
		assert.equals("HUNTER", char.class)
		assert.equals("enUS", char.locale)
	end)

	it("ignores ADDON_LOADED for other addons", function()
		Mock.fire("ADDON_LOADED", "SomeOtherAddon")
		assert.is_nil(_G.MrglglglDB)
		assert.is_false(ns.Storage.ready)
	end)

	it("keeps the player's existing settings and adds new defaults around them", function()
		_G.MrglglglDB = { schema = 1, settings = { retentionDays = 30, record = { kills = false } } }
		start()
		assert.equals(30, _G.MrglglglDB.settings.retentionDays)
		assert.is_false(_G.MrglglglDB.settings.record.kills)
		assert.is_true(_G.MrglglglDB.settings.record.fishing)
		assert.equals(3, _G.MrglglglDB.settings.lootQuality)
	end)

	it("gates every category on its own setting", function()
		start()
		for _, category in ipairs(CATEGORIES) do
			assert.is_true(ns.Storage.records(category), category)
		end
		for _, category in ipairs(CATEGORIES) do
			_G.MrglglglDB.settings.record[category] = false
			assert.is_false(ns.Storage.records(category), category)
			for _, other in ipairs(CATEGORIES) do
				if other ~= category then
					assert.is_true(ns.Storage.records(other), category .. " off turned " .. other .. " off")
				end
			end
			_G.MrglglglDB.settings.record[category] = true
		end
	end)

	it("records nothing before the journal is loaded", function()
		assert.is_false(ns.Storage.records("kills"))
	end)

	it("leaves a journal from a newer addon untouched and records nothing", function()
		local newer = { schema = 99, days = { ["2026-11-01"] = { future = true } } }
		_G.MrglglglCharDB = newer
		start()
		assert.equals(99, _G.MrglglglCharDB.schema)
		assert.same({ ["2026-11-01"] = { future = true } }, _G.MrglglglCharDB.days)
		assert.is_nil(_G.MrglglglCharDB.char)
		assert.is_false(ns.Storage.records("kills"))
	end)
end)
