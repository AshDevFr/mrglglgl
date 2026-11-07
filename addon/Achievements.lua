-- Achievements. On Forever these are the Legacy Challenges ("Explore Moonglade", class and
-- profession challenges), achievements worth 0 points; any with points are recorded the same way.
local _, ns = ...

local Storage, Day, Compat = ns.Storage, ns.Day, ns.Compat

local EVENT = "ACHIEVEMENT_EARNED"

if Compat.EventExists(EVENT) then
	-- alreadyEarned: another character on the account earned it first; the account's, not this
	-- character's, day.
	ns.On(EVENT, function(_, id, alreadyEarned)
		if not Storage.records("achievements") or type(id) ~= "number" or alreadyEarned then
			return
		end
		local info = Compat.GetAchievement(id) or {}
		Day.append("achievement", { ach = id, name = info.name, cat = info.category, pts = info.points })
	end)
end
