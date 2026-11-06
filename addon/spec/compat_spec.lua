local Mock = require("spec.support.wow_mock")
local Addon = require("spec.support.addon")

describe("Compat", function()
	local Compat

	before_each(function()
		Compat = Addon.load(Mock).Compat
	end)

	it("detects Forever by interface number", function()
		assert.is_true(Compat.IsForever())
		Mock.buildInfo = { "1.15.8", "1", "x", 11508 }
		assert.is_false(Compat.IsForever())
	end)

	it("extracts the NPC ID from a readable creature GUID", function()
		assert.equals(2958, Compat.NpcIdFromGuid("Creature-0-6867-1-2-2958-381DA95EFA"))
	end)

	it("returns no NPC ID for players, secret GUIDs or nil", function()
		assert.is_nil(Compat.NpcIdFromGuid("Player-1-0000ABCD"))
		assert.is_nil(Compat.NpcIdFromGuid(Mock.SECRET))
		assert.is_nil(Compat.NpcIdFromGuid(nil))
	end)

	it("parses single and multiple loot messages from the client's own format strings", function()
		local link = "|cff9d9d9d|Hitem:6150::::::::|h[Old Boot]|h|r"
		assert.same({ link, 1 }, { Compat.ParseLootMessage("You receive loot: " .. link) })
		assert.same({ link, 3 }, { Compat.ParseLootMessage("You receive loot: " .. link .. "x3") })
		assert.same({ link, 1 }, { Compat.ParseLootMessage("You receive item: " .. link) })
		assert.is_nil(Compat.ParseLootMessage("Someone receives loot: " .. link))
		assert.is_nil(Compat.ParseLootMessage(Mock.SECRET))
	end)

	it("reads the area name from both discovery message forms", function()
		assert.equals("Brackwell Pumpkin Patch", Compat.IsDiscoveryMessage("Discovered: Brackwell Pumpkin Patch"))
		assert.equals("Goldshire", Compat.IsDiscoveryMessage("Discovered Goldshire: 45 experience gained"))
		assert.is_nil(Compat.IsDiscoveryMessage("You are hungry."))
	end)

	it("recognises murlocs by name, and never a secret name", function()
		assert.is_true(Compat.IsMurlocName("Murloc Streamrunner"))
		assert.is_false(Compat.IsMurlocName("Prairie Wolf"))
		assert.is_false(Compat.IsMurlocName(Mock.SECRET))
	end)

	it("counts a poor-quality boot as a boot, and waits when the item is not cached", function()
		Mock.items[6150] = { name = "Old Boot", quality = 0 }
		Mock.items[6303] = { name = "Raw Slitherskin Mackerel", quality = 1 }
		assert.is_true(Compat.IsBootCatch("|Hitem:6150::|h[Old Boot]|h"))
		assert.is_false(Compat.IsBootCatch("|Hitem:6303::|h[Raw Slitherskin Mackerel]|h"))
		assert.is_nil(Compat.IsBootCatch("|Hitem:99999::|h[Unknown]|h"))
	end)

	it("takes a screenshot through the client", function()
		assert.is_true(Compat.TakeScreenshot())
		assert.equals(1, Mock.screenshots)
	end)
end)
