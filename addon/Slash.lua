-- /mrgl and /mrglglgl: status, settings, help, and debug output for development. The addon says
-- nothing unless asked.
local _, ns = ...

local L, Storage, Day, Digest = ns.L, ns.Storage, ns.Day, ns.Digest

local function say(text)
	print(L.PREFIX .. text)
end

local function any(counts, ...)
	for _, key in ipairs({ ... }) do
		if (counts[key] or 0) > 0 then
			return true
		end
	end
	return false
end

-- One status line per recording category, in the order the settings panel lists them. The third
-- entry says whether the day has anything to show for it; quiet categories are left out.
local STATUS = {
	{ "sessions", function(c) return L.STATUS_SESSIONS:format(c.sessions) end,
		function(c) return any(c, "sessions") end },
	{ "levels", function(c, d) return L.STATUS_LEVELS:format(c.levels, d.levelTo or UnitLevel("player")) end,
		function(c) return any(c, "levels") end },
	{ "quests", function(c) return L.STATUS_QUESTS:format(c.accepted, c.turnedIn) end,
		function(c) return any(c, "accepted", "turnedIn") end },
	{ "zones", function(c, d) return L.STATUS_ZONES:format(#d.zones, c.discoveries) end,
		function(c, d) return #d.zones > 0 or any(c, "discoveries") end },
	{ "kills", function(c) return L.STATUS_KILLS:format(c.kills, c.murlocs, c.anonKills) end,
		function(c) return any(c, "kills") end },
	{ "deaths", function(c) return L.STATUS_DEATHS:format(c.deaths) end,
		function(c) return any(c, "deaths") end },
	{ "duels", function(c) return L.STATUS_DUELS:format(c.duels) end,
		function(c) return any(c, "duels") end },
	{ "fishing", function(c) return L.STATUS_FISHING:format(c.fish, c.junk, c.boots) end,
		function(c) return any(c, "fish", "junk", "boots") end },
	{ "professions", function(c) return L.STATUS_PROFESSIONS:format(c.skillUps, c.crafted, c.gathered) end,
		function(c) return any(c, "skillUps", "crafted", "gathered") end },
	{ "loot", function(c) return L.STATUS_LOOT:format(c.loot) end,
		function(c) return any(c, "loot") end },
	{ "gear", function(c) return L.STATUS_GEAR:format(c.gear) end,
		function(c) return any(c, "gear") end },
	{ "money", function(c)
		return L.STATUS_MONEY:format(ns.Money.format(c.moneyIn), ns.Money.format(c.moneyOut), c.auctions)
	end, function(c) return any(c, "moneyIn", "moneyOut", "auctions") end },
	{ "collections", function(c) return L.STATUS_COLLECTIONS:format(c.collections) end,
		function(c) return any(c, "collections") end },
	{ "spells", function(c) return L.STATUS_SPELLS:format(c.spells) end,
		function(c) return any(c, "spells") end },
	{ "reputation", function(c) return L.STATUS_REPUTATION:format(c.reputation) end,
		function(c) return any(c, "reputation") end },
	{ "achievements", function(c) return L.STATUS_ACHIEVEMENTS:format(c.achievements) end,
		function(c) return any(c, "achievements") end },
	{ "travel", function(c) return L.STATUS_TRAVEL:format(c.travel) end,
		function(c) return any(c, "travel") end },
	{ "dungeons", function(c) return L.STATUS_DUNGEONS:format(c.instances) end,
		function(c) return any(c, "instances") end },
	{ "group", function(c) return L.STATUS_GROUP:format(c.group) end,
		function(c) return any(c, "group") end },
	{ "chat", function(c) return L.STATUS_CHAT:format(c.chat) end,
		function(c) return any(c, "chat") end },
}

local function today()
	local key = Day.key(time())
	return key, Storage.journal().days[key] or {}
end

-- Only what happened is listed, so a quiet day is a short answer. Having logged in is not
-- something to report on its own: the routine lines appear alongside real activity only.
local function status()
	if not Storage.ready then
		say(L.STATUS_UNAVAILABLE)
		return
	end
	local key, day = today()
	local digest = Digest.build(day)
	local lines, off = {}, {}
	local noted = false
	for _, entry in ipairs(STATUS) do
		local category, line, active = entry[1], entry[2], entry[3]
		if not MrglglglDB.settings.record[category] then
			off[#off + 1] = L["CATEGORY_" .. category:upper()]
		elseif active(digest.counts, digest) then
			lines[#lines + 1] = line(digest.counts, digest)
			-- Logging in puts you in a session and a zone; only more than that is news.
			local routine = category == "sessions"
				or (category == "zones" and #digest.zones <= 1 and not any(digest.counts, "discoveries"))
			noted = noted or not routine
		end
	end
	if not MrglglglDB.settings.record.questText then
		off[#off + 1] = L.CATEGORY_QUEST_TEXT
	end
	say(L.STATUS_HEADER:format(key))
	if noted then
		for _, line in ipairs(lines) do
			say(line)
		end
	else
		say(L.STATUS_NOTHING)
	end
	local boots = Storage.journal().stats.boots
	if boots > 0 then
		say(L.STATUS_BOOTS_TOTAL:format(boots))
	end
	if #off > 0 then
		say(L.STATUS_OFF:format(table.concat(off, ", ")))
	end
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
	for _, line in ipairs({ L.HELP_STATUS, L.HELP_SETTINGS }) do
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
