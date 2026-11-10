-- The settings panel, under Options > AddOns > Mrglglgl. What gets recorded, how long raw events
-- are kept, opt-in screenshots, and forgetting what was already recorded. Plain wording only.
local addonName, ns = ...

local L, Storage, Compat = ns.L, ns.Storage, ns.Compat

local SettingsPanel = {}
ns.Settings = SettingsPanel

-- Recording toggles in panel order, under a header each; their labels and tooltips come from the
-- locale table. Chat has its own page, where its channels are.
local RECORD_GROUPS = {
	{ "SETTINGS_GROUP_ADVENTURE", { "sessions", "levels", "quests", "questText", "zones", "travel", "dungeons" } },
	{ "SETTINGS_GROUP_COMBAT", { "kills", "deaths", "duels" } },
	{ "SETTINGS_GROUP_CRAFT", { "fishing", "professions" } },
	{ "SETTINGS_GROUP_ITEMS", { "loot", "gear", "money", "collections" } },
	{ "SETTINGS_GROUP_GROWTH", { "spells", "reputation", "achievements" } },
	{ "SETTINGS_GROUP_PEOPLE", { "group" } },
}
-- Every category a Forget button exists for, chat included.
local RECORD = {}
for _, group in ipairs(RECORD_GROUPS) do
	for _, key in ipairs(group[2]) do
		RECORD[#RECORD + 1] = key
	end
end
RECORD[#RECORD + 1] = "chat"
local SCREENSHOTS = { "levelUp", "death", "bossKill", "firstBoot", "firstMurlocKill", "firstDeathByMurloc" }

-- A forget button must be pressed twice within this many seconds: the client's confirmation
-- dialogs are off limits to addons, and one stray click must not erase a journal.
local FORGET_CONFIRM_SECONDS = 10

local category
local armed = {}

local function say(text)
	print(L.PREFIX .. text)
end

local function settings()
	return Storage.settings()
end

local function labelOf(key)
	return (key:gsub("%u", "_%0"):upper())
end

local function addRecordToggle(page, key, parent, isEnabled)
	local label = labelOf(key)
	return Compat.AddCheckbox(page, "MRGLGLGL_RECORD_" .. label, L["CATEGORY_" .. label], L["TIP_" .. label],
		function() return settings().record[key] end,
		function(value) settings().record[key] = value end,
		key ~= "chat", parent, isEnabled)
end

local function addRecordToggles(page, layout)
	Compat.AddSettingsHeader(layout, L.SETTINGS_RECORD_HEADER)
	for _, group in ipairs(RECORD_GROUPS) do
		Compat.AddSettingsHeader(layout, L[group[1]])
		for _, key in ipairs(group[2]) do
			addRecordToggle(page, key)
			-- The quality applies to loot alone, so it sits right under it.
			if key == "loot" then
				Compat.AddDropdown(page, "MRGLGLGL_LOOT_QUALITY", L.SETTINGS_LOOT_QUALITY, L.TIP_LOOT_QUALITY,
					{ { 2, L.QUALITY_UNCOMMON }, { 3, L.QUALITY_RARE }, { 4, L.QUALITY_EPIC } },
					function() return settings().lootQuality end,
					function(value) settings().lootQuality = value end,
					3)
			end
		end
	end
end

-- The Chat switch first, then each channel twice: the player's own lines, and other players'
-- lines. The channels grey out while Chat is off, where the client can.
local function addChat(page, layout)
	Compat.AddSettingsHeader(layout, L.SETTINGS_CHAT_HEADER)
	local _, master = addRecordToggle(page, "chat")
	local function chatOn() return settings().record.chat end
	for _, channel in ipairs(ns.Chat.CHANNELS) do
		local label = channel:gsub("%u", "_%0"):upper()
		for _, suffix in ipairs({ "", "Others" }) do
			local key = channel .. suffix
			local name = (suffix == "" and L.CHAT_OWN or L.CHAT_OTHERS):format(L["CHANNEL_" .. label])
			Compat.AddCheckbox(page, "MRGLGLGL_CHAT_" .. key:gsub("%u", "_%0"):upper(), name,
				suffix == "" and L.TIP_CHAT_OWN or L.TIP_CHAT_OTHERS,
				function() return settings().chat[key] end,
				function(value) settings().chat[key] = value end,
				suffix == "" and (channel == "say" or channel == "emote" or channel == "yell"),
				master, chatOn)
		end
	end
end

local function addRetention(page, layout)
	Compat.AddSettingsHeader(layout, L.SETTINGS_RETENTION_HEADER)
	Compat.AddDropdown(page, "MRGLGLGL_RETENTION", L.SETTINGS_RETENTION, L.TIP_RETENTION,
		{ { 14, L.DAYS_14 }, { 30, L.DAYS_30 }, { 60, L.DAYS_60 }, { 90, L.DAYS_90 } },
		function() return settings().retentionDays end,
		function(value) settings().retentionDays = value end,
		30)
end

local function addMessages(page, layout)
	Compat.AddSettingsHeader(layout, L.SETTINGS_MESSAGES_HEADER)
	Compat.AddCheckbox(page, "MRGLGLGL_LOGIN_SUMMARY", L.SETTINGS_LOGIN_SUMMARY, L.TIP_LOGIN_SUMMARY,
		function() return settings().loginSummary end,
		function(value) settings().loginSummary = value end,
		false)
end

local function addScreenshots(page, layout)
	if Screenshot == nil then
		return
	end
	Compat.AddSettingsHeader(layout, L.SETTINGS_SCREENSHOTS_HEADER)
	for _, moment in ipairs(SCREENSHOTS) do
		local label = moment:gsub("%u", "_%0"):upper()
		Compat.AddCheckbox(page, "MRGLGLGL_SCREENSHOT_" .. label, L["SHOT_" .. label], L.TIP_SCREENSHOTS,
			function() return settings().screenshots[moment] end,
			function(value) settings().screenshots[moment] = value end,
			false)
	end
end

-- True on the press that goes through. The first press arms the key and warns in colour, since
-- the options panel is open and a plain chat line is easily missed.
local function confirmed(key, warning)
	local now = time()
	if armed[key] == nil or now - armed[key] > FORGET_CONFIRM_SECONDS then
		armed[key] = now
		say(warning)
		return false
	end
	armed[key] = nil
	return true
end

function SettingsPanel.forget(key)
	local name = L["CATEGORY_" .. labelOf(key)]
	if confirmed(key, L.FORGET_CONFIRM:format(FORGET_CONFIRM_SECONDS, name)) then
		ns.Forget.category(Storage.journal(), key)
		say(L.FORGET_DONE:format(name))
	end
end

function SettingsPanel.forgetAll()
	if confirmed("*", L.FORGET_ALL_CONFIRM:format(FORGET_CONFIRM_SECONDS)) then
		for _, key in ipairs(RECORD) do
			ns.Forget.category(Storage.journal(), key)
		end
		say(L.FORGET_ALL_DONE)
	end
end

local function addForget(_, layout)
	Compat.AddSettingsHeader(layout, L.SETTINGS_FORGET_HEADER)
	Compat.AddButton(layout, L.FORGET_ALL, L.FORGET_ALL_BUTTON, SettingsPanel.forgetAll, L.TIP_FORGET_ALL)
	for _, key in ipairs(RECORD) do
		Compat.AddButton(layout, L["CATEGORY_" .. labelOf(key)], L.FORGET_BUTTON,
			function() SettingsPanel.forget(key) end, L.TIP_FORGET)
	end
end

function SettingsPanel.open()
	if category then
		Compat.OpenSettings(category)
	end
end

ns.On("ADDON_LOADED", function(_, loaded)
	if loaded ~= addonName or not Storage.ready or category then
		return
	end
	-- One main page and a page each for the long or occasional parts, listed under Mrglglgl in
	-- Options > AddOns.
	local layout
	category, layout = Compat.CreateSettingsCategory(L.ADDON_TITLE)
	addRecordToggles(category, layout)
	addRetention(category, layout)
	addMessages(category, layout)
	addChat(Compat.CreateSettingsSubcategory(category, layout, L.PAGE_CHAT))
	if Screenshot ~= nil then
		addScreenshots(Compat.CreateSettingsSubcategory(category, layout, L.PAGE_SCREENSHOTS))
	end
	addForget(Compat.CreateSettingsSubcategory(category, layout, L.PAGE_FORGET))
	Compat.RegisterSettingsCategory(category)
end)
