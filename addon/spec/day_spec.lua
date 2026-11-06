local Mock = require("spec.support.wow_mock")
local Addon = require("spec.support.addon")

local function localTime(year, month, day, hour, min)
	return os.time({ year = year, month = month, day = day, hour = hour, min = min, sec = 0 })
end

local function kinds(day)
	local list = {}
	for _, event in ipairs(day and day.ev or {}) do
		list[#list + 1] = event.k
	end
	return list
end

describe("Day", function()
	local ns

	before_each(function()
		ns = Addon.load(Mock)
		Mock.now = localTime(2026, 11, 5, 23, 30)
		Mock.fire("ADDON_LOADED", "Mrglglgl")
		Mock.fire("PLAYER_LOGIN")
	end)

	it("keys a day by the local calendar date", function()
		assert.equals("2026-11-05", ns.Day.key(localTime(2026, 11, 5, 0, 0)))
		assert.equals("2026-11-05", ns.Day.key(localTime(2026, 11, 5, 23, 59)))
		assert.equals("2026-11-06", ns.Day.key(localTime(2026, 11, 6, 0, 0)))
	end)

	it("splits a session that crosses midnight by each event's own timestamp", function()
		Mock.advance(20 * 60) -- 23:50
		Mock.fire("PLAYER_LEVEL_UP", 6)
		Mock.advance(20 * 60) -- 00:10 the next day
		Mock.fire("PLAYER_LEVEL_UP", 7)
		Mock.advance(20 * 60)
		Mock.fire("PLAYER_LOGOUT")

		local days = _G.MrglglglCharDB.days
		assert.same({ "session_start", "level" }, kinds(days["2026-11-05"]))
		assert.same({ "level", "session_end" }, kinds(days["2026-11-06"]))
		assert.equals(6, days["2026-11-05"].ev[2].lv)
		assert.equals(7, days["2026-11-06"].ev[1].lv)
	end)

	it("stamps every event with time, zone, subzone, map and level", function()
		Mock.zone, Mock.subzone, Mock.mapId = "Mulgore", "Red Rocks", 7
		Mock.fire("PLAYER_LEVEL_UP", 6)
		local event = _G.MrglglglCharDB.days["2026-11-05"].ev[2]
		assert.same({ t = Mock.now, k = "level", z = "Mulgore", sz = "Red Rocks", m = 7, lv = 6 }, event)
	end)

	it("records nothing for sessions and levels when those categories are off", function()
		_G.MrglglglDB.settings.record.sessions = false
		_G.MrglglglDB.settings.record.levels = false
		Mock.fire("PLAYER_LEVEL_UP", 6)
		Mock.fire("PLAYER_LOGOUT")
		assert.same({ "session_start" }, kinds(_G.MrglglglCharDB.days["2026-11-05"]))
	end)
end)
