-- Writes a table the way the WoW client writes SavedVariables: a leading blank line, `Name = {`,
-- one `["key"] = value,` per line with no indentation, array elements as bare values, and a
-- trailing comma after everything. Keys are sorted (strings, then numbers) so output is stable.
local Serialize = {}

local function quote(s)
	return '"' .. s:gsub("\\", "\\\\"):gsub('"', '\\"'):gsub("\n", "\\n") .. '"'
end

local function scalar(value)
	local kind = type(value)
	if kind == "string" then
		return quote(value)
	elseif kind == "number" then
		if value == math.floor(value) then
			return ("%d"):format(value)
		end
		return ("%.17g"):format(value)
	elseif kind == "boolean" then
		return tostring(value)
	end
	error("cannot serialise a " .. kind)
end

local function isArray(t)
	local n = #t
	for key in pairs(t) do
		if type(key) ~= "number" or key < 1 or key > n or key % 1 ~= 0 then
			return false
		end
	end
	return true
end

local function sortedKeys(t)
	local keys = {}
	for key in pairs(t) do
		keys[#keys + 1] = key
	end
	table.sort(keys, function(a, b)
		if type(a) == type(b) then
			return a < b
		end
		return type(a) == "string"
	end)
	return keys
end

local function write(out, value)
	if type(value) ~= "table" then
		out[#out + 1] = scalar(value)
		return
	end
	out[#out + 1] = "{\n"
	if isArray(value) then
		for _, item in ipairs(value) do
			write(out, item)
			out[#out + 1] = ",\n"
		end
	else
		for _, key in ipairs(sortedKeys(value)) do
			out[#out + 1] = "[" .. scalar(key) .. "] = "
			write(out, value[key])
			out[#out + 1] = ",\n"
		end
	end
	out[#out + 1] = "}"
end

-- Returns the file text for global name holding value.
function Serialize.savedVariable(name, value)
	local out = { "\n", name, " = " }
	write(out, value)
	out[#out + 1] = "\n"
	return table.concat(out)
end

-- Reads a SavedVariables file into a table of its globals, without touching the real globals.
function Serialize.load(path)
	local env = {}
	local chunk = assert(loadfile(path))
	setfenv(chunk, env)
	chunk()
	return env
end

return Serialize
