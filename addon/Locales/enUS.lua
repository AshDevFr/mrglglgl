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

	SETTINGS_UNAVAILABLE = "Settings are not available: the journal was saved by a newer version of the addon.",

	CATEGORY_QUEST_TEXT = "Quest descriptions",

	SETTINGS_RECORD_HEADER = "What gets recorded",
	TIP_SESSIONS = "When you log in and out.",
	TIP_LEVELS = "Each level you gain.",
	TIP_QUESTS = "Quests you accept and turn in, with their titles.",
	TIP_QUEST_TEXT = "Each quest's description, stored once per quest. Most of the journal's size.",
	TIP_ZONES = "Zones and subzones you enter, and places you discover.",
	TIP_KILLS = "What you and your party kill, counted per creature per day. In dungeons and raids only a count.",
	TIP_DEATHS = "Your deaths, and what killed you.",
	TIP_FISHING = "Everything you fish up, junk and boots included.",
	TIP_LOOT = "Items you receive at or above the quality below.",
	TIP_GROUP = "The names of players you group with. These are other people's names.",
	SETTINGS_LOOT_QUALITY = "Notable loot quality",
	TIP_LOOT_QUALITY = "The lowest item quality recorded as notable loot.",
	QUALITY_UNCOMMON = "Uncommon and better",
	QUALITY_RARE = "Rare and better",
	QUALITY_EPIC = "Epic and better",

	SETTINGS_RETENTION_HEADER = "Storage",
	SETTINGS_RETENTION = "Keep full detail for",
	TIP_RETENTION = "Older days keep only their daily summary. The Mrglglgl app keeps the full history once it has seen it.",
	DAYS_7 = "7 days",
	DAYS_14 = "14 days",
	DAYS_30 = "30 days",

	SETTINGS_SCREENSHOTS_HEADER = "Screenshots",
	TIP_SCREENSHOTS = "Takes a game screenshot at this moment, for the app to show with the day. Off by default.",
	SHOT_LEVEL_UP = "On level up",
	SHOT_DEATH = "On death",
	SHOT_BOSS_KILL = "On a boss kill",
	SHOT_FIRST_BOOT = "On your first boot",
	SHOT_FIRST_MURLOC_KILL = "On your first murloc slain",
	SHOT_FIRST_DEATH_BY_MURLOC = "On your first death by murloc",

	SETTINGS_FORGET_HEADER = "Forget recorded data",
	FORGET_BUTTON = "Forget",
	TIP_FORGET = "Deletes what this character has recorded in this category. Press twice to confirm.",
	FORGET_CONFIRM = "Press Forget again within %d seconds to delete everything recorded for: %s.",
	FORGET_DONE = "Forgotten on this character: %s. Copies already saved by the Mrglglgl app are not affected.",

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
