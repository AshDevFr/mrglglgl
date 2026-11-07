-- A thin stand-in for the WoW client API, enough to run addon modules under busted. Tests set
-- client state through the Mock table (clock, units, globals) and fire events with Mock.fire.
local Mock = {}

local frames = {}
local timers = {}
local SECRET = setmetatable({}, { __tostring = function() return "<secret>" end })

Mock.SECRET = SECRET
Mock.screenshots = 0
Mock.now = 1793938800 -- 2026-11-05 20:00 local time, in the test machine's time zone
Mock.units = {}
Mock.locale = "enUS"

local function newFrame()
	local frame = { events = {}, scripts = {} }
	function frame:RegisterEvent(event)
		self.events[event] = true
	end
	function frame:UnregisterEvent(event)
		self.events[event] = nil
	end
	function frame:UnregisterAllEvents()
		self.events = {}
	end
	function frame:IsEventRegistered(event)
		return self.events[event] == true
	end
	function frame:SetScript(name, fn)
		self.scripts[name] = fn
	end
	function frame.Hide() end
	function frame.Show() end
	frames[#frames + 1] = frame
	return frame
end

-- Delivers event to every frame registered for it, like the client's event loop.
function Mock.fire(event, ...)
	for _, frame in ipairs(frames) do
		if frame.events[event] and frame.scripts.OnEvent then
			frame.scripts.OnEvent(frame, event, ...)
		end
	end
end

-- True if any frame is registered for event.
function Mock.isRegistered(event)
	for _, frame in ipairs(frames) do
		if frame.events[event] then
			return true
		end
	end
	return false
end

-- Moves the clock forward and runs timers that came due, in order.
-- Timers scheduled by a timer run too, at their own due time, as in the client.
function Mock.advance(seconds)
	local target = Mock.now + seconds
	while true do
		table.sort(timers, function(a, b) return a.due < b.due end)
		local timer = timers[1]
		if timer == nil or timer.due > target then
			break
		end
		table.remove(timers, 1)
		Mock.now = math.max(Mock.now, timer.due)
		timer.fn()
	end
	Mock.now = target
end

-- Sets what the unit functions return for a unit token, e.g. Mock.setUnit("target", {
-- guid = "Creature-0-1-2-3-4-2958-0", name = "Prairie Wolf", classification = "normal" }).
function Mock.setUnit(token, unit)
	Mock.units[token] = unit
end

local function unitField(token, field)
	local unit = Mock.units[token]
	return unit and unit[field]
end

-- Installs the mock as globals and resets all state. Call before loading the addon.
-- Specs must reach SavedVariables as _G.MrglglglDB and _G.MrglglglCharDB: busted gives each spec
-- file its own global environment, so a bare name would read and write a copy the addon never sees.
function Mock.install()
	frames, timers = {}, {}
	Mock.screenshots = 0
	Mock.now = 1793938800
	Mock.units = {
		player = { guid = "Player-1-0001", name = "Wren", surname = "Hollowbrook", level = 5,
			race = "Tauren", className = "Hunter", class = "HUNTER", faction = "Horde" },
	}
	Mock.locale = "enUS"
	Mock.buildInfo = { "1.60.1", "70790", "Nov  3 2026", 16001 }

	_G.MrglglglDB, _G.MrglglglCharDB = nil, nil
	Mock.printed = {}
	_G.print = function(...)
		local parts = {}
		for i = 1, select("#", ...) do
			parts[i] = tostring((select(i, ...)))
		end
		Mock.printed[#Mock.printed + 1] = table.concat(parts, " ")
	end
	_G.CreateFrame = function() return newFrame() end
	_G.GetLocale = function() return Mock.locale end
	_G.GetBuildInfo = function() return unpack(Mock.buildInfo) end
	Mock.addonVersion = "1.2.3"
	_G.C_AddOns = { GetAddOnMetadata = function(_, field)
		if field == "Version" then return Mock.addonVersion end
	end }
	_G.time = function(t)
		if t then
			return os.time(t)
		end
		return Mock.now
	end
	_G.date = function(format, t)
		return os.date(format, t or Mock.now)
	end
	_G.GetTime = function() return Mock.now end
	Mock.zone, Mock.subzone, Mock.mapId = "Mulgore", "", 7
	_G.GetZoneText = function() return Mock.zone end
	_G.GetSubZoneText = function() return Mock.subzone end
	_G.C_Map = { GetBestMapForUnit = function() return Mock.mapId end }
	_G.C_Timer = {
		After = function(delay, fn)
			timers[#timers + 1] = { due = Mock.now + delay, fn = fn }
		end,
	}
	_G.issecretvalue = function(value) return value == SECRET end
	_G.UnitGUID = function(token) return unitField(token, "guid") end
	_G.UnitName = function(token) return unitField(token, "name") end
	_G.UnitFullName = function(token) return unitField(token, "name"), unitField(token, "surname") end
	_G.UnitClassification = function(token) return unitField(token, "classification") or "normal" end
	_G.UnitLevel = function(token) return unitField(token, "level") or 1 end
	_G.UnitRace = function(token) return unitField(token, "race"), unitField(token, "race") end
	_G.UnitClass = function(token) return unitField(token, "className"), unitField(token, "class") end
	_G.UnitFactionGroup = function(token) return unitField(token, "faction") end
	_G.GetRealmName = function() return "Test Realm" end
	_G.GetRealmID = function() return 1 end
	Mock.groupSize, Mock.inRaid, Mock.recap = 0, false, nil
	_G.GetNumGroupMembers = function() return Mock.groupSize end
	_G.IsInRaid = function() return Mock.inRaid end
	_G.IsInGroup = function() return Mock.groupSize > 0 end
	_G.C_DeathRecap = {
		HasRecapEvents = function() return Mock.recap ~= nil and Mock.recap[1] ~= nil end,
		GetRecapEvents = function() return Mock.recap or {} end,
	}
	Mock.questDetail, Mock.questTitles, Mock.questLogSelections = nil, {}, 0
	_G.GetQuestID = function() return Mock.questDetail and Mock.questDetail.id end
	_G.GetTitleText = function() return Mock.questDetail and Mock.questDetail.title end
	_G.GetQuestText = function() return Mock.questDetail and Mock.questDetail.text end
	-- Quests in the log by ID: { level = 9, tagID = 81, isElite = false }.
	Mock.questLog = {}
	_G.C_QuestLog = {
		GetTitleForQuestID = function(id) return Mock.questTitles[id] end,
		GetLogIndexForQuestID = function(id) return Mock.questLog[id] and id or nil end,
		GetInfo = function(index)
			local q = Mock.questLog[index]
			return q and { questID = index, level = q.level } or nil
		end,
		GetQuestTagInfo = function(id)
			local q = Mock.questLog[id]
			if not q or (q.tagID == nil and not q.isElite) then
				return nil
			end
			return { tagID = q.tagID, tagName = q.tagName, isElite = q.isElite == true }
		end,
	}
	_G.SelectQuestLogEntry = function() Mock.questLogSelections = Mock.questLogSelections + 1 end
	_G.C_QuestLog.SetSelectedQuest = _G.SelectQuestLogEntry
	Mock.loot, Mock.fishing, Mock.chatLockdown = {}, false, false
	_G.GetNumLootItems = function() return #Mock.loot end
	_G.GetLootSlotLink = function(slot) return Mock.loot[slot] end
	_G.IsFishingLoot = function() return Mock.fishing end
	_G.C_ChatInfo = { InChatMessagingLockdown = function() return Mock.chatLockdown end }
	_G.UnitIsUnit = function(a, b) return a == b or (Mock.units[a] ~= nil and Mock.units[a] == Mock.units[b]) end
	_G.SlashCmdList = {}
	Mock.skills, Mock.spells = {}, {}
	_G.C_SkillInfo = {
		GetNumSkillLines = function() return #Mock.skills end,
		GetSkillLineInfo = function(index) return Mock.skills[index] end,
	}
	_G.C_Spell = { GetSpellName = function(id) return Mock.spells[id] end }
	_G.LOOT_ITEM_CREATED_SELF = "You create: %s."
	_G.LOOT_ITEM_CREATED_SELF_MULTIPLE = "You create: %sx%d."
	_G.Screenshot = function() Mock.screenshots = Mock.screenshots + 1 end
	-- The client's vertical settings layout, recording what the addon builds. Mock.controls maps
	-- each setting's variable name to { kind, setting, options? }.
	Mock.controls, Mock.headers, Mock.buttons, Mock.registered, Mock.opened = {}, {}, {}, false, nil
	_G.Settings = {
		VarType = { Boolean = "boolean", Number = "number", String = "string" },
		RegisterVerticalLayoutCategory = function(name)
			local layout = { AddInitializer = function(_, initializer)
				if initializer.header then
					Mock.headers[#Mock.headers + 1] = initializer.header
				elseif initializer.button then
					Mock.buttons[initializer.button] = initializer
				end
			end }
			return { name = name, GetID = function() return 42 end }, layout
		end,
		RegisterProxySetting = function(_, variable, _, name, default, getValue, setValue)
			return { variable = variable, name = name, default = default,
				GetValue = function() return getValue() end, SetValue = function(_, v) setValue(v) end }
		end,
		CreateCheckbox = function(_, setting) Mock.controls[setting.variable] = { kind = "checkbox", setting = setting } end,
		CreateDropdown = function(_, setting, options)
			Mock.controls[setting.variable] = { kind = "dropdown", setting = setting, options = options() }
		end,
		CreateControlTextContainer = function()
			local data = {}
			return { Add = function(_, value, label) data[#data + 1] = { value = value, label = label } end,
				GetData = function() return data end }
		end,
		RegisterAddOnCategory = function() Mock.registered = true end,
		OpenToCategory = function(id) Mock.opened = id end,
	}
	_G.CreateSettingsListSectionHeaderInitializer = function(name) return { header = name } end
	_G.CreateSettingsButtonInitializer = function(name, _, click) return { button = name, click = click } end
	_G.LOOT_ITEM_SELF = "You receive loot: %s"
	_G.LOOT_ITEM_SELF_MULTIPLE = "You receive loot: %sx%d"
	_G.LOOT_ITEM_PUSHED_SELF = "You receive item: %s"
	_G.ERR_ZONE_EXPLORED = "Discovered: %s"
	-- Money, in copper, and the client functions the addon hooks to tell repairs and auction mail.
	Mock.money = 0
	_G.GetMoney = function() return Mock.money end
	Mock.hooks = {}
	_G.hooksecurefunc = function(name, callback)
		local original = _G[name]
		Mock.hooks[name] = true
		_G[name] = function(...)
			local result = original(...)
			callback(...)
			return result
		end
	end
	_G.RepairAllItems = function() end
	_G.TakeInboxMoney = function() end
	_G.AutoLootMailItem = function() end
	Mock.inbox = {}
	_G.GetInboxHeaderInfo = function(index)
		local mail = Mock.inbox[index] or {}
		return nil, nil, mail.sender, mail.subject
	end
	_G.ERR_AUCTION_STARTED = "Auction created."
	_G.ERR_AUCTION_SOLD_S = "A buyer has been found for your auction of %s."
	_G.ERR_AUCTION_WON_S = "You won an auction for %s"
	_G.ERR_AUCTION_OUTBID_S = "You have been outbid on %s."
	_G.ERR_AUCTION_EXPIRED_S = "Your auction of %s has expired."
	_G.ERR_AUCTION_REMOVED_S = "Your auction of %s has been cancelled."
	_G.AUCTION_SOLD_MAIL_SUBJECT = "Auction successful: %s"
	_G.ERR_ZONE_EXPLORED_XP = "Discovered %s: %d experience gained"
	_G.COMBATLOG_XPGAIN_FIRSTPERSON = "%s dies, you gain %d experience."
	_G.COMBATLOG_XPGAIN_FIRSTPERSON_GROUP = "%s dies, you gain %d experience. (+%d group bonus)"
	_G.COMBATLOG_XPGAIN_FIRSTPERSON_RAID = "%s dies, you gain %d experience. (-%d raid penalty)"
	_G.C_Item = {
		GetItemInfo = function(link)
			local item = Mock.items and Mock.items[tonumber(link:match("item:(%d+)"))]
			if item then
				return item.name, link, item.quality
			end
		end,
	}
	Mock.items = {}
end

return Mock
