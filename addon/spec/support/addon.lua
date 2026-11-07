-- Loads the addon the way the client does: every file listed in Mrglglgl.toc, in order, each
-- called with the addon name and a shared namespace table.
local Addon = {}

local function tocFiles()
	local files = {}
	for line in io.lines("Mrglglgl.toc") do
		if line ~= "" and not line:match("^##") then
			files[#files + 1] = (line:gsub("\\", "/"))
		end
	end
	return files
end

-- Installs the mock, loads the addon and returns its namespace. `locale` sets the client's
-- language before any file loads, as the game does.
function Addon.load(Mock, locale)
	Mock.install()
	Mock.locale = locale or Mock.locale
	local ns = {}
	for _, file in ipairs(tocFiles()) do
		local chunk = assert(loadfile(file))
		chunk("Mrglglgl", ns)
	end
	return ns
end

return Addon
