std = "lua51"
max_line_length = 120
codes = true
exclude_files = { ".rocks/**" }

-- Globals the addon defines: its SavedVariables.
globals = { "MrglglglDB", "MrglglglCharDB" }

-- Client API the addon reads. Each is documented in docs/forever-api.md.
read_globals = {
	"C_DeathRecap", "C_Item", "C_Map", "C_QuestLog", "C_Timer",
	"CreateFrame", "GetBuildInfo", "GetLocale", "GetLootSlotLink", "GetNumGroupMembers", "GetNumLootItems",
	"GetItemInfo", "GetQuestID", "GetQuestText", "GetRealmID", "GetRealmName", "GetSubZoneText", "GetTime", "GetTitleText",
	"GetZoneText", "IsFishingLoot", "IsInGroup", "IsInRaid", "Screenshot", "Settings", "SlashCmdList",
	"UnitClass", "UnitClassification", "UnitFactionGroup", "UnitFullName", "UnitGUID", "UnitLevel",
	"UnitName", "UnitRace", "_G", "date", "issecretvalue", "time",
	"COMBATLOG_XPGAIN_FIRSTPERSON", "ERR_ZONE_EXPLORED", "ERR_ZONE_EXPLORED_XP", "LOOT_ITEM_PUSHED_SELF",
	"LOOT_ITEM_PUSHED_SELF_MULTIPLE", "LOOT_ITEM_SELF", "LOOT_ITEM_SELF_MULTIPLE",
}

files["spec/**"] = {
	std = "+busted",
	globals = { "_G" },
}
files["tools/**"] = { std = "luajit" }
