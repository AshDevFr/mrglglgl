local Mock = require("spec.support.wow_mock")
local Addon = require("spec.support.addon")

local function events(kind)
	local list = {}
	for _, day in pairs(_G.MrglglglCharDB.days) do
		for _, event in ipairs(day.ev or {}) do
			if event.k == kind then
				list[#list + 1] = event
			end
		end
	end
	return list
end

local function enter(zone, subzone)
	Mock.zone, Mock.subzone = zone, subzone
	Mock.fire("ZONE_CHANGED")
end

-- Starts a session; with a journal, as a character the addon has recorded before.
local function start(journal, level)
	Addon.load(Mock)
	Mock.units.player.level = level or Mock.units.player.level
	_G.MrglglglCharDB = journal
	Mock.fire("ADDON_LOADED", "Mrglglgl")
	Mock.fire("PLAYER_LOGIN")
	Mock.fire("PLAYER_ENTERING_WORLD", true, false)
end

local function recorded()
	return { schema = 1, char = {}, days = {}, quests = {}, companions = {}, stats = {} }
end

describe("Zones", function()
	before_each(function()
		start(recorded())
	end)

	it("records a zone change once, not on every subzone change inside it", function()
		enter("Mulgore", "Red Rocks")
		enter("Mulgore", "Bael'dun Digsite")
		assert.equals(1, #events("zone"))
		assert.equals("Mulgore", events("zone")[1].z)
	end)

	it("records at most one event per distinct subzone while walking along a border", function()
		for i = 1, 10 do
			enter("Mulgore", i % 2 == 0 and "Red Rocks" or "Bael'dun Digsite")
			Mock.advance(5)
		end
		local subzones = {}
		for _, event in ipairs(events("subzone")) do
			subzones[event.sz] = (subzones[event.sz] or 0) + 1
		end
		assert.same({ ["Red Rocks"] = 1, ["Bael'dun Digsite"] = 1 }, subzones)
	end)

	it("records a subzone again after a quiet minute", function()
		enter("Mulgore", "Red Rocks")
		enter("Mulgore", "Bael'dun Digsite")
		Mock.advance(61)
		enter("Mulgore", "Red Rocks")
		assert.equals(3, #events("subzone"))
	end)

	it("records a discovery from the client's message", function()
		Mock.fire("UI_INFO_MESSAGE", 1, "Discovered: Thunder Bluff")
		local found = events("discover")
		assert.equals(1, #found)
		assert.same({ "Thunder Bluff", "message" }, { found[1].area, found[1].src })
	end)

	it("falls back to the first visit, and never records the same area twice", function()
		enter("Mulgore", "Red Rocks")
		Mock.fire("CHAT_MSG_SYSTEM", "Discovered Red Rocks: 45 experience gained")
		Mock.advance(120)
		enter("Mulgore", "Red Rocks")
		local areas = {}
		for _, event in ipairs(events("discover")) do
			areas[#areas + 1] = event.area .. "/" .. event.src
		end
		table.sort(areas)
		assert.same({ "Mulgore/visit", "Red Rocks/visit" }, areas)
	end)

	it("knows the places of an already played character's first session without telling them as discoveries", function()
		start(nil)
		enter("Mulgore", "Red Rocks")
		Mock.fire("UI_INFO_MESSAGE", 1, "Discovered: Thunder Bluff")
		local areas = {}
		for _, event in ipairs(events("discover")) do
			areas[#areas + 1] = event.area
		end
		assert.same({ "Thunder Bluff" }, areas)
		assert.is_true(_G.MrglglglCharDB.discovered["Red Rocks"])
		assert.is_true(_G.MrglglglCharDB.discovered.Mulgore)
	end)

	it("tells a new level 1 character's first visits as discoveries", function()
		start(nil, 1)
		enter("Mulgore", "Camp Narache")
		assert.equals(2, #events("discover"))
	end)

	it("records nothing when zones are off", function()
		_G.MrglglglDB.settings.record.zones = false
		enter("Mulgore", "Red Rocks")
		Mock.fire("UI_INFO_MESSAGE", 1, "Discovered: Thunder Bluff")
		assert.same({}, events("zone"))
		assert.same({}, events("subzone"))
		assert.same({}, events("discover"))
	end)
end)
