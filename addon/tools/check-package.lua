-- Checks the package definition without building it: every file the TOC loads exists and is not
-- excluded by .pkgmeta, the TOC carries what the packager and the client need, and nothing used
-- only for development is left in the package.
--
-- The package definition is the repository's .pkgmeta: the addon sits in addon/ of a monorepo,
-- so its paths there carry that prefix.
--
-- Usage, from addon/: luajit tools/check-package.lua
local PKGMETA = "../.pkgmeta"
local PREFIX = "addon/"
local problems = 0

local function fail(message)
	io.write("package: ", message, "\n")
	problems = problems + 1
end

local function exists(path)
	local file = io.open(path)
	if file then
		file:close()
		return true
	end
	return false
end

local ignored = {}
for line in io.lines(PKGMETA) do
	local entry = line:match("^%s*%-%s*(%S+)%s*$")
	if entry then
		ignored[#ignored + 1] = entry
	end
end

local function isIgnored(path)
	path = PREFIX .. path
	for _, entry in ipairs(ignored) do
		if path == entry or path:sub(1, #entry + 1) == entry .. "/" then
			return true
		end
	end
	return false
end

local headers, files = {}, {}
for line in io.lines("Mrglglgl.toc") do
	local key, value = line:match("^## ([%w%-]+):%s*(.-)%s*$")
	if key then
		headers[key] = value
	elseif line ~= "" then
		files[#files + 1] = (line:gsub("\\", "/"))
	end
end

for _, key in ipairs({ "Interface", "Title", "Version", "SavedVariables", "SavedVariablesPerCharacter" }) do
	if not headers[key] or headers[key] == "" then
		fail("TOC is missing ## " .. key)
	end
end
if headers.Interface and not headers.Interface:match("16%d%d%d") then
	fail("TOC interface " .. headers.Interface .. " does not target Forever (16xxx)")
end
if headers.Version ~= "@project-version@" then
	fail("TOC version should be the packager keyword @project-version@")
end

-- The app reads this field to tell whether it can read what the installed addon writes.
local current
for line in io.lines("Migrate.lua") do
	current = current or line:match("^Migrate%.CURRENT = (%d+)")
end
if headers["X-Mrglglgl-Schema"] ~= current then
	fail("TOC X-Mrglglgl-Schema should be the journal schema, " .. tostring(current))
end

for _, path in ipairs(files) do
	if not exists(path) then
		fail("TOC lists " .. path .. ", which does not exist")
	elseif isIgnored(path) then
		fail("TOC lists " .. path .. ", which .pkgmeta excludes from the package")
	end
end

local moved = false
for line in io.lines(PKGMETA) do
	if line:match("^%s*Mrglglgl/addon:%s*Mrglglgl%s*$") then
		moved = true
	end
end
if not moved then
	fail(".pkgmeta does not move addon/ to the package root")
end

for _, devOnly in ipairs({ "spec", "tools", "Makefile", ".rocks" }) do
	if not isIgnored(devOnly) then
		fail(".pkgmeta does not exclude " .. devOnly)
	end
end

-- The packager ships every repository entry .pkgmeta does not exclude, so a folder added at the
-- root (a docs site, say) lands in the addon zip, and the app refuses to install a package holding
-- files the game has no use for. Every tracked root entry must be the addon, the licence, or
-- excluded. Dot entries are left out: the packager skips them on its own.
local SHIPPED_FROM_ROOT = { addon = true, LICENSE = true }
local listing = io.popen("git -C .. ls-files")
if not listing then
	fail("could not list the repository's files with git")
else
	local seen = {}
	for path in listing:lines() do
		local top = path:match("^[^/]+")
		if top and not seen[top] and top:sub(1, 1) ~= "." and not SHIPPED_FROM_ROOT[top] then
			seen[top] = true
			local excluded = false
			for _, entry in ipairs(ignored) do
				if entry == top then
					excluded = true
				end
			end
			if not excluded then
				fail(".pkgmeta does not exclude " .. top .. ", which would ship in the addon zip")
			end
		end
	end
	listing:close()
end

if problems > 0 then
	os.exit(1)
end
io.write("package: ok, ", #files, " files\n")
