local Mock = require("spec.support.wow_mock")
local Addon = require("spec.support.addon")
local Serialize = require("spec.support.serialize")

local DAY = 86400

-- A heavy day of play: long sessions, a lot of fishing, wide travel, many quests, dozens of
-- creature types killed, deaths, loot, crafting and gathering. Profiles: `extreme` is a stress
-- case nobody plays every day of a year; `heavy` is a long day of a dedicated player.
local PROFILES = {
	extreme = { sessions = 3, subzones = 40, discoveries = 10, quests = 30, fish = 150, deaths = 8, loot = 15,
		group = 6, killTypes = 45, questTexts = 900 },
	heavy = { sessions = 2, subzones = 25, discoveries = 6, quests = 15, fish = 60, deaths = 3, loot = 3,
		group = 4, killTypes = 25, questTexts = 600 },
}

local function heavyDay(t, n, p)
	local ev = {}
	local function add(k, fields)
		local e = { t = t + #ev, k = k, z = "Stranglethorn Vale", sz = "Booty Bay", m = 1434, lv = 40 }
		for key, value in pairs(fields or {}) do
			e[key] = value
		end
		ev[#ev + 1] = e
	end
	for _ = 1, p.sessions do
		add("session_start")
		add("session_end")
	end
	for i = 1, p.subzones do
		add("subzone", { sz = "Subzone " .. i })
	end
	for i = 1, p.discoveries do
		add("discover", { area = "Area " .. n .. "-" .. i, src = "visit" })
	end
	for i = 1, p.quests do
		add("quest_accept", { q = n * 100 + i })
		add("quest_turnin", { q = n * 100 + i, xp = 4500, money = 2500 })
	end
	for i = 1, p.fish do
		add("fish", { item = 6300 + (i % 12), name = "Raw Spotted Yellowtail", junk = i % 9 == 0, boot = i % 50 == 0 })
	end
	for _ = 1, p.deaths do
		add("death", { killer = { name = "Bloodsail Swashbuckler", npc = 1563, murloc = false } })
	end
	for i = 1, p.loot do
		add("loot", { item = 9000 + i, name = "Some Notable Blade of the Tiger", ql = 3, qty = 1 })
	end
	for i = 1, p.group do
		add("group_join", { who = "Companion" .. i .. "-Surname" })
	end
	add("level", { lv = 41 })
	local kills = {}
	for i = 1, p.killTypes do
		kills[1000 + i] = { n = 20 + i, name = "Creature Type Number " .. i, cls = i % 10 == 0 and "elite" or "normal",
			murloc = i % 15 == 0 }
	end
	local skills = {
		[186] = { name = "Mining", cat = "Professions", from = 200, to = 212 },
		[44] = { name = "Axes", cat = "Weapon Skills", from = 180, to = 200 },
	}
	local crafted = { [1251] = { n = 40, name = "Heavy Silk Bandage" } }
	local gathered = { [2772] = { n = 35, name = "Iron Ore", via = "Mining" } }
	return { ev = ev, kills = kills, anonKills = 12, skills = skills, crafted = crafted, gathered = gathered }
end

local function yearSize(profile, keep)
	local p = PROFILES[profile]
		local ns = Addon.load(Mock)
		local now = os.time({ year = 2027, month = 11, day = 6, hour = 12 })
		local journal = { schema = 1, char = { name = "Wren", surname = "Hollowbrook" }, days = {}, quests = {},
			companions = {}, discovered = {}, stats = { boots = 0, murlocsSlain = 0, deathsByMurloc = 0, drownings = 0 } }
		for n = 0, 364 do
			local t = now - n * DAY
			journal.days[os.date("%Y-%m-%d", t)] = heavyDay(t, n, p)
		end
		for q = 1, p.questTexts do
			journal.quests[q] = { title = "A Quest Title of Average Length",
				last = os.date("%Y-%m-%d", now - (q % 365) * DAY),
				text = ("Quest description text of a typical length. "):rep(12) }
		end
		ns.Compact.run(journal, now, keep)
		local bytes = #Serialize.savedVariable("MrglglglCharDB", journal)
		io.stderr:write(("\n  %s year at %d-day retention: %.0f KB"):format(profile, keep, bytes / 1024))
		return bytes
end

-- The guard: a heavy year at the default retention stays under this. Owner decision
-- (2026-11-06): compacted days keep their key events and aggregates, so the budget is 8 MB
-- (about 6.8 MB measured); whether the client loads a file this size quickly is an in-game check.
local HEAVY_DEFAULT_LIMIT = 8 * 1024 * 1024

describe("SavedVariables size", function()
	it("keeps a heavy year at the default 30-day retention under 8 MB", function()
		local bytes = yearSize("heavy", 30)
		assert.is_true(bytes < HEAVY_DEFAULT_LIMIT, ("%.0f KB"):format(bytes / 1024))
	end)

	it("reports a year at every retention setting", function()
		for _, profile in ipairs({ "heavy", "extreme" }) do
			for _, keep in ipairs({ 14, 30, 60, 90 }) do
				yearSize(profile, keep)
			end
		end
		io.stderr:write("\n")
	end)
end)
