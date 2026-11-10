-- The optional line at login about the last day that has something in it. Off by default: the
-- addon says nothing unless asked.
local _, ns = ...

local L, Storage, Day, Digest, Compact = ns.L, ns.Storage, ns.Day, ns.Digest, ns.Compact

-- Count keys in the order they are worth mentioning; the line keeps the first few that are not
-- zero. Each has a singular and a plural string, SUMMARY_<KEY>_ONE and SUMMARY_<KEY>_MANY.
local NOTABLE = { "levels", "turnedIn", "achievements", "bosses", "kills", "deaths", "boots", "instances", "loot",
	"discoveries" }
local MAX_PARTS = 4

-- A compacted day only has its digest; a recent one is counted from its events.
local function countsOf(day)
	if Compact.hasRaw(day) then
		return Digest.build(day).counts
	end
	return day.digest and day.digest.counts or {}
end

local function describe(counts)
	local parts = {}
	for _, key in ipairs(NOTABLE) do
		local n = counts[key] or 0
		if n > 0 and #parts < MAX_PARTS then
			local text = L["SUMMARY_" .. key:upper() .. (n == 1 and "_ONE" or "_MANY")]
			parts[#parts + 1] = text:format(n)
		end
	end
	return parts
end

-- The newest day before today with something to say, as its key and the parts of the line.
function ns.LastDaySummary()
	local before = Day.key(time())
	local keys = {}
	for key in pairs(Storage.journal().days) do
		if key < before then
			keys[#keys + 1] = key
		end
	end
	table.sort(keys, function(a, b) return a > b end)
	for _, key in ipairs(keys) do
		local parts = describe(countsOf(Storage.journal().days[key]))
		if #parts > 0 then
			return key, parts
		end
	end
end

-- Only a real login: a reload or a loading screen would repeat it.
ns.On("PLAYER_ENTERING_WORLD", function(_, isInitialLogin)
	if not isInitialLogin or not Storage.ready or not Storage.settings().loginSummary then
		return
	end
	local key, parts = ns.LastDaySummary()
	if key then
		print(L.PREFIX .. L.SUMMARY_LOGIN:format(key, table.concat(parts, ", ")))
	end
end)
