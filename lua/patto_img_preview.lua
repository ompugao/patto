--- patto_img_preview.lua
--- Renders images (and in the future, math) in patto note buffers using vim.ui.img.
---
--- The LSP server sends a `patto/mediaItems` notification whenever a buffer is
--- opened or changed.  This module listens for that notification, resolves each
--- image path / URL to a local PNG file, and displays it via vim.ui.img.set().
---
--- State is kept per-buffer so that when the buffer content changes, previously
--- shown images are removed before new ones are placed.

local M = {}

--- { [bufnr] = { [line] = { id = <img_id>, src = <resolved_path> } } }
M._state = {}

--- Whether the module is globally enabled.
M._enabled = true

-- ---------------------------------------------------------------------------
-- Helpers
-- ---------------------------------------------------------------------------

--- Return the directory that contains the file loaded in `bufnr`.
--- Falls back to cwd if the buffer has no associated file.
local function buf_dir(bufnr)
  local path = vim.api.nvim_buf_get_name(bufnr)
  if path == '' then
    return vim.loop.cwd()
  end
  return vim.fn.fnamemodify(path, ':h')
end

--- Simple hash of a string — used to build stable /tmp filenames for URLs.
local function str_hash(s)
  local h = 5381
  for i = 1, #s do
    h = (h * 33 + s:byte(i)) % 0x100000000
  end
  return string.format('%08x', h)
end

--- Resolve an image `src` (relative path or URL) to an absolute local path.
--- * Relative paths are resolved against the buffer's directory.
--- * URLs are left as-is; the caller must download them first.
local function resolve_src(src, bufnr)
  if src:match('^https?://') then
    return src  -- handled separately via download
  end
  if src:match('^/') then
    return src  -- already absolute
  end
  -- Relative path
  return buf_dir(bufnr) .. '/' .. src
end

--- Derive a /tmp cache path for a URL.
local function url_cache_path(url)
  return '/tmp/patto_img_' .. str_hash(url) .. '.png'
end

-- ---------------------------------------------------------------------------
-- Core rendering
-- ---------------------------------------------------------------------------

--- Remove all images tracked for `bufnr`.
local function clear_buf_images(bufnr)
  local buf_state = M._state[bufnr]
  if not buf_state then return end
  for _, entry in pairs(buf_state) do
    if entry.id then
      pcall(vim.ui.img.del, entry.id)
    end
  end
  M._state[bufnr] = {}
end

--- Display a PNG file at the given buffer line (0-indexed).
--- Returns the vim.ui.img id, or nil on failure.
local function show_image_at_line(path, line, bufnr, win)
  -- Check the file actually exists and is readable.
  if vim.fn.filereadable(path) == 0 then
    return nil
  end

  local ok, bytes = pcall(vim.fn.readblob, path)
  if not ok or not bytes or #bytes == 0 then
    return nil
  end

  -- Convert to string if readblob returned a Blob object.
  if type(bytes) ~= 'string' then
    ok, bytes = pcall(tostring, bytes)
    if not ok then return nil end
  end

  -- row/col in vim.ui.img are 1-indexed screen coordinates.
  -- We use the window's screen position for the line.
  local screen_row = vim.fn.win_screenpos(win)[1] + line  -- win top + 0-indexed line offset
  local screen_col = vim.fn.win_screenpos(win)[2]

  -- Constrain dimensions to something reasonable (configurable via M.config).
  local cfg = M.config
  local ok2, id = pcall(vim.ui.img.set, bytes, {
    row    = screen_row,
    col    = screen_col,
    width  = cfg.width,
    height = cfg.height,
    zindex = cfg.zindex,
  })
  if not ok2 then return nil end
  return id
end

--- Download a URL to its cache path asynchronously, then call `cb(path)`.
local function download_url(url, cb)
  local dest = url_cache_path(url)
  -- If already cached, use it immediately.
  if vim.fn.filereadable(dest) == 1 then
    cb(dest)
    return
  end
  vim.system(
    { 'curl', '-fsSL', '-o', dest, '--', url },
    { detach = false },
    function(result)
      vim.schedule(function()
        if result.code == 0 then
          cb(dest)
        end
        -- On failure we silently skip — no partial file left.
      end)
    end
  )
end

--- Process a single media item for `bufnr` in `win`.
--- `item` is the table from the LSP notification:
---   { kind, src, content, inline, line, character }
local function process_item(item, bufnr, win)
  if item.kind ~= 'image' then
    -- Math rendering is not yet implemented.
    return
  end

  local src = item.src
  if not src or src == '' then return end

  local line = item.line  -- 0-indexed

  local function place(path)
    -- Remove stale image at this line, if any.
    local buf_state = M._state[bufnr] or {}
    local old = buf_state[line]
    if old and old.id then
      pcall(vim.ui.img.del, old.id)
    end

    local id = show_image_at_line(path, line, bufnr, win)
    buf_state[line] = { id = id, src = src }
    M._state[bufnr] = buf_state
  end

  if src:match('^https?://') then
    download_url(src, place)
  else
    local abs = resolve_src(src, bufnr)
    place(abs)
  end
end

-- ---------------------------------------------------------------------------
-- Public API
-- ---------------------------------------------------------------------------

--- Configuration (can be overridden by the user before calling setup()).
M.config = {
  width  = 40,   -- image width in terminal cells
  height = 10,   -- image height in terminal cells
  zindex = 30,
}

--- Called from the patto/mediaItems LSP notification handler.
--- `result` is { uri: string, items: [...] }
function M.on_media_items(result, ctx)
  if not M._enabled then return end
  if not result or not result.items then return end

  local bufnr = ctx and ctx.bufnr
  if not bufnr or bufnr == 0 then
    -- Resolve bufnr from URI.
    local fname = vim.uri_to_fname(result.uri)
    bufnr = vim.fn.bufnr(fname)
  end
  if not bufnr or bufnr < 1 then return end
  if not vim.api.nvim_buf_is_loaded(bufnr) then return end

  -- Find a window displaying this buffer.
  local win = nil
  for _, w in ipairs(vim.api.nvim_list_wins()) do
    if vim.api.nvim_win_get_buf(w) == bufnr then
      win = w
      break
    end
  end
  if not win then return end

  -- Remove all previously shown images for this buffer first.
  clear_buf_images(bufnr)

  for _, item in ipairs(result.items) do
    process_item(item, bufnr, win)
  end
end

--- Attach image preview to a buffer.  Called from on_attach in patto.lua.
function M.attach(bufnr)
  -- Register a BufWipeout autocmd to clean up images when the buffer closes.
  vim.api.nvim_create_autocmd('BufWipeout', {
    buffer  = bufnr,
    once    = true,
    callback = function()
      clear_buf_images(bufnr)
      M._state[bufnr] = nil
    end,
  })
end

--- Remove all images for a buffer and unregister its state.
function M.detach(bufnr)
  clear_buf_images(bufnr)
  M._state[bufnr] = nil
end

--- Toggle image preview on/off globally.
function M.toggle()
  M._enabled = not M._enabled
  if not M._enabled then
    -- Remove all images across all buffers.
    for bufnr in pairs(M._state) do
      clear_buf_images(bufnr)
    end
  end
  vim.notify(
    'Patto image preview: ' .. (M._enabled and 'enabled' or 'disabled'),
    vim.log.levels.INFO
  )
end

--- Setup: register the LSP notification handler.
--- Call this once (e.g. from your plugin config), not per-buffer.
function M.setup(opts)
  if opts then
    M.config = vim.tbl_extend('force', M.config, opts)
  end

  -- Register the handler for the custom LSP notification.
  vim.lsp.handlers['patto/mediaItems'] = function(err, result, ctx, _config)
    if err then return end
    M.on_media_items(result, ctx)
  end
end

return M
