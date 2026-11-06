local Fixtures = require("tools.fixtures")
local Serialize = require("spec.support.serialize")

local DIR = Fixtures.DIR

local EVENT_KINDS = {
	"session_start", "session_end", "level", "zone", "subzone", "discover", "quest_accept", "quest_turnin",
	"death", "boss", "fish", "loot", "group_join", "screenshot",
}

-- The only names fixtures may contain: they are committed to a public repository.
local FICTIONAL = { Wren = true, Hollowbrook = true, ["Test Realm"] = true, ["Tamsin-Reedwater"] = true }

local function read(name)
	local file = assert(io.open(DIR .. name))
	local text = file:read("*a")
	file:close()
	return text
end

local function journal(name)
	return Serialize.load(DIR .. name).MrglglglCharDB
end

describe("Shared fixtures", function()
	it("match what the addon generates today", function()
		for name, text in pairs(Fixtures.build()) do
			assert.equals(text, read(name), name .. " is stale: run `make fixtures`")
		end
	end)

	it("cover every event kind between them", function()
		local seen = {}
		for _, name in ipairs({ "synthetic-character.lua", "real-session.lua" }) do
			for _, day in pairs(journal(name).days) do
				for _, event in ipairs(day.ev or {}) do
					seen[event.k] = true
				end
			end
		end
		for _, kind in ipairs(EVENT_KINDS) do
			assert.is_true(seen[kind] == true, "no fixture has a " .. kind .. " event")
		end
	end)

	it("include a compacted day, a day of only fishing and a day where nothing happened", function()
		local days = journal("synthetic-character.lua").days
		assert.is_nil(days["2026-11-01"].ev)
		assert.is_table(days["2026-11-01"].digest)
		assert.is_true(days["2026-11-09"].digest.onlyFished)
		assert.is_true(days["2026-11-10"].digest.nothingHappened)
	end)

	it("contain only fictional character, realm and companion names", function()
		for _, name in ipairs({ "synthetic-character.lua", "real-session.lua", "unknown-fields.lua", "schema-0.lua" }) do
			local j = journal(name)
			for _, value in ipairs({ j.char.name, j.char.surname, j.char.realm }) do
				assert.is_true(FICTIONAL[value] == true, name .. " contains a non-fictional name")
			end
			for who in pairs(j.companions or {}) do
				assert.is_true(FICTIONAL[who] == true, name .. " contains a non-fictional companion")
			end
		end
	end)

	it("are written the way the client writes SavedVariables", function()
		for _, name in ipairs({ "synthetic-character.lua", "real-session.lua" }) do
			local text = read(name)
			assert.equals("\nMrglglglCharDB = {\n", text:sub(1, 20), name)
			assert.is_nil(text:find("\n[ \t]+%S"), name .. " is indented")
			assert.is_nil(text:find("%-%- %[%d+%]"), name .. " has index comments")
		end
	end)

	it("include an older schema and fields from a newer addon", function()
		assert.equals(0, journal("schema-0.lua").schema)
		assert.is_table(journal("unknown-fields.lua").addedByANewerAddon)
	end)
end)
