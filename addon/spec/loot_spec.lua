local Mock = require("spec.support.wow_mock")
local Addon = require("spec.support.addon")

local function link(id, name)
	return ("|cff0070dd|Hitem:%d::::::::|h[%s]|h|r"):format(id, name)
end

local function loot()
	local list = {}
	local day = _G.MrglglglCharDB.days[os.date("%Y-%m-%d", Mock.now)]
	for _, event in ipairs(day and day.ev or {}) do
		if event.k == "loot" then
			list[#list + 1] = event
		end
	end
	return list
end

describe("Loot", function()
	before_each(function()
		Addon.load(Mock)
		Mock.fire("ADDON_LOADED", "Mrglglgl")
		Mock.fire("PLAYER_LOGIN")
		Mock.items[2140] = { name = "Carving Knife", quality = 3 }
		Mock.items[2589] = { name = "Linen Cloth", quality = 1 }
		Mock.items[6303] = { name = "Raw Slitherskin Mackerel", quality = 3 }
	end)

	it("records received items at or above the quality threshold", function()
		Mock.fire("CHAT_MSG_LOOT", "You receive loot: " .. link(2140, "Carving Knife"), "Wren")
		Mock.fire("CHAT_MSG_LOOT", "You receive loot: " .. link(2589, "Linen Cloth") .. "x4", "Wren")
		assert.equals(1, #loot())
		assert.same({ 2140, "Carving Knife", 3, 1 }, { loot()[1].item, loot()[1].name, loot()[1].ql, loot()[1].qty })
	end)

	it("follows the configured threshold", function()
		_G.MrglglglDB.settings.lootQuality = 1
		Mock.fire("CHAT_MSG_LOOT", "You receive loot: " .. link(2589, "Linen Cloth") .. "x4", "Wren")
		assert.equals(4, loot()[1].qty)
	end)

	it("judges an uncached item once its data arrives, with the time and place it was looted", function()
		Mock.items[2140] = nil
		Mock.zone, Mock.subzone = "Mulgore", "Red Rocks"
		local lootedAt = Mock.now
		Mock.fire("CHAT_MSG_LOOT", "You receive loot: " .. link(2140, "Carving Knife"), "Wren")
		assert.same({}, loot())
		Mock.items[2140] = { name = "Carving Knife", quality = 3 }
		Mock.zone, Mock.subzone = "Thunder Bluff", ""
		Mock.advance(3)
		Mock.fire("GET_ITEM_INFO_RECEIVED", 2140, true)
		Mock.fire("GET_ITEM_INFO_RECEIVED", 2140, true)
		assert.equals(1, #loot())
		assert.same({ lootedAt, "Mulgore", "Red Rocks" }, { loot()[1].t, loot()[1].z, loot()[1].sz })
	end)

	it("does not count a fishing catch again as loot", function()
		Mock.fishing, Mock.loot = true, { link(6303, "Raw Slitherskin Mackerel") }
		Mock.fire("LOOT_OPENED", true, false)
		Mock.fire("LOOT_CLOSED")
		Mock.fishing, Mock.loot = false, {}
		Mock.fire("CHAT_MSG_LOOT", "You receive loot: " .. link(6303, "Raw Slitherskin Mackerel"), "Wren")
		assert.same({}, loot())
	end)

	it("reads the loot window instead when the client withholds chat text", function()
		Mock.chatLockdown = true
		Mock.loot = { link(2140, "Carving Knife") }
		Mock.fire("LOOT_OPENED", false, false)
		Mock.fire("CHAT_MSG_LOOT", Mock.SECRET, "Wren")
		assert.equals(1, #loot())
	end)

	it("ignores the loot window when chat text is readable, so nothing is counted twice", function()
		Mock.loot = { link(2140, "Carving Knife") }
		Mock.fire("LOOT_OPENED", false, false)
		Mock.fire("CHAT_MSG_LOOT", "You receive loot: " .. link(2140, "Carving Knife"), "Wren")
		assert.equals(1, #loot())
	end)

	it("records nothing when loot is off", function()
		_G.MrglglglDB.settings.record.loot = false
		Mock.fire("CHAT_MSG_LOOT", "You receive loot: " .. link(2140, "Carving Knife"), "Wren")
		assert.same({}, loot())
	end)
end)
