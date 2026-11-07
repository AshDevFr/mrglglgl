-- Bounded storage: every whole day gets a fresh digest, and days older than the retention setting
-- are then compacted: they keep their key events (with times and places), their aggregates
-- (kills, skills, crafting, gathering) and the digest, and drop the bulk events the digest counts
-- (zone changes, fishing, loot). A compacted day is marked `c` and its digest is never rebuilt.
-- Runs at login and logout. The app archives what it sees, but a player may not open it for
-- weeks, so a compacted day keeps what the app needs to tell it.
local _, ns = ...

local Storage, Day, Digest = ns.Storage, ns.Day, ns.Digest

local Compact = {}
ns.Compact = Compact

local SECONDS_PER_DAY = 86400
-- Days older than this keep a thinned digest (Digest.thin).
local THIN_AFTER_DAYS = 365

-- The events a compacted day keeps: few per day, and each one a moment of the story.
local KEPT_KINDS = {
	session_start = true, session_end = true, level = true, death = true, boss = true,
	discover = true, quest_accept = true, quest_turnin = true, screenshot = true, group_join = true,
	auction = true, achievement = true,
}

-- Day keys sort as strings, so a day is older than the cutoff when its key sorts before it.
-- Counting back from noon keeps a daylight saving change from shifting the cutoff by a day.
local function cutoffKey(now, keepDays)
	local today = date("*t", now)
	today.hour, today.min, today.sec = 12, 0, 0
	return Day.key(time(today) - (keepDays - 1) * SECONDS_PER_DAY)
end

-- Per-day fields that hold raw data, as opposed to the digest.
local RAW_FIELDS = { "ev", "kills", "anonKills", "skills", "crafted", "gathered", "money", "chat" }

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
		-- A compacted day's digest was built from the whole day; what is left is not enough to
		-- rebuild it, even when a late event lands in the day.
		local whole = not day.c and Compact.hasRaw(day)
		if whole then
			day.digest = Digest.build(day)
		end
		if key < cutoff and whole then
			local kept = {}
			for _, event in ipairs(day.ev or {}) do
				if KEPT_KINDS[event.k] then
					kept[#kept + 1] = event
				end
			end
			day.ev = #kept > 0 and kept or nil
			-- What was said is the largest part of a day; the app has archived it by now, and
			-- the digest keeps how many lines there were.
			day.chat = nil
			day.c = true
			compacted = compacted + 1
		end
		if key < thinCutoff and day.digest and (day.c or not Compact.hasRaw(day)) then
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
