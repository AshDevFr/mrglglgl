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

	it("only registers events that a module listens to", function()
		assert.is_false(Mock.isRegistered("PLAYER_LEVEL_UP"))
		ns.On("PLAYER_LEVEL_UP", function() end)
		assert.is_true(Mock.isRegistered("PLAYER_LEVEL_UP"))
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
