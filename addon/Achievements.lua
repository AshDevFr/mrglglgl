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

-- Every challenge the character has completed, kept on the journal rather than its days: those
-- earned before the addon was installed have no day here, and a day made of one old challenge
-- would have a narrator write about a day nobody recorded. Refreshed at each login.
ns.On("PLAYER_ENTERING_WORLD", function(_, isInitialLogin, isReloadingUi)
	if (isInitialLogin or isReloadingUi) and Storage.records("achievements") then
		local completed = Compat.GetCompletedAchievements()
		if next(completed) then
			Storage.journal().challenges = completed
		end
	end
end)
