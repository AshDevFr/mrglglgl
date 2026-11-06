-- SavedVariables: account-wide settings (MrglglglDB) and the per-character journal
-- (MrglglglCharDB). docs/schema.md documents every field.
local addonName, ns = ...

local Storage = {}
ns.Storage = Storage

local SETTINGS_SCHEMA = 1

local DEFAULT_SETTINGS = {
	record = {
		sessions = true, levels = true, quests = true, questText = true, zones = true,
		kills = true, deaths = true, fishing = true, loot = true, group = true, professions = true,
	},
	lootQuality = 3,
	retentionDays = 14,
	screenshots = {
		levelUp = false, death = false, bossKill = false,
		firstBoot = false, firstMurlocKill = false, firstDeathByMurloc = false,
	},
}

local DEFAULT_STATS = { boots = 0, murlocsSlain = 0, deathsByMurloc = 0, drownings = 0 }

-- Fills keys missing from target with copies of the defaults, recursively. Existing values win,
-- so a player's settings survive new defaults being added.
local function fillDefaults(target, defaults)
	for key, value in pairs(defaults) do
		if type(value) == "table" then
			if type(target[key]) ~= "table" then
				target[key] = {}
			end
			fillDefaults(target[key], value)
		elseif target[key] == nil then
			target[key] = value
		end
	end
end

-- False until the journal is loaded and usable; stays false for a journal written by a newer addon.
Storage.ready = false

function Storage.settings()
	return MrglglglDB.settings
end

function Storage.journal()
	return MrglglglCharDB
end

-- The single recording gate: every handler calls this first and returns when it is false.
function Storage.records(category)
	return Storage.ready and MrglglglDB.settings.record[category] == true
end

local function loadSettings()
	MrglglglDB = MrglglglDB or {}
	MrglglglDB.schema = MrglglglDB.schema or SETTINGS_SCHEMA
	MrglglglDB.settings = MrglglglDB.settings or {}
	fillDefaults(MrglglglDB.settings, DEFAULT_SETTINGS)
end

local function loadJournal()
	if MrglglglCharDB == nil then
		MrglglglCharDB = { schema = ns.Migrate.CURRENT, char = {}, days = {}, quests = {}, companions = {}, stats = {} }
	elseif not ns.Migrate.run(MrglglglCharDB) then
		return false
	end
	MrglglglCharDB.char = MrglglglCharDB.char or {}
	fillDefaults(MrglglglCharDB.stats, DEFAULT_STATS)
	return true
end

-- Unit information is reliable from PLAYER_LOGIN on. On Forever the second value of UnitFullName
-- is the character's surname, and the realm comes from GetRealmName.
local function recordCharacter()
	local char = MrglglglCharDB.char
	local name, surname = UnitFullName("player")
	local _, class = UnitClass("player")
	char.name = name
	char.surname = surname
	char.realm = GetRealmName()
	char.realmId = GetRealmID()
	char.race = select(2, UnitRace("player"))
	char.class = class
	char.faction = UnitFactionGroup("player")
	char.locale = GetLocale()
end

ns.On("ADDON_LOADED", function(_, loaded)
	if loaded ~= addonName then
		return
	end
	loadSettings()
	Storage.ready = loadJournal()
end)

ns.On("PLAYER_LOGIN", function()
	if Storage.ready then
		recordCharacter()
	end
end)
