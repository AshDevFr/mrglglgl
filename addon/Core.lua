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

-- A handler that fails must not stop the others, or one bug would silently stop recording
-- whatever shares its event (logout runs the logger, then compaction). Errors are counted in the
-- journal with the last message, so the app can show them, and said once per session in chat;
-- the client's own error display still receives them.
local pendingErrors, lastError, warned = 0, nil, false

-- Moves errors counted before the journal was loaded into it.
function ns.FlushErrors()
	local journal = ns.Storage and ns.Storage.ready and MrglglglCharDB
	if not journal or pendingErrors == 0 then
		return
	end
	local errors = journal.errors or { n = 0 }
	journal.errors = errors
	errors.n = errors.n + pendingErrors
	errors.last = lastError
	pendingErrors = 0
end

function ns.ReportError(event, err)
	local msg = tostring(err)
	if #msg > 300 then
		msg = msg:sub(1, 300)
	end
	lastError = { msg = msg, event = event, t = time() }
	pendingErrors = pendingErrors + 1
	ns.FlushErrors()
	if not warned then
		warned = true
		print(ns.L.ERROR_NOTICE)
	end
	local handler = geterrorhandler and geterrorhandler()
	if handler then
		pcall(handler, err)
	end
end

frame:SetScript("OnEvent", function(_, event, ...)
	local list = handlers[event]
	if not list then
		return
	end
	for i = 1, #list do
		local ok, err = pcall(list[i], event, ...)
		if not ok then
			ns.ReportError(event, err)
		end
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
