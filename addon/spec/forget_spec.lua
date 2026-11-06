local Mock = require("spec.support.wow_mock")
local Addon = require("spec.support.addon")

local function journal()
	return {
		days = {
			["2026-11-20"] = {
				ev = {
					{ t = 1, k = "fish", z = "Mulgore", sz = "", lv = 5, item = 6150, name = "Old Boot", junk = true, boot = true },
					{ t = 2, k = "death", z = "Mulgore", sz = "", lv = 5, env = "Drowning" },
					{ t = 3, k = "boss", z = "Mulgore", sz = "", lv = 5, enc = 1, name = "Hogger" },
				},
				kills = { [2958] = { n = 3, name = "Prairie Wolf", cls = "normal", murloc = false } },
				anonKills = 1,
			},
			-- Already compacted: only a digest remains.
			["2026-10-01"] = { digest = {
				v = 1, counts = { kills = 9, murlocs = 2, anonKills = 0, bosses = 0, fish = 4, junk = 1, boots = 1,
					deaths = 1 },
				kills = { { npc = 46, name = "Murloc Forager", n = 2, murloc = true } }, notableKills = {}, bosses = {},
				fish = { { item = 6150, n = 1 } }, deaths = { { env = "Drowning" } }, onlyFished = false,
			} },
		},
		quests = { [749] = { title = "The Ravaged Caravan", text = "Wolves." } },
		companions = { ["Tamsin-Reedwater"] = { first = "2026-10-01", last = "2026-11-20", days = 2 } },
		stats = { boots = 1, murlocsSlain = 2, deathsByMurloc = 0, drownings = 1 },
	}
end

describe("Forget", function()
	local Forget

	before_each(function()
		Forget = Addon.load(Mock).Forget
	end)

	it("removes kills from raw days and compacted digests, and resets the murloc count", function()
		local j = journal()
		Forget.category(j, "kills")
		local raw, old = j.days["2026-11-20"], j.days["2026-10-01"]
		assert.is_nil(raw.kills)
		assert.is_nil(raw.anonKills)
		for _, event in ipairs(raw.ev) do
			assert.not_equals("boss", event.k)
		end
		assert.equals(0, raw.digest.counts.kills)
		assert.same({}, old.digest.kills)
		assert.equals(0, old.digest.counts.kills)
		assert.equals(0, j.stats.murlocsSlain)
		-- Other categories untouched.
		assert.equals(1, raw.digest.counts.fish)
		assert.equals(4, old.digest.counts.fish)
		assert.equals(1, j.stats.boots)
	end)

	it("removes fishing everywhere and resets the boot count", function()
		local j = journal()
		Forget.category(j, "fishing")
		assert.equals(0, j.days["2026-11-20"].digest.counts.fish)
		assert.same({}, j.days["2026-10-01"].digest.fish)
		assert.equals(0, j.stats.boots)
		assert.equals(1, j.days["2026-11-20"].digest.counts.deaths)
	end)

	it("drops quest descriptions but keeps titles for quest text", function()
		local j = journal()
		Forget.category(j, "questText")
		assert.same({ title = "The Ravaged Caravan" }, j.quests[749])
	end)

	it("drops companions for group members", function()
		local j = journal()
		Forget.category(j, "group")
		assert.same({}, j.companions)
	end)

	it("refuses an unknown category", function()
		assert.has_error(function() Forget.category(journal(), "everything") end)
	end)
end)
