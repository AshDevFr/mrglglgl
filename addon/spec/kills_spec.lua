local Mock = require("spec.support.wow_mock")
local Addon = require("spec.support.addon")

local PLAYER = "Player-1-0001"
local FRIEND = "Player-1-0002"
local STRANGER = "Player-1-0099"

local function creature(npcId, spawn)
	return ("Creature-0-6867-1-2-%d-%010X"):format(npcId, spawn or 1)
end

local function today()
	return _G.MrglglglCharDB.days[os.date("%Y-%m-%d", Mock.now)] or {}
end

describe("Kills", function()
	local ns

	before_each(function()
		ns = Addon.load(Mock)
		Mock.fire("ADDON_LOADED", "Mrglglgl")
		Mock.fire("PLAYER_LOGIN")
	end)

	it("never listens to the combat log, which addons cannot read on Forever", function()
		assert.is_false(Mock.isRegistered("COMBAT_LOG_EVENT_UNFILTERED"))
	end)

	it("ignores kills by players outside the group without allocating anything", function()
		-- The client runs plain Lua 5.1; LuaJIT's trace compiler would allocate on its own.
		if jit then
			jit.off()
		end
		collectgarbage("stop")
		local before = collectgarbage("count")
		local target = creature(2958)
		for _ = 1, 500 do
			ns.Kills.onPartyKill(STRANGER, target)
		end
		local grown = collectgarbage("count") - before
		collectgarbage("restart")
		if jit then
			jit.on()
		end
		assert.is_nil(today().kills)
		assert.is_true(grown < 1, ("allocated %.2f KB"):format(grown))
	end)

	it("aggregates 23 kills of one creature into a single record", function()
		Mock.setUnit("target", { guid = creature(2958), name = "Prairie Wolf", classification = "normal" })
		Mock.fire("PLAYER_TARGET_CHANGED")
		for i = 1, 23 do
			Mock.fire("PARTY_KILL", PLAYER, creature(2958, i))
		end
		assert.same({ [2958] = { n = 23, name = "Prairie Wolf", cls = "normal", murloc = false } }, today().kills)
	end)

	it("counts a party member's kills", function()
		Mock.groupSize = 1
		Mock.setUnit("party1", { guid = FRIEND })
		Mock.fire("GROUP_ROSTER_UPDATE")
		Mock.fire("PARTY_KILL", FRIEND, creature(2958))
		assert.equals(1, today().kills[2958].n)
	end)

	it("takes classification from nameplates and records unknown when never seen", function()
		Mock.setUnit("nameplate3", { guid = creature(3035), name = "Flatland Cougar", classification = "elite" })
		Mock.fire("NAME_PLATE_UNIT_ADDED", "nameplate3")
		Mock.fire("PARTY_KILL", PLAYER, creature(3035))
		Mock.fire("PARTY_KILL", PLAYER, creature(2959))
		assert.equals("elite", today().kills[3035].cls)
		assert.equals("unknown", today().kills[2959].cls)
	end)

	it("names an unseen creature from the XP message that follows the kill", function()
		Mock.fire("PARTY_KILL", PLAYER, creature(2958))
		Mock.fire("CHAT_MSG_COMBAT_XP_GAIN", "Prairie Wolf dies, you gain 46 experience.")
		assert.equals("Prairie Wolf", today().kills[2958].name)
	end)

	it("does not name a kill from an XP message that comes too late", function()
		Mock.fire("PARTY_KILL", PLAYER, creature(2958))
		Mock.advance(5)
		Mock.fire("CHAT_MSG_COMBAT_XP_GAIN", "Prairie Wolf dies, you gain 46 experience.")
		assert.is_nil(today().kills[2958].name)
	end)

	it("flags murlocs and counts them, including one named only by its XP message", function()
		Mock.setUnit("target", { guid = creature(46), name = "Murloc Forager" })
		Mock.fire("PLAYER_TARGET_CHANGED")
		Mock.fire("PARTY_KILL", PLAYER, creature(46))
		Mock.fire("PARTY_KILL", PLAYER, creature(732))
		Mock.fire("CHAT_MSG_COMBAT_XP_GAIN", "Murloc Streamrunner dies, you gain 50 experience.")
		assert.is_true(today().kills[46].murloc)
		assert.is_true(today().kills[732].murloc)
		assert.equals(2, _G.MrglglglCharDB.stats.murlocsSlain)
	end)

	it("counts a kill anonymously when the client withholds the target", function()
		Mock.fire("PARTY_KILL", PLAYER, Mock.SECRET)
		Mock.fire("PARTY_KILL", PLAYER, Mock.SECRET)
		assert.equals(2, today().anonKills)
		assert.is_nil(today().kills)
	end)

	it("records boss kills", function()
		Mock.fire("BOSS_KILL", 1144, "Hogger")
		local events = today().ev
		local last = events[#events]
		assert.same({ "boss", 1144, "Hogger" }, { last.k, last.enc, last.name })
	end)

	it("records nothing when kills are off", function()
		_G.MrglglglDB.settings.record.kills = false
		Mock.fire("PARTY_KILL", PLAYER, creature(2958))
		Mock.fire("PARTY_KILL", PLAYER, Mock.SECRET)
		Mock.fire("BOSS_KILL", 1144, "Hogger")
		assert.is_nil(today().kills)
		assert.is_nil(today().anonKills)
		assert.equals(0, _G.MrglglglCharDB.stats.murlocsSlain)
		assert.equals(1, #today().ev) -- only the session start
	end)

	it("forgets cached names on a zone change", function()
		Mock.setUnit("target", { guid = creature(2958), name = "Prairie Wolf" })
		Mock.fire("PLAYER_TARGET_CHANGED")
		Mock.fire("ZONE_CHANGED_NEW_AREA")
		Mock.fire("PARTY_KILL", PLAYER, creature(2958))
		assert.is_nil(today().kills[2958].name)
		assert.is_table(ns.Kills)
	end)
end)
