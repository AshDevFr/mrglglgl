local Mock = require("spec.support.wow_mock")
local Addon = require("spec.support.addon")

local function joins()
	local list = {}
	for _, day in pairs(_G.MrglglglCharDB.days) do
		for _, event in ipairs(day.ev or {}) do
			if event.k == "group_join" then
				list[#list + 1] = event.who
			end
		end
	end
	return list
end

-- Seconds grouped with each companion, over every day.
local function grouped()
	local total = {}
	for _, day in pairs(_G.MrglglglCharDB.days) do
		for who, seconds in pairs(day.grp or {}) do
			total[who] = (total[who] or 0) + seconds
		end
	end
	return total
end

local function groupWith(...)
	local members = { ... }
	Mock.groupSize = #members + 1
	for i, member in ipairs(members) do
		Mock.setUnit("party" .. i, member)
	end
	Mock.fire("GROUP_ROSTER_UPDATE")
end

describe("Group", function()
	before_each(function()
		Addon.load(Mock)
		Mock.fire("ADDON_LOADED", "Mrglglgl")
		Mock.fire("PLAYER_LOGIN")
		Mock.fire("PLAYER_ENTERING_WORLD", true, false)
	end)

	it("records a companion by first name and surname", function()
		groupWith({ name = "Tamsin", surname = "Reedwater" })
		assert.same({ "Tamsin-Reedwater" }, joins())
		local today = os.date("%Y-%m-%d", Mock.now)
		assert.same({ first = today, last = today, days = 1 }, _G.MrglglglCharDB.companions["Tamsin-Reedwater"])
	end)

	it("counts each day met once, however often the roster changes", function()
		local tamsin = { name = "Tamsin", surname = "Reedwater" }
		groupWith(tamsin)
		groupWith(tamsin, { name = "Odo", surname = "Brightbarrel" })
		groupWith(tamsin)
		Mock.advance(24 * 3600)
		groupWith(tamsin)
		Mock.advance(24 * 3600)
		groupWith(tamsin)
		assert.equals(3, _G.MrglglglCharDB.companions["Tamsin-Reedwater"].days)
		local count = 0
		for _, who in ipairs(joins()) do
			if who == "Tamsin-Reedwater" then
				count = count + 1
			end
		end
		assert.equals(3, count)
	end)

	it("reads raid members and skips the player", function()
		Mock.inRaid, Mock.groupSize = true, 2
		Mock.setUnit("raid1", Mock.units.player)
		Mock.setUnit("raid2", { name = "Tamsin", surname = "Reedwater" })
		Mock.fire("GROUP_ROSTER_UPDATE")
		assert.same({ "Tamsin-Reedwater" }, joins())
	end)

	it("skips members whose names the client withholds", function()
		groupWith({ name = Mock.SECRET, surname = Mock.SECRET })
		assert.same({}, joins())
	end)

	it("counts how long a companion stayed in the group", function()
		groupWith({ name = "Tamsin", surname = "Reedwater" })
		Mock.advance(3600)
		groupWith()
		assert.same({ ["Tamsin-Reedwater"] = 3600 }, grouped())
	end)

	it("counts each companion apart when one leaves and another stays", function()
		local tamsin = { name = "Tamsin", surname = "Reedwater" }
		groupWith(tamsin, { name = "Odo", surname = "Brightbarrel" })
		Mock.advance(600)
		groupWith(tamsin)
		Mock.advance(1200)
		Mock.fire("PLAYER_LOGOUT")
		assert.same({ ["Tamsin-Reedwater"] = 1800, ["Odo-Brightbarrel"] = 600 }, grouped())
	end)

	it("adds up the time of several groupings the same day", function()
		local tamsin = { name = "Tamsin", surname = "Reedwater" }
		groupWith(tamsin)
		Mock.advance(300)
		groupWith()
		Mock.advance(1000)
		groupWith(tamsin)
		Mock.advance(200)
		Mock.fire("PLAYER_LOGOUT")
		assert.same({ ["Tamsin-Reedwater"] = 500 }, grouped())
	end)

	it("counts no time for members whose names the client withholds", function()
		groupWith({ name = Mock.SECRET, surname = Mock.SECRET })
		Mock.advance(600)
		groupWith()
		assert.same({}, grouped())
	end)

	it("records nothing when group members are off", function()
		_G.MrglglglDB.settings.record.group = false
		groupWith({ name = "Tamsin", surname = "Reedwater" })
		Mock.advance(600)
		Mock.fire("PLAYER_LOGOUT")
		assert.same({}, joins())
		assert.same({}, _G.MrglglglCharDB.companions)
		assert.same({}, grouped())
	end)
end)
