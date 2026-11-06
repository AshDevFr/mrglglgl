local Mock = require("spec.support.wow_mock")
local Addon = require("spec.support.addon")

-- A journal as schema 0 wrote it: days only, no quests, companions or stats.
local function schemaZero()
	return {
		schema = 0,
		char = { name = "Wren" },
		days = {
			["2026-11-01"] = {
				ev = { { t = 1793520000, k = "level", z = "Mulgore", sz = "Red Rocks", lv = 4 } },
				kills = { [2958] = { n = 3, name = "Prairie Wolf", cls = "normal", murloc = false } },
			},
		},
	}
end

describe("Migrate", function()
	local ns

	before_each(function()
		ns = Addon.load(Mock)
	end)

	it("upgrades a schema 0 journal to the current schema without losing data", function()
		local db = schemaZero()
		assert.is_true(ns.Migrate.run(db))
		assert.equals(ns.Migrate.CURRENT, db.schema)
		assert.same(schemaZero().days, db.days)
		assert.same({}, db.quests)
		assert.same({}, db.companions)
		assert.is_table(db.stats)
	end)

	it("is a no-op on a journal already at the current schema", function()
		local db = schemaZero()
		ns.Migrate.run(db)
		db.quests[54] = { title = "A quest" }
		local before = db.quests
		assert.is_true(ns.Migrate.run(db))
		assert.equals(before, db.quests)
		assert.equals(ns.Migrate.CURRENT, db.schema)
	end)

	it("treats a journal with no schema field as schema 0", function()
		local db = schemaZero()
		db.schema = nil
		assert.is_true(ns.Migrate.run(db))
		assert.equals(ns.Migrate.CURRENT, db.schema)
	end)

	it("refuses a journal from a newer addon", function()
		local db = { schema = ns.Migrate.CURRENT + 1 }
		assert.is_false(ns.Migrate.run(db))
		assert.equals(ns.Migrate.CURRENT + 1, db.schema)
	end)

	it("migrates the loaded journal once at ADDON_LOADED", function()
		_G.MrglglglCharDB = schemaZero()
		Mock.fire("ADDON_LOADED", "Mrglglgl")
		assert.equals(ns.Migrate.CURRENT, _G.MrglglglCharDB.schema)
		assert.same(schemaZero().days, _G.MrglglglCharDB.days)
		assert.is_true(ns.Storage.ready)
	end)
end)
