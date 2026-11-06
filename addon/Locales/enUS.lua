local _, ns = ...

ns.RegisterLocale("enUS", {
	ADDON_TITLE = "Mrglglgl",
	-- Murloc names for identifying murlocs where an NPC ID is not available. Lowercase,
	-- separated by "|"; matched against whole creature names.
	MURLOC_NAME_HINTS = "murloc|mrgl",
	-- Item names that count as a boot when fished up as junk. Lowercase, "|"-separated.
	BOOT_NAME_HINTS = "boot",
})
