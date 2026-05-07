--- patto_img_preview.lua
--- Shows images inline (or fullscreen) when the cursor rests on a patto
--- image line.  Handles JPG/GIF/WebP by converting to PNG via ImageMagick.
---
--- Features:
---   • Cursor-driven: image appears when cursor enters an image line, hides
---     when it leaves.
---   • Scroll tracking: WinScrolled repositions the image so it never drifts.
---   • Aspect-ratio preserving: reads pixel dims from PNG header; estimates
---     terminal cell size to compute correct cell width/height.
---   • Inline ↔ fullscreen ↔ hidden: cycle with <CR> on an image line.
---   • Multiple images on one line: cycle with ]i / [i.
---   • Non-PNG formats converted to PNG via `convert` (ImageMagick).

local M = {}

-- ---------------------------------------------------------------------------
-- State
-- ---------------------------------------------------------------------------

-- { [bufnr] = { [line0] = { {src, path}, ... } } }
-- Each line maps to an ordered list of image items (multiple embeds possible).
M._items = {}

-- { [bufnr] = { id, line0, idx, mode } }
-- Currently visible image per buffer.
--   id   : vim.ui.img id
--   line0: 0-indexed buffer line
--   idx  : 1-indexed position in the items list for that line
--   mode : "inline" | "fullscreen"
M._shown = {}

M._enabled = true

-- ---------------------------------------------------------------------------
-- Config
-- ---------------------------------------------------------------------------

M.config = {
  inline_height = 10,   -- cell height for inline mode
  zindex        = 30,
  cycle_next    = ']i', -- key to cycle to next image on the same line
  cycle_prev    = '[i', -- key to cycle to previous image on the same line
}

-- ---------------------------------------------------------------------------
-- Helpers: filesystem / hashing
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
  return '/tmp/patto_img_' .. str_hash(url)
  -- extension added after we know the format
end

local function png_cache_path(original_path)
  return '/tmp/patto_img_png_' .. str_hash(original_path) .. '.png'
end

-- ---------------------------------------------------------------------------
-- Helpers: PNG dimensions
-- ---------------------------------------------------------------------------

-- Read pixel width/height from the IHDR chunk of a PNG file.
-- Returns (w, h) or (nil, nil) on failure.
local function read_png_dimensions(path)
  local f = io.open(path, 'rb')
  if not f then return nil, nil end
  -- PNG signature: 8 bytes; IHDR chunk: 4 len + 4 type + 4 w + 4 h + ...
  local header = f:read(24)
  f:close()
  if not header or #header < 24 then return nil, nil end
  -- Bytes 17-20: width, 21-24: height (big-endian u32, 1-indexed in Lua)
  local function u32be(s, i)
    local a, b, c, d = s:byte(i, i+3)
    return a * 0x1000000 + b * 0x10000 + c * 0x100 + d
  end
  return u32be(header, 17), u32be(header, 21)
end

-- Estimate terminal cell size in pixels.
-- Uses TIOCGWINSZ via vim.fn if available, otherwise falls back to a
-- reasonable default (8×16 px per cell).
local function cell_px()
  -- Try to get pixel dimensions from the terminal via tput / stty.
  -- We use a best-effort approach: on most modern terminals xterm-compatible
  -- CSI sequences report this, but we can approximate from known resolution.
  -- Fallback: 8 wide, 16 tall (common for 1080p terminals at default font).
  local cell_w, cell_h = 8, 16
  -- vim.api.nvim_list_uis() may carry cell pixel info in future Nvim.
  local uis = vim.api.nvim_list_uis()
  if uis and uis[1] then
    local ui = uis[1]
    -- Neovim 0.10+ exposes cell pixel size when the UI provides it.
    if ui.cell_width and ui.cell_height then
      cell_w = ui.cell_width
      cell_h = ui.cell_height
    end
  end
  return cell_w, cell_h
end

-- Compute (width_cells, height_cells) that best fit the image within the
-- given cell budget while preserving the pixel aspect ratio.
local function fit_cells(px_w, px_h, max_w_cells, max_h_cells)
  local cell_w, cell_h = cell_px()
  -- Pixel budget
  local budget_px_w = max_w_cells * cell_w
  local budget_px_h = max_h_cells * cell_h
  -- Scale to fit within budget
  local scale = math.min(budget_px_w / px_w, budget_px_h / px_h)
  local out_px_w = math.floor(px_w * scale)
  local out_px_h = math.floor(px_h * scale)
  -- Convert back to cells (at least 1)
  local out_w = math.max(1, math.floor(out_px_w / cell_w))
  local out_h = math.max(1, math.floor(out_px_h / cell_h))
  return out_w, out_h
end

-- ---------------------------------------------------------------------------
-- Format conversion
-- ---------------------------------------------------------------------------

-- Ensure `src_path` is a PNG.  Returns the PNG path (may equal src_path if
-- already PNG), or nil on failure.  Synchronous — called only after we have
-- the file on disk.
local function ensure_png(src_path, cb)
  -- Detect by magic bytes.
  local f = io.open(src_path, 'rb')
  if not f then cb(nil); return end
  local magic = f:read(4)
  f:close()
  if not magic then cb(nil); return end

  local is_png = magic:sub(1,4) == '\x89PNG'
  if is_png then cb(src_path); return end

  -- Need conversion.
  local dest = png_cache_path(src_path)
  if vim.fn.filereadable(dest) == 1 then cb(dest); return end

  vim.system(
    { 'convert', src_path, dest },
    { detach = false },
    function(result)
      vim.schedule(function()
        if result.code == 0 and vim.fn.filereadable(dest) == 1 then
          cb(dest)
        else
          cb(nil)
        end
      end)
    end
  )
end

-- ---------------------------------------------------------------------------
-- Core display
-- ---------------------------------------------------------------------------

local function hide_current(bufnr)
  local shown = M._shown[bufnr]
  if shown and shown.id then
    pcall(vim.ui.img.del, shown.id)
  end
  M._shown[bufnr] = nil
end

-- Compute screen position and cell dimensions for the given mode.
-- Returns { row, col, width, height } or nil if line is off-screen.
local function image_opts(line0, win, mode)
  local cfg = M.config
  local winpos = vim.fn.win_screenpos(win)   -- {screen_row, screen_col}, 1-indexed
  local win_h  = vim.api.nvim_win_get_height(win)
  local win_w  = vim.api.nvim_win_get_width(win)

  if mode == 'fullscreen' then
    return {
      row    = winpos[1],
      col    = winpos[2],
      width  = win_w,
      height = win_h,
    }
  end

  -- inline: image sits at the line's screen row
  -- Convert buffer line → screen row within window via screenpos().
  local spos = vim.fn.screenpos(win, line0 + 1, 1)  -- line0 is 0-indexed
  if not spos or spos.row == 0 then return nil end    -- line is off-screen

  local max_h = math.min(cfg.inline_height, win_h - (spos.row - winpos[1]))
  if max_h < 1 then return nil end

  return {
    row    = spos.row,
    col    = winpos[2],
    width  = win_w,     -- will be refined by aspect ratio below
    height = max_h,
  }
end

-- Place or reposition the image for `bufnr`.
-- If `shown` is given (existing entry), we update its position in-place.
local function place(png_path, line0, idx, win, bufnr, mode, existing_shown)
  local opts = image_opts(line0, win, mode)
  if not opts then return end

  -- Refine width/height using aspect ratio (PNG dims).
  local px_w, px_h = read_png_dimensions(png_path)
  if px_w and px_h and px_w > 0 and px_h > 0 then
    local w, h = fit_cells(px_w, px_h, opts.width, opts.height)
    opts.width  = w
    opts.height = h
  end

  if existing_shown and existing_shown.id then
    -- Update position of existing image.
    pcall(vim.ui.img.set, existing_shown.id, opts)
    existing_shown.line0 = line0
    existing_shown.mode  = mode
  else
    -- New image.
    local ok, bytes = pcall(vim.fn.readblob, png_path)
    if not ok or not bytes or #bytes == 0 then return end
    if type(bytes) ~= 'string' then
      local ok2; ok2, bytes = pcall(tostring, bytes)
      if not ok2 then return end
    end
    local ok3, id = pcall(vim.ui.img.set, bytes, opts)
    if not ok3 then return end
    M._shown[bufnr] = { id = id, line0 = line0, idx = idx, mode = mode }
  end
end

-- Resolve src → local file → PNG → display.
-- `on_ready` is called after async steps complete; used to check cursor hasn't moved.
local function resolve_and_show(item, line0, idx, win, bufnr, mode)
  local src = item.src

  local function show_png(png_path)
    -- Cache the resolved PNG path for instant repositioning later.
    item.png = png_path
    place(png_path, line0, idx, win, bufnr, mode, nil)
  end

  local function got_local(local_path)
    item.local_path = local_path
    ensure_png(local_path, function(png_path)
      if png_path then show_png(png_path) end
    end)
  end

  -- If we already have the PNG cached, skip resolution steps.
  if item.png then
    place(item.png, line0, idx, win, bufnr, mode, nil)
    return
  end

  if src:match('^https?://') then
    -- Derive a download path that preserves the original extension.
    local ext = src:match('%.([^%.%?#]+)%??') or 'bin'
    local dl_path = url_cache_path(src) .. '.' .. ext
    item.local_path = dl_path
    if vim.fn.filereadable(dl_path) == 1 then
      got_local(dl_path)
      return
    end
    vim.system(
      { 'curl', '-fsSL', '-o', dl_path, '--', src },
      { detach = false },
      function(result)
        vim.schedule(function()
          -- Only show if cursor is still on this line.
          local cur_win = vim.api.nvim_get_current_win()
          if result.code == 0
              and vim.api.nvim_win_get_buf(cur_win) == bufnr
              and vim.fn.line('.') - 1 == line0 then
            got_local(dl_path)
          elseif result.code ~= 0 then
            item.local_path = nil
          end
        end)
      end
    )
  else
    got_local(resolve_local(src, bufnr))
  end
end

-- ---------------------------------------------------------------------------
-- Show / reposition / hide
-- ---------------------------------------------------------------------------

-- Show image `idx` (1-based) for `line0` in `bufnr`/`win`, in `mode`.
local function show_image(line0, idx, win, bufnr, mode)
  local line_items = (M._items[bufnr] or {})[line0]
  if not line_items or #line_items == 0 then return end
  idx = ((idx - 1) % #line_items) + 1  -- wrap

  hide_current(bufnr)

  local item = line_items[idx]
  if item.png then
    place(item.png, line0, idx, win, bufnr, mode, nil)
  else
    resolve_and_show(item, line0, idx, win, bufnr, mode)
  end
end

-- Reposition the currently shown image after a scroll.
local function reposition_current(bufnr, win)
  local shown = M._shown[bufnr]
  if not shown or not shown.id then return end
  local line_items = (M._items[bufnr] or {})[shown.line0]
  if not line_items then return end
  local item = line_items[shown.idx]
  if not item or not item.png then return end
  place(item.png, shown.line0, shown.idx, win, bufnr, shown.mode, shown)
end

-- ---------------------------------------------------------------------------
-- Cursor / scroll callbacks
-- ---------------------------------------------------------------------------

local function on_cursor_moved(bufnr)
  if not M._enabled then return end
  local items = M._items[bufnr]
  if not items then return end

  local line0 = vim.fn.line('.') - 1
  local shown = M._shown[bufnr]

  -- Same line, same image → just reposition (handles inline scroll via cursor).
  if shown and shown.line0 == line0 then
    local win = vim.api.nvim_get_current_win()
    reposition_current(bufnr, win)
    return
  end

  hide_current(bufnr)

  local line_items = items[line0]
  if not line_items or #line_items == 0 then return end

  local win = vim.api.nvim_get_current_win()
  if vim.api.nvim_win_get_buf(win) ~= bufnr then return end
  show_image(line0, 1, win, bufnr, 'inline')
end

local function on_win_scrolled(bufnr, win)
  if not M._enabled then return end
  local shown = M._shown[bufnr]
  if not shown then return end
  reposition_current(bufnr, win)
end

-- Cycle to next (+1) or previous (-1) image on the current line.
local function cycle_image(bufnr, delta)
  local shown = M._shown[bufnr]
  if not shown then return end
  local win = vim.api.nvim_get_current_win()
  show_image(shown.line0, shown.idx + delta, win, bufnr, shown.mode)
end

-- Toggle fullscreen ↔ inline for the current image; if no image shown,
-- try to show the one at the cursor line.
local function toggle_size(bufnr)
  if not M._enabled then return end
  local win = vim.api.nvim_get_current_win()
  local line0 = vim.fn.line('.') - 1
  local shown = M._shown[bufnr]

  if not shown then
    local line_items = (M._items[bufnr] or {})[line0]
    if line_items and #line_items > 0 then
      show_image(line0, 1, win, bufnr, 'fullscreen')
    end
    return
  end

  -- Cycle: inline → fullscreen → hidden
  if shown.mode == 'inline' then
    local idx = shown.idx
    local l0  = shown.line0
    hide_current(bufnr)
    show_image(l0, idx, win, bufnr, 'fullscreen')
  elseif shown.mode == 'fullscreen' then
    hide_current(bufnr)
  end
end

-- ---------------------------------------------------------------------------
-- Public API
-- ---------------------------------------------------------------------------

function M.on_media_items(result, ctx)
  if not M._enabled then return end
  if not result or not result.items then return end

  local bufnr = ctx and ctx.bufnr
  if not bufnr or bufnr == 0 then
    bufnr = vim.fn.bufnr(vim.uri_to_fname(result.uri))
  end
  if not bufnr or bufnr < 1 then return end
  if not vim.api.nvim_buf_is_loaded(bufnr) then return end

  hide_current(bufnr)

  local old_items = M._items[bufnr] or {}
  local new_items = {}

  for _, item in ipairs(result.items) do
    if item.kind == 'image' and item.src and item.src ~= '' then
      local line0 = item.line
      if not new_items[line0] then new_items[line0] = {} end
      -- Preserve cached paths if src unchanged.
      local old_list = old_items[line0] or {}
      local idx = #new_items[line0] + 1
      local old = old_list[idx]
      local entry = { src = item.src }
      if old and old.src == item.src then
        entry.local_path = old.local_path
        entry.png        = old.png
      end
      table.insert(new_items[line0], entry)
    end
  end

  M._items[bufnr] = new_items

  -- Show immediately if cursor is already on an image line.
  local win = vim.fn.bufwinid(bufnr)
  if win ~= -1 then
    local cur_line0 = vim.fn.line('.') - 1
    local line_items = new_items[cur_line0]
    if line_items and #line_items > 0 then
      show_image(cur_line0, 1, win, bufnr, 'inline')
    end
  end
end

function M.attach(bufnr)
  local aug = vim.api.nvim_create_augroup('PattoImgPreview_' .. bufnr, { clear = true })

  vim.api.nvim_create_autocmd({ 'CursorMoved', 'CursorMovedI' }, {
    group    = aug,
    buffer   = bufnr,
    callback = function() on_cursor_moved(bufnr) end,
  })

  vim.api.nvim_create_autocmd('WinScrolled', {
    group    = aug,
    buffer   = bufnr,
    callback = function()
      local win = vim.api.nvim_get_current_win()
      if vim.api.nvim_win_get_buf(win) == bufnr then
        on_win_scrolled(bufnr, win)
      end
    end,
  })

  vim.api.nvim_create_autocmd('BufWipeout', {
    group    = aug,
    buffer   = bufnr,
    once     = true,
    callback = function()
      hide_current(bufnr)
      M._items[bufnr] = nil
      M._shown[bufnr] = nil
      vim.api.nvim_del_augroup_by_id(aug)
    end,
  })

  local cfg = M.config
  -- <CR>: cycle inline → fullscreen → hidden
  vim.keymap.set('n', '<CR>', function() toggle_size(bufnr) end,
    { buffer = bufnr, desc = 'Patto: toggle image inline/fullscreen' })
  -- ]i / [i: cycle multiple images on same line
  vim.keymap.set('n', cfg.cycle_next, function() cycle_image(bufnr, 1) end,
    { buffer = bufnr, desc = 'Patto: next image on line' })
  vim.keymap.set('n', cfg.cycle_prev, function() cycle_image(bufnr, -1) end,
    { buffer = bufnr, desc = 'Patto: previous image on line' })
end

function M.detach(bufnr)
  hide_current(bufnr)
  M._items[bufnr] = nil
  M._shown[bufnr] = nil
  pcall(vim.api.nvim_del_augroup_by_name, 'PattoImgPreview_' .. bufnr)
  pcall(vim.keymap.del, 'n', '<CR>',       { buffer = bufnr })
  pcall(vim.keymap.del, 'n', M.config.cycle_next, { buffer = bufnr })
  pcall(vim.keymap.del, 'n', M.config.cycle_prev, { buffer = bufnr })
end

function M.toggle()
  M._enabled = not M._enabled
  if not M._enabled then
    for bufnr in pairs(M._shown) do hide_current(bufnr) end
  end
  vim.notify('Patto image preview: ' .. (M._enabled and 'enabled' or 'disabled'),
    vim.log.levels.INFO)
end

function M.setup(opts)
  if opts then M.config = vim.tbl_extend('force', M.config, opts) end
  vim.lsp.handlers['patto/mediaItems'] = function(err, result, ctx, _)
    if err then return end
    M.on_media_items(result, ctx)
  end
end

return M
