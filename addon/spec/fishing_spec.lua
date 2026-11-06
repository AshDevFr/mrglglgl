local Mock = require("spec.support.wow_mock")
local Addon = require("spec.support.addon")

local function link(id, name)
	return ("|cffffffff|Hitem:%d::::::::|h[%s]|h|r"):format(id, name)
end

local function catches()
	local list = {}
	for _, event in ipairs(_G.MrglglglCharDB.days[os.date("%Y-%m-%d", Mock.now)].ev) do
		if event.k == "fish" then
			list[#list + 1] = event
		end
	end
	return list
end

local function fish(...)
	Mock.fishing, Mock.loot = true, { ... }
	Mock.fire("LOOT_OPENED", true, false)
	Mock.fire("LOOT_CLOSED")
	Mock.fishing, Mock.loot = false, {}
end

describe("Fishing", function()
	before_each(function()
		Addon.load(Mock)
		Mock.fire("ADDON_LOADED", "Mrglglgl")
		Mock.fire("PLAYER_LOGIN")
		Mock.fire("PLAYER_ENTERING_WORLD", true, false)
		Mock.items[6303] = { name = "Raw Slitherskin Mackerel", quality = 1 }
		Mock.items[6150] = { name = "Old Boot", quality = 0 }
		Mock.items[6149] = { name = "Rusty Hook", quality = 0 }
	end)

	it("records a fish with its name", function()
		fish(link(6303, "Raw Slitherskin Mackerel"))
		local catch = catches()[1]
		assert.same({ 6303, "Raw Slitherskin Mackerel", false }, { catch.item, catch.name, catch.junk })
		assert.is_nil(catch.boot)
	end)

	it("counts a junk boot as junk and as a boot", function()
		fish(link(6150, "Old Boot"))
		local catch = catches()[1]
		assert.is_true(catch.junk)
		assert.is_true(catch.boot)
		assert.equals(1, _G.MrglglglCharDB.stats.boots)
	end)

	it("counts other junk as junk but not as a boot", function()
		fish(link(6149, "Rusty Hook"))
		assert.is_true(catches()[1].junk)
		assert.is_nil(catches()[1].boot)
		assert.equals(0, _G.MrglglglCharDB.stats.boots)
	end)

	it("completes an uncached catch once its item data arrives, and only once", function()
		Mock.items[6150] = nil
		fish(link(6150, "Old Boot"))
		assert.is_nil(catches()[1].junk)
		Mock.items[6150] = { name = "Old Boot", quality = 0 }
		Mock.fire("GET_ITEM_INFO_RECEIVED", 6150, true)
		Mock.fire("GET_ITEM_INFO_RECEIVED", 6150, true)
		assert.equals(1, #catches())
		assert.is_true(catches()[1].boot)
		assert.equals(1, _G.MrglglglCharDB.stats.boots)
	end)

	it("counts a catch once when the client reports the same loot window twice", function()
		Mock.fishing, Mock.loot = true, { link(6303, "Raw Slitherskin Mackerel") }
		Mock.fire("LOOT_OPENED", true, false)
		Mock.fire("LOOT_OPENED", true, false)
		Mock.fire("LOOT_CLOSED")
		assert.equals(1, #catches())
	end)

	it("ignores loot windows that are not fishing", function()
		Mock.loot = { link(6303, "Raw Slitherskin Mackerel") }
		Mock.fire("LOOT_OPENED", false, false)
		assert.same({}, catches())
	end)

	it("records nothing when fishing is off", function()
		_G.MrglglglDB.settings.record.fishing = false
		fish(link(6150, "Old Boot"))
		assert.same({}, catches())
		assert.equals(0, _G.MrglglglCharDB.stats.boots)
	end)
end)
