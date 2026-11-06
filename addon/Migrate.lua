-- Ordered migrations for the per-character journal. migrations[n] upgrades a journal from schema
-- n - 1 to schema n. Additive fields do not need a migration; renames, removals and changes of
-- meaning do (docs/schema.md).
local _, ns = ...

local Migrate = {}
ns.Migrate = Migrate

Migrate.CURRENT = 1

local migrations = {
	-- Schema 0 predates quest text, companions and stats.
	[1] = function(db)
		db.days = db.days or {}
		db.quests = db.quests or {}
		db.companions = db.companions or {}
		db.stats = db.stats or {}
	end,
}

-- Upgrades db in place to the current schema. Returns false, leaving db untouched, when db was
-- written by a newer addon: rewriting it in an older shape would lose data.
function Migrate.run(db)
	local schema = db.schema or 0
	if schema > Migrate.CURRENT then
		return false
	end
	for target = schema + 1, Migrate.CURRENT do
		migrations[target](db)
		db.schema = target
	end
	return true
end
