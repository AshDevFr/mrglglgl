-- "Forget recorded ...": deletes one category from the journal on this character, from raw days
-- and from the digests of compacted days. Copies the app has already archived are the app's to
-- delete; its settings say so.
local _, ns = ...

local Digest = ns.Digest

local Forget = {}
ns.Forget = Forget

-- Per category: the event kinds it owns, the digest counts and lists to clear on compacted days,
-- and what to reset outside the days.
local CATEGORIES = {
	sessions = { kinds = { session_start = true, session_end = true }, counts = { "sessions" } },
	levels = { kinds = { level = true }, counts = { "levels" } },
	quests = {
		kinds = { quest_accept = true, quest_turnin = true },
		counts = { "accepted", "turnedIn" }, lists = { "accepted", "turnedIn" },
		journal = function(journal) journal.quests = {} end,
	},
	questText = {
		journal = function(journal)
			for _, quest in pairs(journal.quests) do
				quest.text, quest.done = nil, nil
			end
		end,
	},
	zones = {
		kinds = { zone = true, subzone = true, discover = true },
		counts = { "discoveries" }, lists = { "zones", "subzones", "discovered" },
		journal = function(journal) journal.discovered = {} end,
	},
	kills = {
		kinds = { boss = true }, raw = { "kills", "anonKills" },
		counts = { "kills", "murlocs", "anonKills", "bosses" }, lists = { "kills", "notableKills", "bosses" },
		journal = function(journal) journal.stats.murlocsSlain = 0 end,
	},
	deaths = {
		kinds = { death = true }, counts = { "deaths" }, lists = { "deaths" },
		journal = function(journal)
			journal.stats.deathsByMurloc = 0
			journal.stats.drownings = 0
		end,
	},
	fishing = {
		kinds = { fish = true }, counts = { "fish", "junk", "boots" }, lists = { "fish" },
		journal = function(journal) journal.stats.boots = 0 end,
	},
	loot = { kinds = { loot = true }, counts = { "loot" }, lists = { "loot" } },
	professions = {
		raw = { "skills", "crafted", "gathered" },
		counts = { "skillUps", "crafted", "gathered" }, lists = { "skills", "crafted", "gathered" },
	},
	money = {
		kinds = { auction = true }, raw = { "money" }, balances = true,
		counts = { "moneyIn", "moneyOut", "auctions" }, lists = { "auctions" },
	},
	chat = { raw = { "chat" }, counts = { "chat" } },
	achievements = {
		kinds = { achievement = true }, counts = { "achievements" }, lists = { "achievements" },
		journal = function(journal) journal.challenges = nil end,
	},
	spells = { kinds = { spell = true }, counts = { "spells" } },
	collections = { kinds = { mount = true, pet = true }, counts = { "collections" } },
	reputation = { kinds = { rep = true }, counts = { "reputation" } },
	gear = { kinds = { gear = true }, counts = { "gear" } },
	travel = { kinds = { bind = true, flight_path = true }, counts = { "travel" } },
	duels = { kinds = { duel = true }, counts = { "duels" } },
	dungeons = { kinds = { instance = true }, counts = { "instances" } },
	group = {
		kinds = { group_join = true }, counts = { "group" }, lists = { "companions" },
		journal = function(journal) journal.companions = {} end,
	},
}

Forget.CATEGORIES = CATEGORIES

local function scrubDigest(digest, spec)
	for _, count in ipairs(spec.counts or {}) do
		digest.counts[count] = 0
	end
	for _, list in ipairs(spec.lists or {}) do
		digest[list] = {}
	end
	if spec.balances then
		digest.money = { inc = {}, out = {} }
		digest.balStart, digest.balEnd = nil, nil
	end
	if spec.kinds and spec.kinds.fish then
		digest.onlyFished = false
	end
end

-- Removes category from every day and from the journal's running totals.
function Forget.category(journal, category)
	local spec = CATEGORIES[category]
	if spec == nil then
		error("unknown category " .. tostring(category), 2)
	end
	for _, day in pairs(journal.days) do
		local hasRaw = ns.Compact.hasRaw(day)
		if spec.kinds and day.ev then
			local kept = {}
			for _, event in ipairs(day.ev) do
				if not spec.kinds[event.k] then
					kept[#kept + 1] = event
				end
			end
			day.ev = kept
		end
		for _, field in ipairs(spec.raw or {}) do
			day[field] = nil
		end
		-- Balances ride on session events, which belong to sessions; only the balance goes.
		if spec.balances then
			for _, event in ipairs(day.ev or {}) do
				event.bal = nil
			end
		end
		if hasRaw then
			day.digest = Digest.build(day)
		end
		-- Scrubbed after a rebuild too: every event carries its zone, so a rebuilt digest would
		-- still list the zones of a day whose zone events were forgotten.
		if day.digest then
			scrubDigest(day.digest, spec)
		end
	end
	if spec.journal then
		spec.journal(journal)
	end
end
