-- Bounded storage: every day with raw events gets a fresh digest, and days older than the
-- retention setting then drop their raw events, keeping only the digest. Runs at login and
-- logout. The app backs up and archives full history, so dropping raw events here loses nothing
-- the app has already seen.
local _, ns = ...

local Storage, Day, Digest = ns.Storage, ns.Day, ns.Digest

local Compact = {}
ns.Compact = Compact

local SECONDS_PER_DAY = 86400
-- Days older than this keep a thinned digest (Digest.thin).
local THIN_AFTER_DAYS = 90

-- Day keys sort as strings, so a day is older than the cutoff when its key sorts before it.
-- Counting back from noon keeps a daylight saving change from shifting the cutoff by a day.
local function cutoffKey(now, keepDays)
	local today = date("*t", now)
	today.hour, today.min, today.sec = 12, 0, 0
	return Day.key(time(today) - (keepDays - 1) * SECONDS_PER_DAY)
end

-- Per-day fields that hold raw data: dropped when a day is compacted.
local RAW_FIELDS = { "ev", "kills", "anonKills", "skills", "crafted", "gathered" }

function Compact.hasRaw(day)
	for _, field in ipairs(RAW_FIELDS) do
		if day[field] ~= nil then
			return true
		end
	end
	return false
end

-- Rebuilds digests and compacts days before the retention window. Returns the number of days
-- compacted.
function Compact.run(journal, now, keepDays)
	local cutoff = cutoffKey(now, keepDays)
	local thinCutoff = cutoffKey(now, THIN_AFTER_DAYS)
	local compacted = 0
	for key, day in pairs(journal.days) do
		local hasRaw = Compact.hasRaw(day)
		if hasRaw then
			day.digest = Digest.build(day)
		end
		if key < cutoff and hasRaw then
			for _, field in ipairs(RAW_FIELDS) do
				day[field] = nil
			end
			compacted = compacted + 1
		end
		if key < thinCutoff and not Compact.hasRaw(day) and day.digest then
			Digest.thin(day.digest)
		end
	end
	-- Quest descriptions are kept while the quest is inside the window and only the title after:
	-- the description is most of a journal's size, and the app has archived it by then.
	for _, quest in pairs(journal.quests or {}) do
		if quest.text and (quest.last == nil or quest.last < cutoff) then
			quest.text = nil
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
