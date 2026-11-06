-- Namespace, the single event frame, and event dispatch to modules.
local addonName, ns = ...

ns.name = addonName

local frame = CreateFrame("Frame")
local handlers = {}

-- Registers fn(event, ...) for event. Several modules may listen to the same event; they run in
-- registration order.
function ns.On(event, fn)
	local list = handlers[event]
	if not list then
		list = {}
		handlers[event] = list
		frame:RegisterEvent(event)
	end
	list[#list + 1] = fn
end

frame:SetScript("OnEvent", function(_, event, ...)
	local list = handlers[event]
	if not list then
		return
	end
	for i = 1, #list do
		list[i](event, ...)
	end
end)

-- Signals between modules, for moments that are not client events (a first boot, a first murloc).
local listeners = {}

function ns.Listen(signal, fn)
	local list = listeners[signal] or {}
	listeners[signal] = list
	list[#list + 1] = fn
end

function ns.Emit(signal, ...)
	for _, fn in ipairs(listeners[signal] or {}) do
		fn(...)
	end
end
