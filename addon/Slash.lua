-- /mrgl and /mrglglgl: status, settings, help, and debug output for development. The addon says
-- nothing unless asked.
local _, ns = ...

local L, Storage, Day, Digest = ns.L, ns.Storage, ns.Day, ns.Digest

local function say(text)
	print(L.PREFIX .. text)
end

-- One status line per recording category, in the order the settings panel lists them.
local STATUS = {
	{ "sessions", function(c) return L.STATUS_SESSIONS:format(c.sessions) end },
	{ "levels", function(c, d) return L.STATUS_LEVELS:format(c.levels, d.levelTo or UnitLevel("player")) end },
	{ "quests", function(c) return L.STATUS_QUESTS:format(c.accepted, c.turnedIn) end },
	{ "zones", function(c, d) return L.STATUS_ZONES:format(#d.zones, c.discoveries) end },
	{ "kills", function(c) return L.STATUS_KILLS:format(c.kills, c.murlocs, c.anonKills) end },
	{ "deaths", function(c) return L.STATUS_DEATHS:format(c.deaths) end },
	{ "fishing", function(c) return L.STATUS_FISHING:format(c.fish, c.junk, c.boots) end },
	{ "professions", function(c) return L.STATUS_PROFESSIONS:format(c.skillUps, c.crafted, c.gathered) end },
	{ "loot", function(c) return L.STATUS_LOOT:format(c.loot) end },
	{ "money", function(c)
		return L.STATUS_MONEY:format(ns.Money.format(c.moneyIn), ns.Money.format(c.moneyOut), c.auctions)
	end },
	{ "group", function(c) return L.STATUS_GROUP:format(c.group) end },
	{ "achievements", function(c) return L.STATUS_ACHIEVEMENTS:format(c.achievements) end },
	{ "chat", function(c) return L.STATUS_CHAT:format(c.chat) end },
}

local function today()
	local key = Day.key(time())
	return key, Storage.journal().days[key] or {}
end

local function status()
	if not Storage.ready then
		say(L.STATUS_UNAVAILABLE)
		return
	end
	local key, day = today()
	local digest = Digest.build(day)
	say(L.STATUS_HEADER:format(key))
	for _, entry in ipairs(STATUS) do
		local category, line = entry[1], entry[2]
		if MrglglglDB.settings.record[category] then
			say(line(digest.counts, digest))
		else
			say(L.STATUS_NOT_RECORDED:format(L["CATEGORY_" .. category:upper()]))
		end
	end
	say(L.STATUS_BOOTS_TOTAL:format(Storage.journal().stats.boots))
end

local function describe(value)
	if type(value) ~= "table" then
		return tostring(value)
	end
	local parts = {}
	for key, inner in pairs(value) do
		parts[#parts + 1] = tostring(key) .. "=" .. describe(inner)
	end
	table.sort(parts)
	return "{" .. table.concat(parts, " ") .. "}"
end

local RAW_EVENTS = 20

local function debug(argument)
	if not Storage.ready then
		say(L.STATUS_UNAVAILABLE)
		return
	end
	local _, day = today()
	if argument == "raw" then
		local events = day.ev or {}
		for i = math.max(1, #events - RAW_EVENTS + 1), #events do
			say(describe(events[i]))
		end
		return
	end
	local digest = Digest.build(day)
	say(describe(digest.counts))
	for _, kill in ipairs(digest.kills) do
		say(describe(kill))
	end
end

local function help()
	say(L.HELP_HEADER)
	for _, line in ipairs({ L.HELP_STATUS, L.HELP_SETTINGS, L.HELP_DEBUG }) do
		say(line)
	end
end

local commands = {
	status = status,
	settings = function()
		if ns.Settings and ns.Settings.open then
			ns.Settings.open()
		else
			say(L.SETTINGS_UNAVAILABLE)
		end
	end,
	debug = debug,
	help = help,
}

function ns.HandleSlash(message)
	local command, argument = (message or ""):lower():match("^%s*(%S*)%s*(.-)%s*$")
	local handler = commands[command]
	if command == "" then
		handler = status
	end
	if handler then
		handler(argument)
	else
		help()
	end
end

SLASH_MRGLGLGL1 = "/mrgl"
SLASH_MRGLGLGL2 = "/mrglglgl"
SlashCmdList.MRGLGLGL = ns.HandleSlash
