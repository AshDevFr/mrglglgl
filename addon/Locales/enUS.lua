local _, ns = ...

ns.RegisterLocale("enUS", {
	ADDON_TITLE = "Mrglglgl",
	PREFIX = "|cff3fbfbfMrglglgl:|r ",

	CATEGORY_SESSIONS = "Sessions",
	CATEGORY_LEVELS = "Levels",
	CATEGORY_QUESTS = "Quests",
	CATEGORY_ZONES = "Zones",
	CATEGORY_KILLS = "Kills",
	CATEGORY_DEATHS = "Deaths",
	CATEGORY_FISHING = "Fishing",
	CATEGORY_LOOT = "Notable loot",
	CATEGORY_GROUP = "Group members",

	STATUS_HEADER = "Recorded today (%s):",
	STATUS_SESSIONS = "Sessions: %d",
	STATUS_LEVELS = "Levels gained: %d (now level %d)",
	STATUS_QUESTS = "Quests: %d accepted, %d turned in",
	STATUS_ZONES = "Zones visited: %d, places discovered: %d",
	STATUS_KILLS = "Kills: %d (murlocs: %d, unnamed: %d)",
	STATUS_DEATHS = "Deaths: %d",
	STATUS_FISHING = "Fishing: %d catches, %d junk, %d boots",
	STATUS_LOOT = "Notable loot: %d",
	STATUS_GROUP = "Group members met: %d",
	STATUS_NOT_RECORDED = "%s: not recorded (turned off in settings)",
	STATUS_BOOTS_TOTAL = "Boots kept, all time: %d",
	STATUS_UNAVAILABLE = "The journal is not available: it was saved by a newer version of the addon.",

	SETTINGS_UNAVAILABLE = "Settings are not available yet.",

	HELP_HEADER = "Commands:",
	HELP_STATUS = "/mrgl status: what has been recorded today",
	HELP_SETTINGS = "/mrgl settings: choose what gets recorded",
	HELP_DEBUG = "/mrgl debug [raw]: today's digest, or its last raw events",

	-- Murloc names for identifying murlocs where an NPC ID is not available. Lowercase,
	-- separated by "|"; matched against whole creature names.
	MURLOC_NAME_HINTS = "murloc|mrgl",
	-- Item names that count as a boot when fished up as junk. Lowercase, "|"-separated.
	BOOT_NAME_HINTS = "boot",
})
