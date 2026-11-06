local Mock = require("spec.support.wow_mock")
local Addon = require("spec.support.addon")

local DAY = 86400

local function noon(year, month, day)
	return os.time({ year = year, month = month, day = day, hour = 12 })
end

-- A journal with one raw day for each of the last `count` days, today included.
local function journalWithDays(now, count)
	local journal = { days = {} }
	for i = 0, count - 1 do
		local t = now - i * DAY
		journal.days[os.date("%Y-%m-%d", t)] = {
			ev = { { t = t, k = "fish", z = "Mulgore", sz = "", lv = 5, item = 6303, name = "Fish" } },
			kills = { [2958] = { n = i + 1, name = "Prairie Wolf", cls = "normal", murloc = false } },
		}
	end
	return journal
end

local function deepCopy(value)
	if type(value) ~= "table" then
		return value
	end
	local copy = {}
	for k, v in pairs(value) do
		copy[k] = deepCopy(v)
	end
	return copy
end

describe("Compact", function()
	local ns

	before_each(function()
		ns = Addon.load(Mock)
	end)

	for _, keep in ipairs({ 7, 14, 30 }) do
		it(("keeps %d days raw and compacts the rest, digests unchanged"):format(keep), function()
			local now = noon(2026, 11, 20)
			local journal = journalWithDays(now, 40)
			local expected = {}
			for key, day in pairs(journal.days) do
				expected[key] = ns.Digest.build(deepCopy(day))
			end

			assert.equals(40 - keep, ns.Compact.run(journal, now, keep))

			local raw = 0
			for key, day in pairs(journal.days) do
				assert.same(expected[key], day.digest, key)
				if day.ev then
					raw = raw + 1
				else
					assert.is_nil(day.kills, key)
				end
			end
			assert.equals(keep, raw)
			assert.is_table(journal.days[os.date("%Y-%m-%d", now)].ev)
			assert.is_nil(journal.days[os.date("%Y-%m-%d", now - keep * DAY)].ev)
		end)
	end

	it("leaves a compacted day's digest alone on later runs", function()
		local now = noon(2026, 11, 20)
		local journal = journalWithDays(now, 20)
		ns.Compact.run(journal, now, 14)
		local old = os.date("%Y-%m-%d", now - 19 * DAY)
		local digest = journal.days[old].digest
		assert.equals(0, ns.Compact.run(journal, now, 14))
		-- A day later exactly one more day leaves the window.
		assert.equals(1, ns.Compact.run(journal, now + DAY, 14))
		assert.equals(digest, journal.days[old].digest)
	end)

	it("drops quest descriptions outside the window and keeps titles", function()
		local now = noon(2026, 11, 20)
		local journal = { days = {}, quests = {
			[1] = { title = "Recent", text = "Kept.", last = os.date("%Y-%m-%d", now - 3 * DAY) },
			[2] = { title = "Old", text = "Dropped.", last = os.date("%Y-%m-%d", now - 40 * DAY) },
			[3] = { title = "Unknown age", text = "Dropped." },
		} }
		ns.Compact.run(journal, now, 14)
		assert.same({ "Kept.", nil, nil }, { journal.quests[1].text, journal.quests[2].text, journal.quests[3].text })
		assert.same({ "Recent", "Old", "Unknown age" }, { journal.quests[1].title, journal.quests[2].title,
			journal.quests[3].title })
	end)

	it("thins the digests of days older than 90 days to their highlights, counts kept", function()
		local now = noon(2026, 11, 20)
		local journal = journalWithDays(now, 120)
		local old = journal.days[os.date("%Y-%m-%d", now - 100 * DAY)]
		for i = 1, 10 do
			old.ev[#old.ev + 1] = { t = 0, k = "loot", z = "Mulgore", sz = "", lv = 5, item = 9000 + i, name = "Item", ql = 3 }
		end
		ns.Compact.run(journal, now, 14)
		local d = old.digest
		assert.is_true(d.thin)
		assert.equals(3, #d.loot)
		assert.equals(10, d.counts.loot)
		assert.same({}, d.subzones)
		local recent = journal.days[os.date("%Y-%m-%d", now - 30 * DAY)].digest
		assert.is_nil(recent.thin)
	end)

	it("runs at logout with the retention setting, after the session end is recorded", function()
		Mock.now = noon(2026, 11, 20)
		Mock.fire("ADDON_LOADED", "Mrglglgl")
		_G.MrglglglDB.settings.retentionDays = 7
		_G.MrglglglCharDB.days[os.date("%Y-%m-%d", Mock.now - 10 * DAY)] = { ev = { { t = 0, k = "level", lv = 4 } } }
		Mock.fire("PLAYER_LOGIN")
		Mock.fire("PLAYER_ENTERING_WORLD", true, false)
		Mock.fire("PLAYER_LOGOUT")
		local today = _G.MrglglglCharDB.days[os.date("%Y-%m-%d", Mock.now)]
		assert.equals(1, today.digest.counts.sessions)
		assert.equals("session_end", today.ev[#today.ev].k)
		assert.is_nil(_G.MrglglglCharDB.days[os.date("%Y-%m-%d", Mock.now - 10 * DAY)].ev)
	end)
end)
