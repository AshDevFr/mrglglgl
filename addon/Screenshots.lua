-- Opt-in screenshots at chosen moments, all off by default. The addon never learns the file name:
-- it records when it asked, and the app matches the client's file to the event by time.
local _, ns = ...

local Storage, Day, Compat = ns.Storage, ns.Day, ns.Compat

-- Waits so the moment is on screen (the level-up glow, the death screen).
local DELAY = 1
-- At most one screenshot this often, so a wipe does not take five.
local MIN_INTERVAL = 10
-- A success this long after asking is someone else's screenshot (the player's own key press).
local SUCCESS_WINDOW = 5

local lastTaken
local pending

local function shoot(moment)
	if not Storage.ready or not Storage.settings().screenshots[moment] then
		return
	end
	local now = time()
	if lastTaken and now - lastTaken < MIN_INTERVAL then
		return
	end
	lastTaken = now
	local snapshot = Day.snapshot()
	C_Timer.After(DELAY, function()
		if Compat.TakeScreenshot() then
			pending = { moment = moment, snapshot = snapshot, askedAt = time() }
		end
	end)
end

ns.On("SCREENSHOT_SUCCEEDED", function()
	if pending == nil or time() - pending.askedAt > SUCCESS_WINDOW then
		pending = nil
		return
	end
	local snapshot = pending.snapshot
	snapshot.moment = pending.moment
	pending = nil
	Day.append("screenshot", snapshot)
end)

ns.On("SCREENSHOT_FAILED", function()
	pending = nil
end)

ns.On("PLAYER_LEVEL_UP", function() shoot("levelUp") end)
ns.On("PLAYER_DEAD", function() shoot("death") end)
ns.On("BOSS_KILL", function() shoot("bossKill") end)
ns.Listen("firstBoot", function() shoot("firstBoot") end)
ns.Listen("firstMurlocKill", function() shoot("firstMurlocKill") end)
ns.Listen("firstDeathByMurloc", function() shoot("firstDeathByMurloc") end)
