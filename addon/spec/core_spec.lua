local Mock = require("spec.support.wow_mock")
local Addon = require("spec.support.addon")

describe("Core", function()
	local ns

	before_each(function()
		ns = Addon.load(Mock)
	end)

	it("loads every file in the TOC into one namespace", function()
		assert.equals("Mrglglgl", ns.name)
		assert.is_function(ns.On)
		assert.is_table(ns.L)
		assert.is_table(ns.Compat)
	end)

	it("dispatches an event to every handler registered for it, in order", function()
		local calls = {}
		ns.On("PLAYER_LEVEL_UP", function(_, level) calls[#calls + 1] = "a" .. level end)
		ns.On("PLAYER_LEVEL_UP", function(_, level) calls[#calls + 1] = "b" .. level end)
		Mock.fire("PLAYER_LEVEL_UP", 7)
		assert.same({ "a7", "b7" }, calls)
	end)

	it("keeps dispatching when a handler fails, and keeps the error in the journal", function()
		Mock.fire("ADDON_LOADED", "Mrglglgl")
		local after = false
		ns.On("PLAYER_LEVEL_UP", function() error("boom in a handler") end)
		ns.On("PLAYER_LEVEL_UP", function() after = true end)
		Mock.fire("PLAYER_LEVEL_UP", 7)
		Mock.fire("PLAYER_LEVEL_UP", 8)
		assert.is_true(after, "the next handler still ran")
		local errors = _G.MrglglglCharDB.errors
		assert.equals(2, errors.n)
		assert.equals("PLAYER_LEVEL_UP", errors.last.event)
		assert.truthy(errors.last.msg:find("boom in a handler", 1, true))
		assert.equals(Mock.now, errors.last.t)
		local notices = 0
		for _, line in ipairs(Mock.printed) do
			if line:find("hit an error", 1, true) then
				notices = notices + 1
			end
		end
		assert.equals(1, notices, "said once per session, not per error")
	end)

	it("counts an error that happens before the journal is loaded once it is", function()
		ns.On("ADDON_LOADED", function() error("too early") end)
		Mock.fire("ADDON_LOADED", "Mrglglgl")
		Mock.fire("PLAYER_LOGIN")
		assert.equals(1, _G.MrglglglCharDB.errors.n)
	end)

	it("only registers events that a module listens to", function()
		assert.is_false(Mock.isRegistered("PLAYER_REGEN_ENABLED"))
		ns.On("PLAYER_REGEN_ENABLED", function() end)
		assert.is_true(Mock.isRegistered("PLAYER_REGEN_ENABLED"))
	end)
end)

describe("Locale", function()
	it("falls back to the key for a missing string", function()
		local ns = Addon.load(Mock)
		assert.equals("NOT_A_REAL_KEY", ns.L.NOT_A_REAL_KEY)
		assert.equals("Mrglglgl", ns.L.ADDON_TITLE)
	end)

	it("applies a non-English locale only on a client running it", function()
		local ns = Addon.load(Mock)
		ns.RegisterLocale("frFR", { ADDON_TITLE = "Mrglglgl (fr)" })
		assert.equals("Mrglglgl", ns.L.ADDON_TITLE)

		ns = Addon.load(Mock)
		Mock.locale = "frFR"
		ns.RegisterLocale("frFR", { ADDON_TITLE = "Mrglglgl (fr)" })
		assert.equals("Mrglglgl (fr)", ns.L.ADDON_TITLE)
	end)

	it("refuses direct writes, so strings only come from locale files", function()
		local ns = Addon.load(Mock)
		assert.has_error(function() ns.L.ADDON_TITLE = "x" end)
	end)
end)
