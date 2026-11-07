local Mock = require("spec.support.wow_mock")
local Addon = require("spec.support.addon")

local function achievements()
	local list = {}
	for _, event in ipairs(_G.MrglglglCharDB.days[os.date("%Y-%m-%d", Mock.now)].ev) do
		if event.k == "achievement" then
			list[#list + 1] = event
		end
	end
	return list
end

describe("Achievements", function()
	local ns
	before_each(function()
		ns = Addon.load(Mock)
		Mock.fire("ADDON_LOADED", "Mrglglgl")
		Mock.fire("PLAYER_LOGIN")
		Mock.fire("PLAYER_ENTERING_WORLD", true, false)
	end)

	it("records a Legacy Challenge completed, with its name, category and points", function()
		Mock.fire("ACHIEVEMENT_EARNED", 855, false)
		local got = achievements()
		assert.equals(1, #got)
		assert.same({ 855, "Explore Moonglade", "Kalimdor", 0 }, { got[1].ach, got[1].name, got[1].cat, got[1].pts })
	end)

	it("leaves out one another character on the account earned first", function()
		Mock.fire("ACHIEVEMENT_EARNED", 855, true)
		assert.same({}, achievements())
	end)

	it("counts and lists them in the digest, keeps them on compaction, and forgets them", function()
		Mock.fire("ACHIEVEMENT_EARNED", 855, false)
		local day = _G.MrglglglCharDB.days[os.date("%Y-%m-%d", Mock.now)]
		local digest = ns.Digest.build(day)
		assert.equals(1, digest.counts.achievements)
		assert.equals("Explore Moonglade", digest.achievements[1].name)
		ns.Forget.category(_G.MrglglglCharDB, "achievements")
		assert.same({}, achievements())
	end)

	it("stays quiet when turned off", function()
		_G.MrglglglDB.settings.record.achievements = false
		Mock.fire("ACHIEVEMENT_EARNED", 855, false)
		assert.same({}, achievements())
	end)
end)
