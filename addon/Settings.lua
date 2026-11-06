-- The settings panel, under Options > AddOns > Mrglglgl. What gets recorded, how long raw events
-- are kept, opt-in screenshots, and forgetting what was already recorded. Plain wording only.
local addonName, ns = ...

local L, Storage, Compat = ns.L, ns.Storage, ns.Compat

local SettingsPanel = {}
ns.Settings = SettingsPanel

-- Categories in panel order; their labels and tooltips come from the locale table.
local RECORD = {
	"sessions", "levels", "quests", "questText", "zones", "kills", "deaths", "fishing", "professions", "loot", "group",
}
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

local function addRecordToggles(layout)
	Compat.AddSettingsHeader(layout, L.SETTINGS_RECORD_HEADER)
	for _, key in ipairs(RECORD) do
		local label = key:gsub("%u", "_%0"):upper()
		Compat.AddCheckbox(category, "MRGLGLGL_RECORD_" .. label, L["CATEGORY_" .. label], L["TIP_" .. label],
			function() return settings().record[key] end,
			function(value) settings().record[key] = value end,
			true)
	end
	Compat.AddDropdown(category, "MRGLGLGL_LOOT_QUALITY", L.SETTINGS_LOOT_QUALITY, L.TIP_LOOT_QUALITY,
		{ { 2, L.QUALITY_UNCOMMON }, { 3, L.QUALITY_RARE }, { 4, L.QUALITY_EPIC } },
		function() return settings().lootQuality end,
		function(value) settings().lootQuality = value end,
		3)
end

local function addRetention(layout)
	Compat.AddSettingsHeader(layout, L.SETTINGS_RETENTION_HEADER)
	Compat.AddDropdown(category, "MRGLGLGL_RETENTION", L.SETTINGS_RETENTION, L.TIP_RETENTION,
		{ { 7, L.DAYS_7 }, { 14, L.DAYS_14 }, { 30, L.DAYS_30 } },
		function() return settings().retentionDays end,
		function(value) settings().retentionDays = value end,
		14)
end

local function addScreenshots(layout)
	if Screenshot == nil then
		return
	end
	Compat.AddSettingsHeader(layout, L.SETTINGS_SCREENSHOTS_HEADER)
	for _, moment in ipairs(SCREENSHOTS) do
		local label = moment:gsub("%u", "_%0"):upper()
		Compat.AddCheckbox(category, "MRGLGLGL_SCREENSHOT_" .. label, L["SHOT_" .. label], L.TIP_SCREENSHOTS,
			function() return settings().screenshots[moment] end,
			function(value) settings().screenshots[moment] = value end,
			false)
	end
end

function SettingsPanel.forget(key)
	local name = L["CATEGORY_" .. key:gsub("%u", "_%0"):upper()]
	local now = time()
	if armed[key] == nil or now - armed[key] > FORGET_CONFIRM_SECONDS then
		armed[key] = now
		say(L.FORGET_CONFIRM:format(FORGET_CONFIRM_SECONDS, name))
		return
	end
	armed[key] = nil
	ns.Forget.category(Storage.journal(), key)
	say(L.FORGET_DONE:format(name))
end

local function addForget(layout)
	Compat.AddSettingsHeader(layout, L.SETTINGS_FORGET_HEADER)
	for _, key in ipairs(RECORD) do
		local label = key:gsub("%u", "_%0"):upper()
		Compat.AddButton(layout, L["CATEGORY_" .. label], L.FORGET_BUTTON,
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
	local layout
	category, layout = Compat.CreateSettingsCategory(L.ADDON_TITLE)
	addRecordToggles(layout)
	addRetention(layout)
	addScreenshots(layout)
	addForget(layout)
	Compat.RegisterSettingsCategory(category)
end)
