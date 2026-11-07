local Mock = require("spec.support.wow_mock")
local Addon = require("spec.support.addon")

-- Reads a locale file's table without running the addon: the keys and their strings.
local function entries(path)
	local found
	local chunk = assert(loadfile(path))
	chunk("Mrglglgl", { RegisterLocale = function(_, t) found = t end })
	return found
end

local function specifiers(text)
	local list = {}
	for spec in text:gmatch("%%[%d%.]*[sd]") do
		list[#list + 1] = spec
	end
	return table.concat(list, " ")
end

describe("French", function()
	it("translates every English string, with the same placeholders", function()
		local en, fr = entries("Locales/enUS.lua"), entries("Locales/frFR.lua")
		for key, text in pairs(en) do
			if key ~= "ADDON_TITLE" and key ~= "PREFIX" then
				assert.is_string(fr[key], key)
				assert.equals(specifiers(text), specifiers(fr[key]), key)
			end
		end
		for key in pairs(fr) do
			assert.is_not_nil(en[key], "no English for " .. key)
		end
	end)

	it("speaks French on a French client and English otherwise", function()
		local ns = Addon.load(Mock, "frFR")
		assert.equals("Pêche", ns.L.CATEGORY_FISHING)
		assert.equals("Mrglglgl", ns.L.ADDON_TITLE)
		ns = Addon.load(Mock)
		assert.equals("Fishing", ns.L.CATEGORY_FISHING)
	end)

	it("recognises French gathering spells and murlocs", function()
		local ns = Addon.load(Mock, "frFR")
		for _, spell in ipairs({ "Minage", "Cueillette", "Dépeçage", "Herboristerie" }) do
			assert.is_true(ns.Compat.IsGatheringSpell(spell), spell)
		end
		assert.is_false(ns.Compat.IsGatheringSpell("Pêche"))
		assert.is_true(ns.Compat.IsMurlocName("Murloc des courants"))
	end)
end)
