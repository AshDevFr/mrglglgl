-- Sessions and level-ups.
local _, ns = ...

local Storage, Day, Compat = ns.Storage, ns.Day, ns.Compat

-- The balance in copper, on session events, when money is recorded.
local function balance(fields)
	if Storage.records("money") then
		fields = fields or {}
		fields.bal = Compat.GetMoney()
	end
	return fields
end

-- PLAYER_ENTERING_WORLD rather than PLAYER_LOGIN: at login the zone is not known yet, so the
-- session would start nowhere. It also fires on every loading screen; only a login or a UI reload
-- starts a session.
ns.On("PLAYER_ENTERING_WORLD", function(_, isInitialLogin, isReloadingUi)
	if (isInitialLogin or isReloadingUi) and Storage.records("sessions") then
		Day.append("session_start", balance())
	end
end)

ns.On("PLAYER_LOGOUT", function()
	if Storage.records("sessions") then
		Day.append("session_end", balance(Day.lastPlace()))
	end
end)

ns.On("PLAYER_LEVEL_UP", function(_, level)
	if Storage.records("levels") then
		Day.append("level", { lv = level })
	end
end)
