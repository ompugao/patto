-- Thumbnails for `[@embed ...]` links. Each site is asked for a preview image
-- with curl, and the image is downloaded into the cache, so the inline image
-- code only ever sees local files. Thumbnail URLs often have no extension
-- (Google Photos), which snacks would reject.
local M = {}

-- Google serves a stripped share page (without Open Graph tags) to unknown
-- clients, so its pages are fetched as a browser. Not used for other sites:
-- SlideShare answers a browser without JavaScript with a bot challenge.
-- Same as utils::BROWSER_USER_AGENT.
local USER_AGENT =
  "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0 Safari/537.36"

local EXTENSIONS = {
  ["image/jpeg"] = "jpg",
  ["image/jpg"] = "jpg",
  ["image/png"] = "png",
  ["image/webp"] = "webp",
  ["image/gif"] = "gif",
  ["image/avif"] = "avif",
}
local CACHED = { "jpg", "png", "webp", "gif", "avif", "none" }

local TWITTER_HOSTS = {
  ["twitter.com"] = true,
  ["www.twitter.com"] = true,
  ["mobile.twitter.com"] = true,
  ["x.com"] = true,
  ["www.x.com"] = true,
}

function M.parse_url(url)
  local scheme, authority, path, query = url:match("^(%a[%w+.-]*)://([^/?#]*)([^?#]*)%??([^#]*)")
  if not scheme then
    return
  end
  local host = authority:gsub("^.*@", ""):gsub(":%d+$", ""):lower()
  return { scheme = scheme:lower(), host = host, path = path, query = query }
end

local function query_param(query, name)
  for pair in query:gmatch("[^&]+") do
    local k, v = pair:match("^([^=]*)=?(.*)$")
    if k == name then
      return v
    end
  end
end

function M.youtube_id(url)
  local u = M.parse_url(url)
  if not u then
    return
  end
  if u.host == "youtu.be" then
    local id = u.path:sub(2)
    return id ~= "" and id or nil
  end
  if u.host == "www.youtube.com" or u.host == "youtube.com" then
    if u.path == "/watch" then
      return query_param(u.query, "v")
    end
    return u.path:match("^/embed/([^/]+)") or u.path:match("^/v/([^/]+)")
  end
end

function M.is_pdf(url)
  return (url:gsub("[?#].*$", "")):lower():sub(-4) == ".pdf"
end

local function is_google_photos(u)
  return u.host == "photos.app.goo.gl"
    or (u.host == "goo.gl" and u.path:find("^/photos") ~= nil)
    or (u.host == "photos.google.com" and u.path:find("/share/") ~= nil)
end

function M.kind(url)
  if M.is_pdf(url) then
    return "pdf"
  end
  local u = M.parse_url(url)
  if not u then
    return
  end
  if M.youtube_id(url) then
    return "youtube"
  end
  if is_google_photos(u) then
    return "google_photos"
  end
  if TWITTER_HOSTS[u.host] then
    return "twitter"
  end
  if u.host == "speakerdeck.com" or u.host == "www.speakerdeck.com" then
    return "speakerdeck"
  end
  if u.host == "slideshare.net" or u.host == "www.slideshare.net" then
    return "slideshare"
  end
  return "other"
end

local function decode_entities(s)
  s = s:gsub("&#[xX](%x+);", function(hex)
    return vim.fn.nr2char(tonumber(hex, 16))
  end)
  s = s:gsub("&#(%d+);", function(dec)
    return vim.fn.nr2char(tonumber(dec))
  end)
  return (s:gsub("&quot;", '"'):gsub("&lt;", "<"):gsub("&gt;", ">"):gsub("&amp;", "&"))
end

local function attribute(tag, name)
  local _, value = tag:match("%f[%w]" .. name .. "%s*=%s*([\"'])(.-)%1")
  return value
end

-- `property` is what Open Graph specifies; `name` is common for twitter:image.
function M.og_meta(html, prop)
  for tag in html:gmatch("<meta%s[^>]*>") do
    local key = attribute(tag, "property") or attribute(tag, "name")
    if key and key:lower() == prop then
      local content = attribute(tag, "content")
      if content and content ~= "" then
        return decode_entities(content)
      end
    end
  end
end

function M.absolute(url, base)
  if url:find("^%a[%w+.-]*://") then
    return url
  end
  local u = M.parse_url(base)
  if not u then
    return url
  end
  if url:sub(1, 2) == "//" then
    return u.scheme .. ":" .. url
  end
  local origin = u.scheme .. "://" .. u.host
  if url:sub(1, 1) == "/" then
    return origin .. url
  end
  return origin .. u.path:gsub("[^/]*$", "") .. url
end

-- Drop the `=w600-h315-...` sizing suffix, as parse_google_photos_page does.
function M.google_photos_thumb(og_image)
  return og_image:gsub("=[^=/]*$", "") .. "=w1280-h720-no"
end

local function first(list, field)
  local item = type(list) == "table" and list[1]
  local value = type(item) == "table" and item[field]
  return type(value) == "string" and value or nil
end

function M.tweet_thumb(json)
  local media = type(json) == "table" and type(json.tweet) == "table" and json.tweet.media
  if type(media) ~= "table" then
    return
  end
  return first(media.photos, "url") or first(media.videos, "thumbnail_url")
end

local function tweet_id(url)
  local u = M.parse_url(url)
  return u and u.path:match("/status/(%d+)")
end

local function curl(args, cb)
  local cmd = { "curl", "-sSL", "--fail", "--connect-timeout", "5", "--max-time", "15" }
  vim.list_extend(cmd, args)
  local done = vim.schedule_wrap(cb)
  local ok = pcall(vim.system, cmd, { text = true }, done)
  if not ok then
    done({ code = -1, stdout = "" })
  end
end

local function fetch_json(url, cb)
  curl({ url }, function(r)
    if r.code ~= 0 then
      return cb(nil, true)
    end
    local ok, json = pcall(vim.json.decode, r.stdout)
    cb(ok and json or nil, false)
  end)
end

local function fetch_page_image(url, cb, as_browser)
  local args = as_browser and { "-A", USER_AGENT } or {}
  vim.list_extend(args, { "--max-filesize", "5000000", url })
  curl(args, function(r)
    if r.code ~= 0 then
      return cb(nil, true)
    end
    local image = M.og_meta(r.stdout, "og:image") or M.og_meta(r.stdout, "twitter:image")
    cb(image and M.absolute(image, url), false)
  end)
end

-- cb(thumbnail_url, network_error)
local lookups = {
  youtube = function(url, cb)
    cb("https://i.ytimg.com/vi/" .. M.youtube_id(url) .. "/hqdefault.jpg", false)
  end,
  google_photos = function(url, cb)
    fetch_page_image(url, function(image, err)
      cb(image and M.google_photos_thumb(image), err)
    end, true)
  end,
  twitter = function(url, cb)
    local id = tweet_id(url)
    if not id then
      return cb(nil, false)
    end
    fetch_json("https://api.fxtwitter.com/status/" .. id, function(json, err)
      cb(M.tweet_thumb(json), err)
    end)
  end,
  -- Neither has a thumbnail in its oEmbed: Speaker Deck never had one, and
  -- SlideShare's oEmbed answers 404 for current `/slideshow/...` URLs.
  speakerdeck = fetch_page_image,
  slideshare = fetch_page_image,
  other = fetch_page_image,
}

local function cache_dir()
  return vim.fn.stdpath("cache") .. "/patto/embeds"
end

local function cached(base)
  local uv = vim.uv or vim.loop
  for _, ext in ipairs(CACHED) do
    local path = base .. "." .. ext
    if uv.fs_stat(path) then
      return path, ext
    end
  end
end

-- A page without a thumbnail is remembered on disk so it is not asked again
-- every session; network errors are only remembered until Neovim exits.
local function mark_none(base)
  local f = io.open(base .. ".none", "w")
  if f then
    f:close()
  end
end

local function download(thumb, base, cb)
  local part = base .. ".part"
  curl({ "-o", part, "-w", "%{content_type}", thumb }, function(r)
    local uv = vim.uv or vim.loop
    if r.code ~= 0 then
      uv.fs_unlink(part)
      return cb(true)
    end
    local ext = EXTENSIONS[(r.stdout:match("^[^;]*") or ""):lower()]
    if ext then
      uv.fs_rename(part, base .. "." .. ext)
    else
      uv.fs_unlink(part)
      mark_none(base)
    end
    cb(false)
  end)
end

local pending, failed = {}, {}

function M.thumbnail(url, on_ready)
  local kind = M.kind(url)
  local lookup = kind and lookups[kind]
  if not lookup then
    return nil, "skipped"
  end
  local base = cache_dir() .. "/" .. vim.fn.sha256(url)
  local path, ext = cached(base)
  if path then
    if ext == "none" then
      return nil, "skipped"
    end
    return path
  end
  if failed[url] then
    return nil, "skipped"
  end
  if pending[url] then
    return nil, "pending"
  end
  pending[url] = true
  vim.fn.mkdir(cache_dir(), "p")
  local function finish(err)
    pending[url] = nil
    failed[url] = err or nil
    on_ready()
  end
  lookup(url, function(thumb, err)
    if thumb then
      download(thumb, base, finish)
    else
      if not err then
        mark_none(base)
      end
      finish(err)
    end
  end)
  return nil, "pending"
end

return M
