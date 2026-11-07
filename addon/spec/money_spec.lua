local Mock = require("spec.support.wow_mock")
local Addon = require("spec.support.addon")

local function today()
	return _G.MrglglglCharDB.days[os.date("%Y-%m-%d", Mock.now)]
end

local function events(kind)
	local list = {}
	for _, event in ipairs(today().ev) do
		if event.k == kind then
			list[#list + 1] = event
		end
	end
	return list
end

-- The client changes the balance, then says so.
local function pay(copper)
	Mock.money = Mock.money + copper
	Mock.fire("PLAYER_MONEY")
end

describe("Money", function()
	local ns
	before_each(function()
		ns = Addon.load(Mock)
		Mock.money = 12345
		Mock.fire("ADDON_LOADED", "Mrglglgl")
		Mock.fire("PLAYER_LOGIN")
		Mock.fire("PLAYER_ENTERING_WORLD", true, false)
	end)

	it("notes the balance when a session starts and ends", function()
		pay(500)
		Mock.fire("PLAYER_LOGOUT")
		assert.equals(12345, events("session_start")[1].bal)
		assert.equals(12845, events("session_end")[1].bal)
	end)

	it("ends a session on the last balance seen, as the client reports none at logout", function()
		pay(500)
		Mock.money = 0
		Mock.fire("PLAYER_LOGOUT")
		assert.equals(12845, events("session_end")[1].bal)
	end)

	it("sums each change by where it came from or went", function()
		Mock.fire("LOOT_OPENED", false, false)
		pay(37)
		Mock.fire("LOOT_CLOSED")
		Mock.advance(10)
		Mock.fire("MERCHANT_SHOW")
		pay(250)
		pay(-100)
		_G.RepairAllItems()
		pay(-420)
		Mock.fire("MERCHANT_CLOSED")
		Mock.advance(10)
		Mock.fire("QUEST_TURNED_IN", 749, 450, 75)
		pay(75)
		Mock.advance(10)
		Mock.fire("TAXIMAP_OPENED")
		Mock.fire("TAXIMAP_CLOSED")
		pay(-90)
		Mock.advance(10)
		pay(5)
		assert.same({
			inc = { loot = 37, vendor = 250, quest = 75, other = 5 },
			out = { vendor = 100, repair = 420, travel = 90 },
		}, today().money)
	end)

	it("tells auction house money taken from the mail apart from other mail", function()
		Mock.inbox[1] = { sender = "Auction House", subject = "Auction successful: Copper Ore" }
		Mock.inbox[2] = { sender = "Tamsin", subject = "Thanks!" }
		Mock.fire("MAIL_SHOW")
		_G.TakeInboxMoney(1)
		pay(4000)
		Mock.advance(5)
		_G.TakeInboxMoney(2)
		pay(100)
		Mock.fire("MAIL_CLOSED")
		assert.same({ auction = 4000, mail = 100 }, today().money.inc)
	end)

	it("records what happens at the auction house", function()
		Mock.fire("CHAT_MSG_SYSTEM", "Auction created.")
		Mock.fire("CHAT_MSG_SYSTEM", "A buyer has been found for your auction of Copper Ore.")
		Mock.fire("CHAT_MSG_SYSTEM", "You have been outbid on Linen Cloth.")
		Mock.fire("CHAT_MSG_SYSTEM", "Discovered: Red Rocks")
		local seen = {}
		for _, e in ipairs(events("auction")) do
			seen[#seen + 1] = e.act .. ":" .. tostring(e.name)
		end
		assert.same({ "listed:nil", "sold:Copper Ore", "outbid:Linen Cloth" }, seen)
	end)

	it("puts the day's money and auctions in the digest", function()
		Mock.fire("AUCTION_HOUSE_SHOW")
		pay(-60)
		Mock.fire("CHAT_MSG_SYSTEM", "A buyer has been found for your auction of Copper Ore.")
		Mock.fire("AUCTION_HOUSE_CLOSED")
		Mock.advance(10)
		pay(300)
		Mock.fire("PLAYER_LOGOUT")
		local digest = ns.Digest.build(today())
		assert.same({ inc = { other = 300 }, out = { auction = 60 } }, digest.money)
		assert.same({ 300, 60, 1 }, { digest.counts.moneyIn, digest.counts.moneyOut, digest.counts.auctions })
		assert.same({ { act = "sold", name = "Copper Ore" } }, digest.auctions)
		assert.same({ 12345, 12585 }, { digest.balStart, digest.balEnd })
	end)

	it("records nothing about money when money is off", function()
		_G.MrglglglDB.settings.record.money = false
		pay(100)
		Mock.fire("CHAT_MSG_SYSTEM", "Auction created.")
		Mock.fire("PLAYER_LOGOUT")
		assert.is_nil(today().money)
		assert.same({}, events("auction"))
		assert.is_nil(events("session_end")[1].bal)
	end)

	it("writes copper the way the game does", function()
		assert.equals("3g 20s 5c", ns.Money.format(32005))
		assert.equals("75c", ns.Money.format(75))
		assert.equals("1g", ns.Money.format(10000))
		assert.equals("0c", ns.Money.format(0))
	end)

	it("forgets money, auctions and balances, and keeps the sessions", function()
		pay(100)
		Mock.fire("CHAT_MSG_SYSTEM", "Auction created.")
		Mock.fire("PLAYER_LOGOUT")
		ns.Forget.category(_G.MrglglglCharDB, "money")
		assert.is_nil(today().money)
		assert.same({}, events("auction"))
		assert.equals(1, #events("session_start"))
		assert.is_nil(events("session_start")[1].bal)
	end)
end)
