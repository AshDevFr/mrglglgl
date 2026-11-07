-- SavedVariables: account-wide settings (MrglglglDB) and the per-character journal
-- (MrglglglCharDB). docs/schema.md documents every field.
local addonName, ns = ...

local Storage = {}
ns.Storage = Storage

local SETTINGS_SCHEMA = 2
-- The days a player may keep in full; the first is the shortest allowed.
local RETENTION_CHOICES = { 14, 30, 60, 90 }

local DEFAULT_SETTINGS = {
	record = {
		sessions = true, levels = true, quests = true, questText = true, zones = true,
		kills = true, deaths = true, fishing = true, loot = true, group = true, professions = true,
		money = true, chat = false,
	},
	-- Chat channels, once chat is on: the player's own lines (`say`) and others' (`sayOthers`).
	chat = {
		say = true, emote = true, yell = true, party = false, raid = false, guild = false, officer = false,
		whisper = false, channel = false,
		sayOthers = false, emoteOthers = false, yellOthers = false, partyOthers = false, raidOthers = false,
		guildOthers = false, officerOthers = false, whisperOthers = false, channelOthers = false,
	},
	lootQuality = 3,
	retentionDays = 30,
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

-- True for the session in which this character's journal was created: the addon was just
-- installed, or the character is new. Never saved, so it lasts one session.
Storage.fresh = false

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
	local settings = MrglglglDB.settings
	-- Version 1 stored the old 14-day default for everyone, and offered 7 days: the new default
	-- is 30, and 14 the shortest.
	if MrglglglDB.schema < 2 then
		if settings.retentionDays == 14 then
			settings.retentionDays = 30
		elseif settings.retentionDays == 7 then
			settings.retentionDays = 14
		end
		MrglglglDB.schema = 2
	end
	fillDefaults(settings, DEFAULT_SETTINGS)
	-- A value outside the choices (an edited file, an older addon) would compact the wrong days.
	local valid = false
	for _, days in ipairs(RETENTION_CHOICES) do
		valid = valid or settings.retentionDays == days
	end
	if not valid then
		settings.retentionDays = DEFAULT_SETTINGS.retentionDays
	end
end

local function loadJournal()
	if MrglglglCharDB == nil then
		MrglglglCharDB = { schema = ns.Migrate.CURRENT, char = {}, days = {}, quests = {}, companions = {}, stats = {} }
		Storage.fresh = true
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
	ns.FlushErrors()
end)

ns.On("PLAYER_LOGIN", function()
	if Storage.ready then
		recordCharacter()
		-- Which addon and client last wrote the journal, and when: the app shows the addon as
		-- recording, and notices an update that has not been loaded yet.
		local _, build, _, interface = GetBuildInfo()
		MrglglglCharDB.addon = {
			version = ns.Compat.AddonVersion(),
			build = build,
			interface = interface,
			login = time(),
		}
	else
		-- A journal saved by a newer addon is left untouched; without this line the player would
		-- not know nothing is being recorded.
		print(ns.L.STATUS_UNAVAILABLE)
	end
end)
