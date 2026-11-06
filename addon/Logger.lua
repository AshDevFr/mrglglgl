-- Sessions and level-ups.
local _, ns = ...

local Storage, Day = ns.Storage, ns.Day

-- PLAYER_ENTERING_WORLD rather than PLAYER_LOGIN: at login the zone is not known yet, so the
-- session would start nowhere. It also fires on every loading screen; only a login or a UI reload
-- starts a session.
ns.On("PLAYER_ENTERING_WORLD", function(_, isInitialLogin, isReloadingUi)
	if (isInitialLogin or isReloadingUi) and Storage.records("sessions") then
		Day.append("session_start")
	end
end)

ns.On("PLAYER_LOGOUT", function()
	if Storage.records("sessions") then
		Day.append("session_end", Day.lastPlace())
	end
end)

ns.On("PLAYER_LEVEL_UP", function(_, level)
	if Storage.records("levels") then
		Day.append("level", { lv = level })
	end
end)
