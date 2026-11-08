local Mock = require("spec.support.wow_mock")
local Addon = require("spec.support.addon")

local function link(id, name)
	return ("|cffffffff|Hitem:%d::::::::|h[%s]|h|r"):format(id, name)
end

local function today()
	return _G.MrglglglCharDB.days[os.date("%Y-%m-%d", Mock.now)] or {}
end

describe("Professions", function()
	before_each(function()
		Addon.load(Mock)
		Mock.skills = {
			{ isHeader = true, name = "Professions" },
			{ skillID = 393, name = "Skinning", rank = 75 },
			{ isHeader = true, name = "Weapon Skills" },
			{ skillID = 44, name = "Axes", rank = 50 },
		}
		Mock.spells = { [2575] = "Mining", [8613] = "Skinning", [133] = "Fireball" }
		Mock.items[2770] = { name = "Copper Ore", quality = 1 }
		Mock.items[2835] = { name = "Rough Stone", quality = 1 }
		Mock.items[1251] = { name = "Linen Bandage", quality = 1 }
		Mock.fire("ADDON_LOADED", "Mrglglgl")
		Mock.fire("PLAYER_LOGIN")
		Mock.fire("PLAYER_ENTERING_WORLD", true, false)
	end)

	it("records one skill record per skill per day, from the first rank to the last", function()
		Mock.skills[2].rank = 76
		Mock.fire("SKILL_LINES_CHANGED")
		Mock.skills[2].rank = 80
		Mock.fire("SKILL_LINES_CHANGED")
		assert.same({ [393] = { name = "Skinning", cat = "Professions", from = 75, to = 80 } }, today().skills)
	end)

	it("records a profession the client lists twice under one skill ID", function()
		-- The skills list can carry the same profession under two IDs (Skinning as 393 and 2947).
		table.insert(Mock.skills, 3, { skillID = 2947, name = "Skinning", rank = 75 })
		Mock.fire("SKILL_LINES_CHANGED")
		Mock.skills[2].rank, Mock.skills[3].rank = 78, 78
		Mock.fire("SKILL_LINES_CHANGED")
		assert.same({ [393] = { name = "Skinning", cat = "Professions", from = 75, to = 78 } }, today().skills)
	end)

	it("collapses weapon skill-ups the same way and keeps their header", function()
		for rank = 51, 60 do
			Mock.skills[4].rank = rank
			Mock.fire("SKILL_LINES_CHANGED")
		end
		assert.same({ name = "Axes", cat = "Weapon Skills", from = 50, to = 60 }, today().skills[44])
	end)

	it("counts a craft once when both the event and the message report it", function()
		Mock.fire("TRADE_SKILL_ITEM_CRAFTED_RESULT", { itemID = 1251, quantity = 5, hyperlink = link(1251, "Linen Bandage") })
		Mock.fire("CHAT_MSG_LOOT", "You create: " .. link(1251, "Linen Bandage") .. "x5.", "Wren")
		assert.same({ [1251] = { n = 5, name = "Linen Bandage" } }, today().crafted)
	end)

	it("counts a craft from the message alone, and once when the event follows it", function()
		Mock.fire("CHAT_MSG_LOOT", "You create: " .. link(1251, "Linen Bandage") .. ".", "Wren")
		Mock.fire("TRADE_SKILL_ITEM_CRAFTED_RESULT", { itemID = 1251, quantity = 1, hyperlink = link(1251, "Linen Bandage") })
		Mock.advance(10)
		Mock.fire("CHAT_MSG_LOOT", "You create: " .. link(1251, "Linen Bandage") .. ".", "Wren")
		assert.equals(2, today().crafted[1251].n)
	end)

	it("records what a gathering cast loots, with the profession", function()
		Mock.fire("UNIT_SPELLCAST_SUCCEEDED", "player", "Cast-1", 2575)
		Mock.loot = { link(2770, "Copper Ore"), link(2835, "Rough Stone") }
		Mock.fire("LOOT_OPENED", false, false)
		Mock.fire("LOOT_CLOSED")
		assert.same({
			[2770] = { n = 1, name = "Copper Ore", via = "Mining" },
			[2835] = { n = 1, name = "Rough Stone", via = "Mining" },
		}, today().gathered)
	end)

	it("does not count loot from a kill, another spell, or a cast too long ago as gathered", function()
		Mock.loot = { link(2770, "Copper Ore") }
		Mock.fire("LOOT_OPENED", false, false)
		Mock.fire("LOOT_CLOSED")
		Mock.fire("UNIT_SPELLCAST_SUCCEEDED", "player", "Cast-2", 133)
		Mock.fire("LOOT_OPENED", false, false)
		Mock.fire("LOOT_CLOSED")
		Mock.fire("UNIT_SPELLCAST_SUCCEEDED", "player", "Cast-3", 2575)
		Mock.advance(30)
		Mock.fire("LOOT_OPENED", false, false)
		Mock.fire("LOOT_CLOSED")
		Mock.fire("UNIT_SPELLCAST_SUCCEEDED", "party1", "Cast-4", 2575)
		Mock.fire("LOOT_OPENED", false, false)
		assert.is_nil(today().gathered)
	end)

	it("shows in the digest and counts toward a busy day", function()
		local ns = Addon.load(Mock)
		local d = ns.Digest.build({ skills = { [393] = { name = "Skinning", from = 75, to = 77 } },
			crafted = { [1251] = { n = 3, name = "Linen Bandage" } } })
		assert.same({ 2, 3, 0 }, { d.counts.skillUps, d.counts.crafted, d.counts.gathered })
		assert.is_false(d.nothingHappened)
	end)

	it("records nothing when professions are off", function()
		_G.MrglglglDB.settings.record.professions = false
		Mock.skills[2].rank = 80
		Mock.fire("SKILL_LINES_CHANGED")
		Mock.fire("TRADE_SKILL_ITEM_CRAFTED_RESULT", { itemID = 1251, quantity = 5, hyperlink = link(1251, "Linen Bandage") })
		Mock.fire("UNIT_SPELLCAST_SUCCEEDED", "player", "Cast-1", 2575)
		Mock.loot = { link(2770, "Copper Ore") }
		Mock.fire("LOOT_OPENED", false, false)
		assert.is_nil(today().skills)
		assert.is_nil(today().crafted)
		assert.is_nil(today().gathered)
	end)

	it("can be forgotten, raw days and digests alike", function()
		local ns = Addon.load(Mock)
		local journal = { days = {
			raw = { crafted = { [1251] = { n = 5, name = "Linen Bandage" } } },
			old = { digest = { counts = { crafted = 5, skillUps = 3, gathered = 2 }, crafted = { {} }, skills = { {} },
				gathered = { {} } } },
		}, stats = {} }
		ns.Forget.category(journal, "professions")
		assert.is_nil(journal.days.raw.crafted)
		assert.equals(0, journal.days.raw.digest.counts.crafted)
		assert.same({ 0, 0, 0 }, { journal.days.old.digest.counts.crafted, journal.days.old.digest.counts.skillUps,
			journal.days.old.digest.counts.gathered })
	end)
end)
