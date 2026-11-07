local Mock = require("spec.support.wow_mock")
local Addon = require("spec.support.addon")

-- Death recap events in the shape the live client returned, newest first.
local function drowning()
	return {
		event = "ENVIRONMENTAL_DAMAGE", environmentalType = "Drowning", hideCaster = true,
		destName = "Wren Hollowbrook", destGUID = "Player-1-0001", amount = 44, overkill = 0,
	}
end

local function swing(sourceName, sourceGUID)
	return {
		event = "SWING_DAMAGE", sourceName = sourceName, sourceGUID = sourceGUID,
		destName = "Wren Hollowbrook", destGUID = "Player-1-0001", amount = 5, overkill = 2,
	}
end

local function deaths()
	local list = {}
	for _, event in ipairs(_G.MrglglglCharDB.days[os.date("%Y-%m-%d", Mock.now)].ev) do
		if event.k == "death" then
			list[#list + 1] = event
		end
	end
	return list
end

describe("Deaths", function()
	before_each(function()
		Addon.load(Mock)
		Mock.fire("ADDON_LOADED", "Mrglglgl")
		Mock.fire("PLAYER_LOGIN")
		Mock.fire("PLAYER_ENTERING_WORLD", true, false)
	end)

	it("names a murloc killer with its NPC ID and counts the death", function()
		Mock.recap = { swing("Murloc Streamrunner", "Creature-0-6867-1-2-732-00000000AA"), drowning() }
		Mock.fire("PLAYER_DEAD")
		Mock.advance(1)
		local death = deaths()[1]
		assert.same({ name = "Murloc Streamrunner", npc = 732, murloc = true }, death.killer)
		assert.is_nil(death.env)
		assert.equals(1, _G.MrglglglCharDB.stats.deathsByMurloc)
	end)

	it("marks a death on a hardcore realm as the character's last", function()
		Mock.recap = { drowning() }
		Mock.fire("PLAYER_DEAD")
		Mock.advance(1)
		assert.is_nil(deaths()[1].final)
		_G.C_GameRules = { IsHardcoreActive = function() return true end }
		Mock.fire("PLAYER_DEAD")
		Mock.advance(1)
		assert.is_true(deaths()[2].final)
		_G.C_GameRules = nil
	end)

	it("records drowning as the environment, with no creature", function()
		Mock.recap = { drowning(), swing("Prairie Wolf", "Creature-0-6867-1-2-2958-00000000AA") }
		Mock.fire("PLAYER_DEAD")
		Mock.advance(1)
		local death = deaths()[1]
		assert.equals("Drowning", death.env)
		assert.is_nil(death.killer)
		assert.equals(1, _G.MrglglglCharDB.stats.drownings)
	end)

	it("waits for a recap that fills in a moment after the death", function()
		Mock.fire("PLAYER_DEAD")
		Mock.advance(0.5)
		assert.same({}, deaths())
		Mock.recap = { swing("Prairie Wolf", "Creature-0-6867-1-2-2958-00000000AA") }
		Mock.advance(1)
		assert.equals("Prairie Wolf", deaths()[1].killer.name)
	end)

	it("still records the death when no recap ever arrives", function()
		Mock.fire("PLAYER_DEAD")
		Mock.advance(10)
		assert.equals(1, #deaths())
		assert.is_nil(deaths()[1].killer)
	end)

	it("keeps a death with a withheld killer, without the secret values", function()
		Mock.recap = { swing(Mock.SECRET, Mock.SECRET) }
		Mock.fire("PLAYER_DEAD")
		Mock.advance(1)
		assert.equals(1, #deaths())
		assert.is_nil(deaths()[1].killer)
	end)

	it("records nothing when deaths are off", function()
		_G.MrglglglDB.settings.record.deaths = false
		Mock.recap = { drowning() }
		Mock.fire("PLAYER_DEAD")
		Mock.advance(10)
		assert.same({}, deaths())
		assert.equals(0, _G.MrglglglCharDB.stats.drownings)
	end)
end)
