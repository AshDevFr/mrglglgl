-- Money: the balance at each session's start and end (on the session events, see Logger), every
-- change of it summed per day by where it came from or went, and what happens at the auction
-- house. So the journal can tell how the character makes, and spends, a living.
local _, ns = ...

local Storage, Day, Compat = ns.Storage, ns.Day, ns.Compat

local Money = {}
ns.Money = Money

-- The windows a change of money happens in, by the events that open and close them.
local WINDOWS = {
	{ "MERCHANT_SHOW", "MERCHANT_CLOSED", "vendor" },
	{ "TRAINER_SHOW", "TRAINER_CLOSED", "trainer" },
	{ "MAIL_SHOW", "MAIL_CLOSED", "mail" },
	{ "AUCTION_HOUSE_SHOW", "AUCTION_HOUSE_CLOSED", "auction" },
	{ "TAXIMAP_OPENED", "TAXIMAP_CLOSED", "travel" },
	{ "TRADE_SHOW", "TRADE_CLOSED", "trade" },
	{ "LOOT_OPENED", "LOOT_CLOSED", "loot" },
}
-- Seconds a window or an action still explains a change: the client reports the money a moment
-- later (a flight is paid as its map closes, a repair after the click).
local GRACE = 2

local balance
local window, windowUntil
local hint, hintUntil

local function now()
	return GetTime()
end

local function note(source)
	hint, hintUntil = source, now() + GRACE
end

-- Where a change of money came from, or went: an action just taken first, then the window open
-- (or just closed), else "other".
function Money.source()
	local t = now()
	if hint and t <= hintUntil then
		return hint
	end
	if window and (windowUntil == nil or t <= windowUntil) then
		return window
	end
	return "other"
end

local function add(direction, source, amount)
	local day = Day.get(Storage.journal(), Day.key(time()))
	day.money = day.money or {}
	local side = day.money[direction] or {}
	day.money[direction] = side
	side[source] = (side[source] or 0) + amount
end

for _, entry in ipairs(WINDOWS) do
	local open, close, source = entry[1], entry[2], entry[3]
	ns.On(open, function()
		window, windowUntil = source, nil
	end)
	ns.On(close, function()
		if window == source then
			windowUntil = now() + GRACE
		end
	end)
end

ns.On("PLAYER_ENTERING_WORLD", function()
	balance = Compat.GetMoney()
end)

ns.On("PLAYER_MONEY", function()
	local current = Compat.GetMoney()
	if current == nil then
		return
	end
	local before = balance
	balance = current
	if before == nil or current == before or not Storage.records("money") then
		return
	end
	local delta = current - before
	add(delta > 0 and "inc" or "out", Money.source(), math.abs(delta))
end)

ns.On("QUEST_TURNED_IN", function(_, _, _, money)
	if money and money > 0 then
		note("quest")
	end
end)

ns.On("PLAYER_LOGIN", function()
	Compat.After("RepairAllItems", function() note("repair") end)
	local function fromMail(index)
		if Compat.IsAuctionMail(index) then
			note("auction")
		end
	end
	Compat.After("TakeInboxMoney", fromMail)
	Compat.After("AutoLootMailItem", fromMail)
end)

ns.On("CHAT_MSG_SYSTEM", function(_, text)
	if not Storage.records("money") then
		return
	end
	local act, name = Compat.ParseAuctionMessage(text)
	if act then
		Day.append("auction", { act = act, name = name })
	end
end)

-- Copper as the game writes it: "3g 20s 5c", leaving out empty parts.
function Money.format(copper)
	local g, s, c = math.floor(copper / 10000), math.floor(copper / 100) % 100, copper % 100
	local parts = {}
	if g > 0 then parts[#parts + 1] = g .. "g" end
	if s > 0 then parts[#parts + 1] = s .. "s" end
	if c > 0 or #parts == 0 then parts[#parts + 1] = c .. "c" end
	return table.concat(parts, " ")
end
