-- Builds the shared SavedVariables fixtures in fixtures/savedvariables/ by playing scripted days
-- through the real addon modules under the API mock, then writing the tables the way the client
-- does. The app's parser and schema tests read these files, so they always match what this addon
-- writes. Regenerate with `make fixtures`; spec/fixtures_spec.lua fails when they are stale.
--
-- Usage, from addon/: luajit tools/fixtures.lua
package.path = "./?.lua;" .. package.path

local Mock = require("spec.support.wow_mock")
local Addon = require("spec.support.addon")
local Serialize = require("spec.support.serialize")

local Fixtures = {}

Fixtures.DIR = "../fixtures/savedvariables/"

local PLAYER = "Player-1-0001"

local function at(month, day, hour, min)
	return os.time({ year = 2026, month = month, day = day, hour = hour, min = min or 0, sec = 0 })
end

local function creature(npcId, spawn)
	return ("Creature-0-1-1-1-%d-%010X"):format(npcId, spawn)
end

local function itemLink(id, name)
	return ("|cffffffff|Hitem:%d::::::::|h[%s]|h|r"):format(id, name)
end

-- One login-to-logout session at time `start`, with the saved tables carried over like the client
-- does between sessions.
local function session(saved, start, play)
	Addon.load(Mock)
	_G.MrglglglDB, _G.MrglglglCharDB = saved.account, saved.character
	Mock.now = start
	Mock.items = {
		[6303] = { name = "Raw Slitherskin Mackerel", quality = 1 },
		[6150] = { name = "Old Boot", quality = 0 },
		[2140] = { name = "Carving Knife", quality = 3 },
		[2770] = { name = "Copper Ore", quality = 1 },
		[2835] = { name = "Rough Stone", quality = 1 },
		[1251] = { name = "Linen Bandage", quality = 1 },
	}
	Mock.spells = { [2575] = "Mining" }
	Mock.skills = {
		{ isHeader = true, name = "Professions" },
		{ skillID = 186, name = "Mining", rank = 40 },
		{ skillID = 393, name = "Skinning", rank = 75 },
		{ isHeader = true, name = "Secondary Skills" },
		{ skillID = 129, name = "First Aid", rank = 30 },
	}
	Mock.zone, Mock.subzone = "Mulgore", "Bloodhoof Village"
	Mock.fire("ADDON_LOADED", "Mrglglgl")
	Mock.fire("PLAYER_LOGIN")
	Mock.fire("PLAYER_ENTERING_WORLD", true, false)
	play()
	Mock.advance(60)
	Mock.fire("PLAYER_LOGOUT")
	saved.account, saved.character = _G.MrglglglDB, _G.MrglglglCharDB
end

local function fish(id, name)
	Mock.fishing, Mock.loot = true, { itemLink(id, name) }
	Mock.fire("LOOT_OPENED", true, false)
	Mock.fire("LOOT_CLOSED")
	Mock.fishing, Mock.loot = false, {}
	Mock.advance(30)
end

local function walk(zone, subzone)
	Mock.zone, Mock.subzone = zone, subzone
	Mock.fire("ZONE_CHANGED_NEW_AREA")
	Mock.advance(90)
end

local function play()
	-- An empty journal from an earlier install: the first session below is not the install
	-- session, so its places are told as discoveries.
	local saved = { character = { schema = 1, char = {}, days = {}, quests = {}, companions = {}, stats = {} } }

	-- Ten days before "today": compacted to its digest by the last logout (retention is 7 days).
	session(saved, at(11, 1, 18), function()
		walk("Mulgore", "Red Cloud Mesa")
		fish(6303, "Raw Slitherskin Mackerel")
		fish(6150, "Old Boot")
		Mock.fire("PARTY_KILL", PLAYER, creature(2958, 1))
		Mock.recap = { { event = "ENVIRONMENTAL_DAMAGE", environmentalType = "Drowning", amount = 44 } }
		Mock.fire("PLAYER_DEAD")
		Mock.advance(2)
		Mock.recap = nil
	end)

	-- A busy day: every event kind.
	session(saved, at(11, 8, 19), function()
		_G.MrglglglDB.settings.retentionDays = 7
		_G.MrglglglDB.settings.screenshots.levelUp = true
		walk("Mulgore", "Red Rocks")
		Mock.fire("UI_INFO_MESSAGE", 1, "Discovered: Thunder Bluff")
		Mock.questDetail = { id = 749, title = "The Ravaged Caravan",
			text = "Some marauding wolves have attacked a caravan near Red Rocks." }
		Mock.fire("QUEST_DETAIL", 0)
		Mock.questLog[749] = { level = 6 }
		Mock.fire("QUEST_ACCEPTED", 749)
		Mock.setUnit("target", { guid = creature(2958, 2), name = "Prairie Wolf", classification = "normal" })
		Mock.fire("PLAYER_TARGET_CHANGED")
		for spawn = 2, 24 do
			Mock.fire("PARTY_KILL", PLAYER, creature(2958, spawn))
		end
		Mock.setUnit("nameplate1", { guid = creature(3035, 1), name = "Flatland Cougar", classification = "elite" })
		Mock.fire("NAME_PLATE_UNIT_ADDED", "nameplate1")
		Mock.fire("PARTY_KILL", PLAYER, creature(3035, 1))
		Mock.fire("PARTY_KILL", PLAYER, creature(732, 1))
		Mock.fire("CHAT_MSG_COMBAT_XP_GAIN", "Murloc Streamrunner dies, you gain 50 experience.")
		Mock.fire("PARTY_KILL", PLAYER, Mock.SECRET)
		Mock.fire("BOSS_KILL", 1144, "Hogger")
		Mock.fire("QUEST_TURNED_IN", 749, 450, 75)
		Mock.fire("PLAYER_LEVEL_UP", 6)
		Mock.advance(1)
		Mock.fire("SCREENSHOT_SUCCEEDED")
		Mock.fire("CHAT_MSG_LOOT", "You receive loot: " .. itemLink(2140, "Carving Knife"), "Wren")
		Mock.groupSize = 2
		Mock.setUnit("party1", { guid = "Player-1-0002", name = "Tamsin", surname = "Reedwater" })
		Mock.fire("GROUP_ROSTER_UPDATE")
		Mock.recap = { { event = "SWING_DAMAGE", sourceName = "Murloc Streamrunner", sourceGUID = creature(732, 2) } }
		Mock.fire("PLAYER_DEAD")
		Mock.advance(2)
		Mock.recap, Mock.groupSize = nil, 0
		-- Professions: a node mined, bandages made, two skill-ups.
		Mock.fire("UNIT_SPELLCAST_SUCCEEDED", "player", "Cast-1", 2575)
		Mock.loot = { itemLink(2770, "Copper Ore"), itemLink(2835, "Rough Stone") }
		Mock.fire("LOOT_OPENED", false, false)
		Mock.fire("LOOT_CLOSED")
		Mock.loot = {}
		Mock.fire("TRADE_SKILL_ITEM_CRAFTED_RESULT", { itemID = 1251, quantity = 5,
			hyperlink = itemLink(1251, "Linen Bandage") })
		Mock.fire("CHAT_MSG_LOOT", "You create: " .. itemLink(1251, "Linen Bandage") .. "x5.", "Wren")
		Mock.skills[2].rank, Mock.skills[5].rank = 41, 35
		Mock.fire("SKILL_LINES_CHANGED")
	end)

	-- Nothing but fishing.
	session(saved, at(11, 9, 20), function()
		fish(6303, "Raw Slitherskin Mackerel")
		fish(6303, "Raw Slitherskin Mackerel")
	end)

	-- Logged in, nothing happened.
	session(saved, at(11, 10, 21), function() end)

	return saved
end

local function schemaZero()
	return {
		schema = 0,
		char = { name = "Wren" },
		days = { ["2026-11-01"] = { ev = {
			{ t = at(11, 1, 18), k = "level", z = "Mulgore", sz = "Red Rocks", lv = 4 },
		} } },
	}
end

local function deepCopy(value)
	if type(value) ~= "table" then
		return value
	end
	local copy = {}
	for key, inner in pairs(value) do
		copy[key] = deepCopy(inner)
	end
	return copy
end

-- Returns { [fileName] = text } for every generated fixture.
function Fixtures.build()
	local saved = play()
	local unknown = deepCopy(saved.character)
	unknown.addedByANewerAddon = { anything = true }
	unknown.days["2026-11-08"].weather = "rain"
	unknown.days["2026-11-08"].ev[1].mood = "cheerful"
	return {
		["synthetic-character.lua"] = Serialize.savedVariable("MrglglglCharDB", saved.character),
		["synthetic-account.lua"] = Serialize.savedVariable("MrglglglDB", saved.account),
		["unknown-fields.lua"] = Serialize.savedVariable("MrglglglCharDB", unknown),
		["schema-0.lua"] = Serialize.savedVariable("MrglglglCharDB", schemaZero()),
	}
end

if arg and arg[0] and arg[0]:match("fixtures%.lua$") then
	for name, text in pairs(Fixtures.build()) do
		local file = assert(io.open(Fixtures.DIR .. name, "w"))
		file:write(text)
		file:close()
		io.write("wrote ", Fixtures.DIR, name, "\n")
	end
end

return Fixtures
