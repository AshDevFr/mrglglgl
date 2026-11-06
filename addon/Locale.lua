-- User-facing strings. Locales/enUS.lua fills the table; a locale file loaded after it may
-- override any key for its own client locale. A key with no translation falls back to the key
-- itself, so a missing string shows up as readable English rather than an error.
local _, ns = ...

local strings = {}

ns.L = setmetatable({}, {
	__index = function(_, key)
		local value = strings[key]
		if value == nil then
			return key
		end
		return value
	end,
	__newindex = function()
		error("write locale strings with ns.RegisterLocale", 2)
	end,
})

-- Adds strings for a client locale. enUS is always applied as the base; other locales only apply
-- on a client running that locale.
function ns.RegisterLocale(locale, entries)
	if locale ~= "enUS" and locale ~= GetLocale() then
		return
	end
	for key, value in pairs(entries) do
		strings[key] = value
	end
end
