local Mock = require("spec.support.wow_mock")
local Addon = require("spec.support.addon")

local function events(kind)
	local list = {}
	local day = _G.MrglglglCharDB.days[os.date("%Y-%m-%d", Mock.now)]
	for _, event in ipairs(day and day.ev or {}) do
		if event.k == kind then
			list[#list + 1] = event
		end
	end
	return list
end

local function link(id, name)
	return "|cff1eff00|Hitem:" .. id .. "::::::::10:::::|h[" .. name .. "]|h|r"
end

describe("Progress", function()
	local ns, factions, equipped, levels
	before_each(function()
		ns = Addon.load(Mock)
		factions = { { name = "Thunder Bluff", reaction = 4 }, { name = "Horde", isHeader = true } }
		equipped, levels = {}, {}
		_G.C_Reputation = {
			GetNumFactions = function() return #factions end,
			GetFactionDataByIndex = function(i) return factions[i] end,
		}
		_G.FACTION_STANDING_LABEL5 = "Friendly"
		_G.GetInventoryItemLink = function(_, slot) return equipped[slot] end
		_G.C_Item.GetDetailedItemLevelInfo = function(l) return levels[tonumber(l:match("item:(%d+)"))] end
		Mock.items[100] = { name = "Worn Shortsword", quality = 1 }
		Mock.items[200] = { name = "Cougar Fang Blade", quality = 2 }
		levels[100], levels[200] = 2, 12
		equipped[16] = link(100, "Worn Shortsword")
		_G.GetBindLocation = function() return "Camp Taurajo" end
		_G.ERR_NEWTAXIPATH = "New flight path discovered!"
		_G.DUEL_WINNER_KNOCKOUT = "%1$s has defeated %2$s in a duel"
		_G.DUEL_WINNER_RETREAT = "%2$s has fled from %1$s in a duel"
		Mock.spells[768] = "Cat Form"
		Mock.instance = nil
		_G.IsInInstance = function() return Mock.instance ~= nil, Mock.instance and "party" or "none" end
		_G.GetInstanceInfo = function() return Mock.instance end
		Mock.fire("ADDON_LOADED", "Mrglglgl")
		Mock.fire("PLAYER_LOGIN")
		Mock.fire("PLAYER_ENTERING_WORLD", true, false)
	end)

	it("records a spell learned by name, druid forms included", function()
		Mock.fire("LEARNED_SPELL_IN_SKILL_LINE", 768, 2)
		assert.same({ 768, "Cat Form" }, { events("spell")[1].spell, events("spell")[1].name })
	end)

	it("records a reputation standing that rises, not the first sight of a faction", function()
		factions[1] = { name = "Thunder Bluff", reaction = 5 }
		factions[3] = { name = "Orgrimmar", reaction = 4 }
		Mock.fire("UPDATE_FACTION")
		local got = events("rep")
		assert.equals(1, #got)
		assert.same({ "Thunder Bluff", "Friendly" }, { got[1].faction, got[1].standing })
	end)

	it("records gear that beats what the slot held, and not a swap back", function()
		equipped[16] = link(200, "Cougar Fang Blade")
		Mock.fire("PLAYER_EQUIPMENT_CHANGED", 16, true)
		equipped[16] = link(100, "Worn Shortsword")
		Mock.fire("PLAYER_EQUIPMENT_CHANGED", 16, true)
		equipped[16] = link(200, "Cougar Fang Blade")
		Mock.fire("PLAYER_EQUIPMENT_CHANGED", 16, true)
		local got = events("gear")
		assert.equals(1, #got)
		local g = got[1]
		assert.same({ 200, "Cougar Fang Blade", 16, 12, 2 }, { g.item, g.name, g.slot, g.ilvl, g.ql })
	end)

	it("counts a first item in an empty slot as an upgrade", function()
		Mock.items[300] = { name = "Band of Thorns", quality = 2 }
		levels[300] = 8
		equipped[11] = link(300, "Band of Thorns")
		Mock.fire("PLAYER_EQUIPMENT_CHANGED", 11, true)
		assert.equals("Band of Thorns", events("gear")[1].name)
	end)

	it("records the hearthstone's new home and a new flight path", function()
		Mock.subzone = "Camp Taurajo"
		Mock.fire("PLAYER_BINDLOCATION_CHANGED")
		Mock.fire("UI_INFO_MESSAGE", 0, "New flight path discovered!")
		assert.equals("Camp Taurajo", events("bind")[1].area)
		assert.equals("Camp Taurajo", events("flight_path")[1].area)
	end)

	it("records duels won and lost, with the opponent", function()
		Mock.fire("CHAT_MSG_SYSTEM", "Wren has defeated Tamsin in a duel")
		Mock.fire("CHAT_MSG_SYSTEM", "Wren has fled from Grok in a duel")
		Mock.fire("CHAT_MSG_SYSTEM", "Tamsin has defeated Grok in a duel")
		local got = events("duel")
		assert.equals(2, #got)
		assert.same({ true, "Tamsin" }, { got[1].won, got[1].who })
		assert.same({ false, "Grok" }, { got[2].won, got[2].who })
	end)

	it("records entering and leaving a dungeon, with how long it took", function()
		Mock.instance = "Wailing Caverns"
		Mock.fire("PLAYER_ENTERING_WORLD", false, false)
		Mock.advance(1800)
		Mock.instance = nil
		Mock.fire("PLAYER_ENTERING_WORLD", false, false)
		local got = events("instance")
		assert.same({ "enter", "Wailing Caverns", "party" }, { got[1].act, got[1].name, got[1].kind })
		assert.same({ "leave", 1800 }, { got[2].act, got[2].dur })
		local digest = ns.Digest.build(_G.MrglglglCharDB.days[os.date("%Y-%m-%d", Mock.now)])
		assert.equals(1, digest.counts.instances)
	end)

	it("records mounts and pets added when the client has those journals", function()
		_G.C_MountJournal = { GetMountInfoByID = function() return "Brown Kodo" end }
		Mock.fire("NEW_MOUNT_ADDED", 12)
		assert.equals("Brown Kodo", events("mount")[1].name)
	end)

	it("stays quiet for a category turned off, and forgets one", function()
		_G.MrglglglDB.settings.record.spells = false
		Mock.fire("LEARNED_SPELL_IN_SKILL_LINE", 768, 2)
		assert.same({}, events("spell"))
		Mock.fire("CHAT_MSG_SYSTEM", "Wren has defeated Tamsin in a duel")
		ns.Forget.category(_G.MrglglglCharDB, "duels")
		assert.same({}, events("duel"))
	end)
end)
