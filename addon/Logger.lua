-- Sessions and level-ups.
local _, ns = ...

local Storage, Day = ns.Storage, ns.Day

ns.On("PLAYER_LOGIN", function()
	if Storage.records("sessions") then
		Day.append("session_start")
	end
end)

ns.On("PLAYER_LOGOUT", function()
	if Storage.records("sessions") then
		Day.append("session_end")
	end
end)

ns.On("PLAYER_LEVEL_UP", function(_, level)
	if Storage.records("levels") then
		Day.append("level", { lv = level })
	end
end)
