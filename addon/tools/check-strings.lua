-- Fails when addon code passes a string literal straight to something the player reads, so every
-- user-facing string goes through the locale table and can be translated. A heuristic on purpose:
-- it catches the calls that put text on screen, not every literal.
--
-- Usage, from addon/: luajit tools/check-strings.lua [file ...]  (default: the files in the TOC)
local displayCalls = { "print", "AddMessage", "SetText", "SetFormattedText", "RegisterCanvasLayoutCategory" }

local function tocFiles()
	local files = {}
	for line in io.lines("Mrglglgl.toc") do
		if line ~= "" and not line:match("^##") then
			files[#files + 1] = (line:gsub("\\", "/"))
		end
	end
	return files
end

local files = #arg > 0 and arg or tocFiles()
local problems = 0
for _, file in ipairs(files) do
	if not file:match("^Locales/") then
		local n = 0
		for line in io.lines(file) do
			n = n + 1
			local code = line:gsub("%-%-.*$", "")
			for _, call in ipairs(displayCalls) do
				if code:match("[%s%.:%(]" .. call .. "%s*%(%s*[\"'%[]") or code:match("^" .. call .. "%s*%(%s*[\"'%[]") then
					print(("%s:%d: string literal passed to %s; use the locale table"):format(file, n, call))
					problems = problems + 1
				end
			end
		end
	end
end
if problems > 0 then
	os.exit(1)
end
