local Mock = require("spec.support.wow_mock")
local Addon = require("spec.support.addon")

local function ev(k, fields)
	local e = { t = 1793938800, k = k, z = "Mulgore", sz = "", lv = 5 }
	for key, value in pairs(fields or {}) do
		e[key] = value
	end
	return e
end

describe("Digest", function()
	local Digest

	before_each(function()
		Digest = Addon.load(Mock).Digest
	end)

	it("summarises a busy day", function()
		local day = {
			ev = {
				ev("session_start"),
				ev("level", { lv = 6 }),
				ev("subzone", { sz = "Red Rocks" }),
				ev("zone", { z = "Thunder Bluff" }),
				ev("discover", { area = "Red Rocks", src = "visit" }),
				ev("quest_accept", { q = 749 }),
				ev("quest_turnin", { q = 749, xp = 450, money = 75 }),
				ev("death", { killer = { name = "Murloc Streamrunner", npc = 732, murloc = true }, sz = "Red Rocks" }),
				ev("fish", { item = 6303, name = "Raw Slitherskin Mackerel", junk = false }),
				ev("fish", { item = 6303, name = "Raw Slitherskin Mackerel", junk = false }),
				ev("fish", { item = 6150, name = "Old Boot", junk = true, boot = true }),
				ev("loot", { item = 2140, name = "Carving Knife", ql = 3, qty = 1 }),
				ev("group_join", { who = "Tamsin-Reedwater" }),
				ev("boss", { enc = 1144, name = "Hogger" }),
			},
			kills = {
				[2958] = { n = 23, name = "Prairie Wolf", cls = "normal", murloc = false },
				[732] = { n = 4, name = "Murloc Streamrunner", cls = "normal", murloc = true },
				[3035] = { n = 1, name = "Flatland Cougar", cls = "elite", murloc = false },
			},
			anonKills = 2,
		}
		local d = Digest.build(day)
		assert.same({
			sessions = 1, levels = 1, accepted = 1, turnedIn = 1, deaths = 1, fish = 3, junk = 1, boots = 1,
			loot = 1, discoveries = 1, bosses = 1, group = 1, screenshots = 0,
			kills = 30, murlocs = 4, anonKills = 2, skillUps = 0, crafted = 0, gathered = 0,
			moneyIn = 0, moneyOut = 0, auctions = 0, chat = 0, achievements = 0,
		}, d.counts)
		assert.same({ "Mulgore", "Thunder Bluff" }, d.zones)
		assert.same({ 5, 6 }, { d.levelFrom, d.levelTo })
		assert.same({ 2958, 732, 3035 }, { d.kills[1].npc, d.kills[2].npc, d.kills[3].npc })
		assert.same({ "Flatland Cougar" }, { d.notableKills[1].name })
		assert.same({ item = 6303, name = "Raw Slitherskin Mackerel", n = 2, junk = false }, d.fish[1])
		assert.equals("Murloc Streamrunner", d.deaths[1].killer.name)
		assert.same({ "Tamsin-Reedwater" }, d.companions)
		assert.same({ "Hogger" }, d.bosses)
		assert.is_false(d.onlyFished)
		assert.is_false(d.nothingHappened)
		assert.equals(Digest.VERSION, d.v)
	end)

	it("caps record lists to the highlights and keeps exact counts", function()
		local day = { ev = {} }
		for i = 1, 35 do
			day.ev[#day.ev + 1] = ev("loot", { item = 9000 + i, name = "Item " .. i, ql = 3, qty = 1 })
			day.ev[#day.ev + 1] = ev("death", { env = "Drowning" })
		end
		local d = Digest.build(day)
		assert.same({ 30, 35, 20, 35 }, { #d.loot, d.counts.loot, #d.deaths, d.counts.deaths })
	end)

	it("flags a day of nothing but fishing", function()
		local fish = ev("fish", { item = 6303, name = "Raw Slitherskin Mackerel" })
		local d = Digest.build({ ev = { ev("session_start"), fish } })
		assert.is_true(d.onlyFished)
		assert.is_false(d.nothingHappened)
	end)

	it("treats an empty day as a valid day where nothing happened", function()
		local d = Digest.build({})
		assert.is_true(d.nothingHappened)
		assert.is_false(d.onlyFished)
		assert.equals(0, d.counts.kills)
	end)

	it("orders tied kills by NPC ID, so the same day always gives the same digest", function()
		local day = { kills = {
			[300] = { n = 2, name = "C", cls = "normal" },
			[100] = { n = 2, name = "A", cls = "normal" },
			[200] = { n = 2, name = "B", cls = "normal" },
		} }
		local d = Digest.build(day)
		assert.same({ 100, 200, 300 }, { d.kills[1].npc, d.kills[2].npc, d.kills[3].npc })
	end)
end)
