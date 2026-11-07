local Mock = require("spec.support.wow_mock")
local Addon = require("spec.support.addon")

local ME = "Player-1-0001"
local TAMSIN = "Player-1-0002"

local function lines()
	local day = _G.MrglglglCharDB.days[os.date("%Y-%m-%d", Mock.now)]
	return day and day.chat or {}
end

-- The client's chat events: text, sender, language, channel, target, flags, zone channel ID,
-- channel index, channel base name, (unused), line ID, sender GUID.
local function chat(event, text, sender, guid, extra)
	extra = extra or {}
	Mock.fire("CHAT_MSG_" .. event, text, sender, "Taur-ahe", extra.channel or "", extra.target or "", "",
		0, extra.index or 0, extra.base or "", nil, 1, guid)
end

describe("Chat", function()
	local ns
	before_each(function()
		ns = Addon.load(Mock)
		Mock.fire("ADDON_LOADED", "Mrglglgl")
		Mock.fire("PLAYER_LOGIN")
		Mock.fire("PLAYER_ENTERING_WORLD", true, false)
	end)

	local function turnOn()
		_G.MrglglglDB.settings.record.chat = true
	end

	it("records nothing until the player turns chat on", function()
		chat("SAY", "Well met.", "Wren", ME)
		assert.same({}, lines())
	end)

	it("records the player's own say, emote and yell lines once on", function()
		turnOn()
		chat("SAY", "Well met.", "Wren", ME)
		chat("EMOTE", "bows deeply.", "Wren", ME)
		chat("TEXT_EMOTE", "You wave.", "Wren", ME)
		chat("YELL", "For the Horde!", "Wren", ME)
		chat("PARTY", "inc", "Wren", ME)
		local got = lines()
		assert.equals(4, #got)
		assert.same({ t = Mock.now, ch = "say", msg = "Well met." }, got[1])
		assert.equals("emote", got[2].ch)
		assert.equals("textEmote", got[3].ch)
		assert.equals("yell", got[4].ch)
	end)

	it("records others' lines only where the player chose to", function()
		turnOn()
		chat("SAY", "Greetings, friend.", "Tamsin", TAMSIN)
		assert.same({}, lines())
		_G.MrglglglDB.settings.chat.sayOthers = true
		chat("SAY", "Greetings, friend.", "Tamsin", TAMSIN)
		assert.same({ t = Mock.now, ch = "say", who = "Tamsin", msg = "Greetings, friend." }, lines()[1])
	end)

	it("folds leader and warning lines into their channel, and notes whispers' other end", function()
		turnOn()
		local c = _G.MrglglglDB.settings.chat
		c.raid, c.party, c.whisper, c.whisperOthers = true, true, true, true
		chat("RAID_WARNING", "Stack!", "Wren", ME)
		chat("PARTY_LEADER", "pull", "Wren", ME)
		chat("WHISPER_INFORM", "On my way.", "Tamsin", ME)
		chat("WHISPER", "Hurry!", "Tamsin", TAMSIN)
		local got = lines()
		assert.same({ "raid", "party", "whisper", "whisper" }, { got[1].ch, got[2].ch, got[3].ch, got[4].ch })
		assert.equals("Tamsin", got[3].to)
		assert.is_nil(got[3].who)
		assert.equals("Tamsin", got[4].who)
	end)

	it("names numbered channels by their base name", function()
		turnOn()
		_G.MrglglglDB.settings.chat.channel = true
		chat("CHANNEL", "LFG Wailing Caverns", "Wren", ME, { channel = "1. General - Barrens", base = "General", index = 1 })
		assert.equals("General", lines()[1].cn)
	end)

	it("keeps link names and drops colour and texture codes", function()
		turnOn()
		chat("SAY", "Look: |cff1eff00|Hitem:2140::::::::5:::::|h[Carving Knife]|h|r {star} |TInterface\\x:0|t", "Wren", ME)
		assert.equals("Look: [Carving Knife] {star} ", lines()[1].msg)
	end)

	it("skips a message the client hides from addons", function()
		turnOn()
		Mock.fire("CHAT_MSG_SAY", Mock.SECRET, "Wren", "", "", "", "", 0, 0, "", nil, 1, ME)
		assert.same({}, lines())
	end)

	it("forgets chat", function()
		turnOn()
		chat("SAY", "Well met.", "Wren", ME)
		ns.Forget.category(_G.MrglglglCharDB, "chat")
		assert.same({}, lines())
	end)
end)
