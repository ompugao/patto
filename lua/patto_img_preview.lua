--- patto_img_preview.lua
--- Shows an image (or in the future: rendered math) inline when the cursor
--- rests on a line containing a patto image node.
---
--- Data flow:
---   1. LSP server sends `patto/mediaItems` on every open/change.
---   2. We store the item list keyed by (bufnr, 0-indexed line).
---   3. A CursorMoved autocmd checks the current line; if it has an image we
---      show it via vim.ui.img.set().  When the cursor leaves, we delete it.
---
--- Only one image is visible at a time per buffer (the one under the cursor).

local M = {}

--- { [bufnr] = { [line0] = { src=string, path=string|nil } } }
--- `path` is the resolved local path (set after first resolution/download).
M._items = {}

--- { [bufnr] = { id=integer, line=integer } }  – currently shown image per buf
M._shown = {}

M._enabled = true

-- ---------------------------------------------------------------------------
-- Helpers
-- ---------------------------------------------------------------------------

local function buf_dir(bufnr)
  local p = vim.api.nvim_buf_get_name(bufnr)
  if p == '' then return vim.uv.cwd() end
  return vim.fn.fnamemodify(p, ':h')
end

local function str_hash(s)
  local h = 5381
  for i = 1, #s do h = (h * 33 + s:byte(i)) % 0x100000000 end
  return string.format('%08x', h)
end

local function resolve_local(src, bufnr)
  if src:match('^/') then return src end
  return buf_dir(bufnr) .. '/' .. src
end

local function url_cache_path(url)
  return '/tmp/patto_img_' .. str_hash(url) .. '.png'
end

-- ---------------------------------------------------------------------------
-- Image display
-- ---------------------------------------------------------------------------

--- Hide the currently shown image for `bufnr`, if any.
local function hide_current(bufnr)
  local shown = M._shown[bufnr]
  if shown and shown.id then
    pcall(vim.ui.img.del, shown.id)
  end
  M._shown[bufnr] = nil
end

--- Place PNG bytes on screen at the cursor line in `win`.
--- `line0` is the 0-indexed buffer line the image belongs to.
local function place_image(bytes, line0, win, bufnr)
  -- win_screenpos returns {row, col}, both 1-indexed.
  local winpos = vim.fn.win_screenpos(win)
  -- Convert the buffer line to the screen row it occupies in the window.
  -- vim.fn.line() is 1-indexed; we need the visual row of line0+1.
  local vis_row = vim.fn.winline()  -- row of cursor within window (1-indexed)
  local screen_row = winpos[1] + vis_row - 1
  local screen_col = winpos[2]

  local cfg = M.config
  local ok, id = pcall(vim.ui.img.set, bytes, {
    row    = screen_row,
    col    = screen_col,
    width  = cfg.width,
    height = cfg.height,
    zindex = cfg.zindex,
  })
  if not ok then return end
  M._shown[bufnr] = { id = id, line = line0 }
end

--- Read a local PNG and display it; stores id in M._shown[bufnr].
local function show_local(path, line0, win, bufnr)
  if vim.fn.filereadable(path) == 0 then return end
  local ok, bytes = pcall(vim.fn.readblob, path)
  if not ok or not bytes or #bytes == 0 then return end
  if type(bytes) ~= 'string' then
    ok, bytes = pcall(tostring, bytes)
    if not ok then return end
  end
  place_image(bytes, line0, win, bufnr)
end

--- Ensure the image for `item` at `line0` in `bufnr`/`win` is visible.
--- Resolves path on first call; subsequent calls reuse the cached path.
local function show_item(item, line0, win, bufnr)
  local src = item.src
  if not src or src == '' then return end

  -- If path already resolved, show immediately.
  if item.path then
    show_local(item.path, line0, win, bufnr)
    return
  end

  if src:match('^https?://') then
    local dest = url_cache_path(src)
    item.path = dest  -- optimistically cache; if download fails, file won't exist
    if vim.fn.filereadable(dest) == 1 then
      show_local(dest, line0, win, bufnr)
      return
    end
    vim.system(
      { 'curl', '-fsSL', '-o', dest, '--', src },
      { detach = false },
      function(result)
        vim.schedule(function()
          if result.code == 0 then
            -- Re-check cursor is still on that line.
            local cur_win = vim.api.nvim_get_current_win()
            if vim.api.nvim_win_get_buf(cur_win) == bufnr
                and vim.fn.line('.') - 1 == line0 then
              hide_current(bufnr)
              show_local(dest, line0, cur_win, bufnr)
            end
          else
            item.path = nil  -- reset so we retry next time
          end
        end)
      end
    )
  else
    item.path = resolve_local(src, bufnr)
    show_local(item.path, line0, win, bufnr)
  end
end

-- ---------------------------------------------------------------------------
-- Cursor tracking
-- ---------------------------------------------------------------------------

--- Called on CursorMoved / CursorMovedI for a patto buffer.
local function on_cursor_moved(bufnr)
  if not M._enabled then return end
  local items = M._items[bufnr]
  if not items then return end

  local line0 = vim.fn.line('.') - 1  -- 0-indexed

  -- Already showing the right image — nothing to do.
  local shown = M._shown[bufnr]
  if shown and shown.line == line0 then return end

  -- Hide whatever is currently shown.
  hide_current(bufnr)

  local item = items[line0]
  if not item then return end

  local win = vim.api.nvim_get_current_win()
  if vim.api.nvim_win_get_buf(win) ~= bufnr then return end

  show_item(item, line0, win, bufnr)
end

-- ---------------------------------------------------------------------------
-- Public API
-- ---------------------------------------------------------------------------

M.config = {
  width  = 40,
  height = 10,
  zindex = 30,
}

--- Called by the `patto/mediaItems` LSP notification handler.
function M.on_media_items(result, ctx)
  if not M._enabled then return end
  if not result or not result.items then return end

  local bufnr = ctx and ctx.bufnr
  if not bufnr or bufnr == 0 then
    bufnr = vim.fn.bufnr(vim.uri_to_fname(result.uri))
  end
  if not bufnr or bufnr < 1 then return end
  if not vim.api.nvim_buf_is_loaded(bufnr) then return end

  -- Hide any currently shown image — item positions may have changed.
  hide_current(bufnr)

  -- Rebuild item index for this buffer.
  -- Preserve already-resolved paths so we don't re-download.
  local old_items = M._items[bufnr] or {}
  local new_items = {}
  for _, item in ipairs(result.items) do
    if item.kind == 'image' and item.src and item.src ~= '' then
      local line0 = item.line
      local old = old_items[line0]
      -- Reuse cached path if src hasn't changed.
      local path = (old and old.src == item.src) and old.path or nil
      new_items[line0] = { src = item.src, path = path }
    end
  end
  M._items[bufnr] = new_items

  -- If the cursor is already sitting on an image line, show it now.
  local win = vim.fn.bufwinid(bufnr)
  if win ~= -1 then
    local cur_line0 = vim.fn.line('.') - 1
    local item = new_items[cur_line0]
    if item then
      show_item(item, cur_line0, win, bufnr)
    end
  end
end

--- Attach preview behaviour to a buffer (called from on_attach).
function M.attach(bufnr)
  local augroup = vim.api.nvim_create_augroup('PattoImgPreview_' .. bufnr, { clear = true })

  vim.api.nvim_create_autocmd({ 'CursorMoved', 'CursorMovedI' }, {
    group   = augroup,
    buffer  = bufnr,
    callback = function() on_cursor_moved(bufnr) end,
  })

  vim.api.nvim_create_autocmd('BufWipeout', {
    group   = augroup,
    buffer  = bufnr,
    once    = true,
    callback = function()
      hide_current(bufnr)
      M._items[bufnr] = nil
      M._shown[bufnr] = nil
      vim.api.nvim_del_augroup_by_id(augroup)
    end,
  })
end

function M.detach(bufnr)
  hide_current(bufnr)
  M._items[bufnr] = nil
  M._shown[bufnr] = nil
  pcall(vim.api.nvim_del_augroup_by_name, 'PattoImgPreview_' .. bufnr)
end

function M.toggle()
  M._enabled = not M._enabled
  if not M._enabled then
    for bufnr in pairs(M._shown) do hide_current(bufnr) end
  end
  vim.notify('Patto image preview: ' .. (M._enabled and 'enabled' or 'disabled'), vim.log.levels.INFO)
end

function M.setup(opts)
  if opts then M.config = vim.tbl_extend('force', M.config, opts) end
  vim.lsp.handlers['patto/mediaItems'] = function(err, result, ctx, _)
    if err then return end
    M.on_media_items(result, ctx)
  end
end

return M
