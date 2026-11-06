-- Bounded storage: every day with raw events gets a fresh digest, and days older than the
-- retention setting then drop their raw events, keeping only the digest. Runs at login and
-- logout. The app backs up and archives full history, so dropping raw events here loses nothing
-- the app has already seen.
local _, ns = ...

local Storage, Day, Digest = ns.Storage, ns.Day, ns.Digest

local Compact = {}
ns.Compact = Compact

local SECONDS_PER_DAY = 86400

-- Day keys sort as strings, so a day is older than the cutoff when its key sorts before it.
-- Counting back from noon keeps a daylight saving change from shifting the cutoff by a day.
local function cutoffKey(now, keepDays)
	local today = date("*t", now)
	today.hour, today.min, today.sec = 12, 0, 0
	return Day.key(time(today) - (keepDays - 1) * SECONDS_PER_DAY)
end

-- Rebuilds digests and compacts days before the retention window. Returns the number of days
-- compacted.
function Compact.run(journal, now, keepDays)
	local cutoff = cutoffKey(now, keepDays)
	local compacted = 0
	for key, day in pairs(journal.days) do
		local hasRaw = day.ev ~= nil or day.kills ~= nil or day.anonKills ~= nil
		if hasRaw then
			day.digest = Digest.build(day)
		end
		if key < cutoff and hasRaw then
			day.ev, day.kills, day.anonKills = nil, nil, nil
			compacted = compacted + 1
		end
	end
	return compacted
end

local function run()
	if Storage.ready then
		Compact.run(Storage.journal(), time(), Storage.settings().retentionDays)
	end
end

ns.On("PLAYER_LOGIN", run)
ns.On("PLAYER_LOGOUT", run)
