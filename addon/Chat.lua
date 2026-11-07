-- Chat: lines said in the channels the player chose, for role play and for remembering what was
-- said. Off until the player turns it on. Each channel has two switches: the player's own lines,
-- and other players' lines; others' words are theirs, so they are recorded only when asked for.
local _, ns = ...

local Storage, Day = ns.Storage, ns.Day

local Chat = {}
ns.Chat = Chat

-- A busy day of role play stays well under this; a spam channel left on does not fill the file.
local MAX_LINES_PER_DAY = 2000

-- Client event (after CHAT_MSG_) -> the channel as recorded, and the setting that governs it.
local EVENTS = {
	SAY = { "say", "say" },
	EMOTE = { "emote", "emote" },
	TEXT_EMOTE = { "textEmote", "emote" },
	YELL = { "yell", "yell" },
	PARTY = { "party", "party" },
	PARTY_LEADER = { "party", "party" },
	RAID = { "raid", "raid" },
	RAID_LEADER = { "raid", "raid" },
	RAID_WARNING = { "raid", "raid" },
	GUILD = { "guild", "guild" },
	OFFICER = { "officer", "officer" },
	WHISPER = { "whisper", "whisper" },
	WHISPER_INFORM = { "whisper", "whisper" },
	CHANNEL = { "channel", "channel" },
}

-- The channels the settings panel offers, in its order.
Chat.CHANNELS = { "say", "emote", "yell", "party", "raid", "guild", "officer", "whisper", "channel" }

-- The text as a player reads it: link names kept, colour, texture and atlas codes dropped.
function Chat.plain(text)
	text = text:gsub("|c%x%x%x%x%x%x%x%x", ""):gsub("|r", "")
	text = text:gsub("|H.-|h(.-)|h", "%1")
	text = text:gsub("|T.-|t", ""):gsub("|A.-|a", "")
	return (text:gsub("||", "|"))
end

local function hidden(value)
	return issecretvalue ~= nil and issecretvalue(value)
end

local function record(event, text, sender, _, _, _, _, _, _, base, _, _, guid)
	local entry = EVENTS[event]
	if not Storage.records("chat") or type(text) ~= "string" or hidden(text) or hidden(sender) then
		return
	end
	local channel, setting = entry[1], entry[2]
	-- A whisper sent names its recipient as the sender; it is always the player's own line.
	local own = event == "WHISPER_INFORM" or (guid ~= nil and guid == UnitGUID("player"))
		or (guid == nil and sender == UnitName("player"))
	local choices = Storage.settings().chat
	if not choices[own and setting or setting .. "Others"] then
		return
	end
	local day = Day.get(Storage.journal(), Day.key(time()))
	day.chat = day.chat or {}
	if #day.chat >= MAX_LINES_PER_DAY then
		return
	end
	local line = { t = time(), ch = channel, msg = Chat.plain(text) }
	if event == "WHISPER_INFORM" then
		line.to = sender
	elseif not own then
		line.who = sender
	end
	if channel == "channel" and type(base) == "string" and base ~= "" then
		line.cn = base
	end
	day.chat[#day.chat + 1] = line
end

for event in pairs(EVENTS) do
	ns.On("CHAT_MSG_" .. event, function(_, ...)
		record(event, ...)
	end)
end
