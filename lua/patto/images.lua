-- Inline image preview for `[@img ...]` lines, rendered through snacks.nvim.
-- snacks' own document mode needs a tree-sitter parser, which patto does not
-- have, so this module locates images itself and drives snacks' placement API.
-- The locator is a loose heuristic (the path token inside the brackets), not a
-- mirror of the patto grammar.
local M = {}

local managers = {}
local hover
local composite_ns = vim.api.nvim_create_namespace("patto.images")
M._managers = managers

local function is_url(src)
  return src:find("^%w%w+://") ~= nil
end

local function closing_bracket(line, from)
  local i, quoted = from, false
  while i <= #line do
    local c = line:sub(i, i)
    if quoted then
      if c == "\\" then
        i = i + 1
      elseif c == '"' then
        quoted = false
      end
    elseif c == '"' then
      quoted = true
    elseif c == "]" then
      return i
    end
    i = i + 1
  end
end

local function path_token(args)
  args = args:gsub('"[^"]*"', " ")
  for token in args:gmatch("%S+") do
    if token:find("^%.%.?/") or token:find("^%a[%w+.-]*://") then
      return token
    end
  end
end

function M.parse_line(line)
  local found, init = {}, 1
  while true do
    local s, e = line:find("%[@img[ \t]+", init)
    if not s then
      break
    end
    local close = closing_bracket(line, e + 1)
    if not close then
      break
    end
    local src = path_token(line:sub(e + 1, close - 1))
    if src then
      found[#found + 1] = { col = s - 1, end_col = close, src = src }
      init = close + 1
    else
      init = e + 1
    end
  end
  return found
end

function M.find(lines, first_row)
  local found = {}
  for i, line in ipairs(lines) do
    for _, m in ipairs(M.parse_line(line)) do
      m.row = first_row + i - 1
      found[#found + 1] = m
    end
  end
  return found
end

local function snacks_image()
  local ok, image = pcall(function()
    return _G.Snacks and Snacks.image
  end)
  if ok and type(image) == "table" and image.config and image.config.enabled ~= false then
    return image
  end
end

local THUMBNAIL_ROWS = 8

local function options()
  local doc = Snacks.image.config.doc
  return {
    max_width = vim.g.patto_image_max_width or doc.max_width,
    max_height = vim.g.patto_image_max_height or THUMBNAIL_ROWS,
    conceal = vim.g.patto_image_conceal == true,
  }
end

local pending, failed = {}, {}

local function magick()
  for _, cmd in ipairs({ "magick", "convert" }) do
    if vim.fn.executable(cmd) == 1 then
      return cmd
    end
  end
end

local function max_bytes()
  return vim.g.patto_image_max_bytes or 256 * 1024
end

local function scaled_path(src, stat, o)
  local size = Snacks.image.terminal.size()
  local w = math.floor(o.max_width * size.cell_width)
  local h = math.floor(o.max_height * size.cell_height)
  local key = vim.fn.sha256(table.concat({ src, stat.mtime.sec, stat.size, w, h }, ":"))
  return vim.fn.stdpath("cache") .. "/patto/images/" .. key .. ".png", w, h
end

-- Over ssh snacks streams the whole file through the terminal on the main
-- thread, so a multi-megabyte screenshot freezes the UI for seconds. Shrink
-- such files first; without ImageMagick they are skipped instead.
local function transferable(src, o, on_ready)
  local uv = vim.uv or vim.loop
  if is_url(src) or not Snacks.image.terminal.env().remote then
    return src
  end
  local stat = uv.fs_stat(src)
  if not stat or stat.size <= max_bytes() then
    return src
  end
  local cmd = magick()
  if not cmd then
    return nil, "skipped"
  end
  local out, w, h = scaled_path(src, stat, o)
  if uv.fs_stat(out) then
    return out
  end
  if failed[out] then
    return nil, "skipped"
  end
  if pending[out] then
    return nil, "pending"
  end
  pending[out] = true
  vim.fn.mkdir(vim.fs.dirname(out), "p")
  local args = { cmd, src .. "[0]", "-strip", "-resize", w .. "x" .. h .. ">", "-colors", "256", out }
  vim.system(args, {}, function(result)
    pending[out] = nil
    if result.code ~= 0 then
      failed[out] = true
    end
    vim.schedule(on_ready)
  end)
  return nil, "pending"
end

local function locate(buf, src)
  local resolved = Snacks.image.doc.resolve(buf, src)
  if not Snacks.image.supports_file(resolved) then
    return
  end
  if not is_url(resolved) and not (vim.uv or vim.loop).fs_stat(resolved) then
    return
  end
  return resolved
end

local function resolve(buf, src, o, on_ready)
  local resolved = locate(buf, src)
  return resolved and transferable(resolved, o, on_ready)
end

local function augroup_name(buf)
  return "patto.images." .. buf
end

local Inline = {}
Inline.__index = Inline

function Inline.new(buf, opts)
  local self = setmetatable({ buf = buf, opts = opts, imgs = {}, idx = {} }, Inline)
  local group = vim.api.nvim_create_augroup(augroup_name(buf), { clear = true })
  local update = Snacks.util.debounce(function()
    if not self.closed then
      self:update()
    end
  end, { ms = 100 })

  vim.api.nvim_create_autocmd({ "BufWritePost", "WinScrolled", "BufWinEnter" }, {
    group = group,
    buffer = buf,
    callback = vim.schedule_wrap(update),
  })
  if opts.conceal then
    vim.api.nvim_create_autocmd({ "ModeChanged", "CursorMoved" }, {
      group = group,
      buffer = buf,
      callback = function()
        if vim.api.nvim_get_current_buf() == buf then
          self:conceal()
        end
      end,
    })
  end
  vim.api.nvim_buf_attach(buf, false, {
    on_lines = function()
      if self.closed then
        return true
      end
      update()
    end,
  })
  vim.schedule(update)
  return self
end

function Inline:conceal()
  local mode = vim.fn.mode():sub(1, 1):lower()
  for _, img in pairs(self.imgs) do
    img:show()
  end
  if vim.wo.concealcursor:find(mode) then
    return
  end
  local from, to = vim.fn.line("v"), vim.fn.line(".")
  from, to = math.min(from, to), math.max(from, to)
  for _, img in pairs(self:get(from, to)) do
    img:hide()
  end
end

function Inline:get(from, to)
  local ret = {}
  local marks = vim.api.nvim_buf_get_extmarks(self.buf, Snacks.image.placement.ns, { from - 1, 0 }, { to, -1 }, {
    overlap = true,
    hl_name = false,
  })
  for _, m in ipairs(marks) do
    local p = self.idx[m[1]]
    if p and not self.imgs[p.id] then
      self.idx[m[1]] = nil
      p = nil
    end
    if p then
      ret[p.id] = p
    end
  end
  return ret
end

-- `botline` leaves out a last line that is only partially displayed, which is
-- often the line a tall image hangs from, so one more line is scanned.
function Inline:shown_range(win)
  local info = vim.fn.getwininfo(win)[1]
  local last = vim.api.nvim_buf_line_count(self.buf)
  return math.max(info.topline - 1, 1), math.min(info.botline + 1, last)
end

function Inline:visible()
  local ret = {}
  for _, win in ipairs(vim.fn.win_findbuf(self.buf)) do
    for k, v in pairs(self:get(self:shown_range(win))) do
      ret[k] = v
    end
  end
  return ret
end

function Inline:found()
  local found = {}
  local function retry()
    if not self.closed then
      self:update()
    end
  end
  for _, win in ipairs(vim.fn.win_findbuf(self.buf)) do
    local from, to = self:shown_range(win)
    local lines = vim.api.nvim_buf_get_lines(self.buf, from - 1, to, false)
    for _, m in ipairs(M.find(lines, from)) do
      local key = m.row .. ":" .. m.col
      if not found[key] then
        m.src = resolve(self.buf, m.src, self.opts, retry)
        if m.src then
          found[key] = m
        end
      end
    end
  end
  return found
end

function Inline:row_of(img)
  local ok, mark = pcall(vim.api.nvim_buf_get_extmark_by_id, self.buf, Snacks.image.placement.ns, img.eids[1], {})
  return ok and mark[1] and mark[1] + 1 or nil
end

function Inline:take(visible, m)
  local fallback
  for id, img in pairs(visible) do
    if img.img.src == m.src then
      if self:row_of(img) == m.row then
        visible[id] = nil
        return img
      end
      fallback = fallback or id
    end
  end
  if fallback then
    local img = visible[fallback]
    visible[fallback] = nil
    return img
  end
end

function Inline:text_width()
  local width = vim.o.columns
  for _, win in ipairs(vim.fn.win_findbuf(self.buf)) do
    local info = vim.fn.getwininfo(win)[1]
    width = math.min(width, info.width - info.textoff)
  end
  return width
end

-- snacks gives every placement its own block of virtual lines, so images that
-- share a text line stack vertically. Members of a group keep their kitty
-- placement but their virtual lines are stripped and merged into one block.
function Inline:indent(row)
  local line = vim.api.nvim_buf_get_lines(self.buf, row - 1, row, false)[1] or ""
  return vim.fn.strdisplaywidth(line:match("^%s*"))
end

function Inline:single_width(row)
  return math.max(1, math.min(self.opts.max_width, self:text_width() - self:indent(row)))
end

function Inline:new_group(row, count)
  local indent = self:indent(row)
  local per = math.floor((self:text_width() - indent - (count - 1)) / count)
  return {
    members = {},
    grids = {},
    indent = string.rep(" ", indent),
    max_width = math.max(1, math.min(self.opts.max_width, per)),
  }
end

function Inline:compose(group)
  local members = {}
  for id, p in pairs(group.members) do
    if self.imgs[id] and group.grids[id] then
      members[#members + 1] = p
    end
  end
  if #members == 0 then
    return
  end
  table.sort(members, function(a, b)
    return a.opts.pos[2] < b.opts.pos[2]
  end)
  local height = 0
  for _, p in ipairs(members) do
    height = math.max(height, #group.grids[p.id].lines)
  end
  local lines = {}
  for i = 1, height do
    local chunks = { { group.indent } }
    for j, p in ipairs(members) do
      local grid = group.grids[p.id]
      local chunk = grid.lines[i] and grid.lines[i][#grid.lines[i]]
      if chunk and not grid.hidden and chunk[1] ~= "" then
        chunks[#chunks + 1] = chunk
      else
        chunks[#chunks + 1] = { string.rep(" ", p._state and p._state.loc.width or 0) }
      end
      if j < #members then
        chunks[#chunks + 1] = { " " }
      end
    end
    lines[i] = chunks
  end
  local row = self:row_of(members[1])
  if not row then
    return
  end
  group.eid = vim.api.nvim_buf_set_extmark(self.buf, composite_ns, row - 1, 0, {
    id = group.eid,
    virt_lines = lines,
    undo_restore = false,
    strict = false,
  })
end

function Inline:join(group, img)
  local old = img.patto_group
  group.members[img.id] = img
  if old and old ~= group then
    group.eid = group.eid or old.eid
    old.eid = nil
    group.grids[img.id] = old.grids[img.id]
    old.members[img.id] = nil
  end
  img.patto_group = group
  img.opts.conceal = false
  img.opts.max_width = group.max_width
  if not group.grids[img.id] then
    img._state = nil
  end
end

function Inline:leave(img)
  local group = img.patto_group
  if not group then
    return
  end
  group.members[img.id] = nil
  group.grids[img.id] = nil
  img.patto_group = nil
  img.opts.conceal = self.opts.conceal
  img._state = nil
end

function Inline:place(m)
  local base_render = Snacks.image.placement._render
  local img = Snacks.image.placement.new(
    self.buf,
    m.src,
    Snacks.config.merge({}, Snacks.image.config.doc, {
      pos = { m.row, m.col },
      range = { m.row, m.col, m.row, m.end_col },
      inline = true,
      type = "image",
      conceal = self.opts.conceal,
      max_width = self.opts.max_width,
      max_height = self.opts.max_height,
      on_update = function(p)
        for _, eid in ipairs(p.eids) do
          self.idx[eid] = p
        end
      end,
    })
  )
  -- snacks pads virtual lines by the byte offset of the image, which puts a
  -- tab-indented image one column in; use the indentation's display width.
  img._render = function(p, extmarks)
    local group = p.patto_group
    local indent = self:indent(p.opts.pos[1])
    for _, e in ipairs(extmarks) do
      if e.virt_text_win_col then
        e.virt_text_win_col = indent
      end
      if e.virt_lines and group then
        group.grids[p.id] = { lines = e.virt_lines, hidden = p.hidden }
        e.virt_lines = nil
      elseif e.virt_lines then
        e.virt_lines = vim.tbl_map(function(l)
          return { { string.rep(" ", indent) }, l[#l] }
        end, e.virt_lines)
      end
    end
    base_render(p, extmarks)
    if group then
      self:compose(group)
    end
  end
  for _, eid in ipairs(img.eids) do
    self.idx[eid] = img
  end
  self.imgs[img.id] = img
  return img
end

-- Sending an image over ssh blocks Neovim, so only one new image is placed per
-- tick and the rest follow after a redraw.
function Inline:update()
  if not vim.api.nvim_buf_is_valid(self.buf) then
    return
  end
  local found = self:found()
  local per_row = {}
  for _, m in pairs(found) do
    per_row[m.row] = (per_row[m.row] or 0) + 1
  end
  local groups = {}
  local visible = self:visible()
  local new = {}
  for _, m in pairs(found) do
    local img = self:take(visible, m)
    if not img then
      new[#new + 1] = m
    else
      img.opts.pos = { m.row, m.col }
      img.opts.range = { m.row, m.col, m.row, m.end_col }
      if per_row[m.row] > 1 then
        groups[m.row] = groups[m.row] or self:new_group(m.row, per_row[m.row])
        self:join(groups[m.row], img)
      else
        self:leave(img)
        img.opts.max_width = self:single_width(m.row)
      end
      img:update()
    end
  end
  for _, img in pairs(visible) do
    self:leave(img)
    img:close()
    self.imgs[img.id] = nil
  end
  for _, group in pairs(self.groups or {}) do
    if group.eid and not next(group.members) then
      vim.api.nvim_buf_del_extmark(self.buf, composite_ns, group.eid)
      group.eid = nil
    end
  end
  self.groups = groups
  for _, group in pairs(groups) do
    self:compose(group)
  end
  table.sort(new, function(a, b)
    return a.row < b.row or (a.row == b.row and a.col < b.col)
  end)
  if new[1] then
    local img = self:place(new[1])
    if per_row[new[1].row] > 1 then
      groups[new[1].row] = groups[new[1].row] or self:new_group(new[1].row, per_row[new[1].row])
      self:join(groups[new[1].row], img)
    else
      img.opts.max_width = self:single_width(new[1].row)
    end
  end
  if new[2] then
    vim.defer_fn(function()
      if not self.closed then
        self:update()
      end
    end, 30)
  end
end

function Inline:close()
  self.closed = true
  pcall(vim.api.nvim_del_augroup_by_name, augroup_name(self.buf))
  for _, img in pairs(self.imgs) do
    img:close()
  end
  if vim.api.nvim_buf_is_valid(self.buf) then
    vim.api.nvim_buf_clear_namespace(self.buf, composite_ns, 0, -1)
  end
  self.imgs, self.idx, self.groups = {}, {}, {}
end

local function hover_close()
  if hover then
    hover.win:close()
    hover.img:close()
    hover = nil
  end
end

local function image_at_cursor(buf, strict)
  local cursor = vim.api.nvim_win_get_cursor(0)
  local line = vim.api.nvim_buf_get_lines(buf, cursor[1] - 1, cursor[1], false)[1] or ""
  local matches = M.parse_line(line)
  for _, m in ipairs(matches) do
    if cursor[2] >= m.col and cursor[2] < m.end_col then
      return m, cursor[1]
    end
  end
  if not strict then
    return matches[1], cursor[1]
  end
end

local function at_cursor(buf)
  local m = image_at_cursor(buf, true)
  if m then
    return resolve(buf, m.src, options(), function()
      M.hover(buf)
    end)
  end
end

local function attach_hover(buf)
  local group = vim.api.nvim_create_augroup(augroup_name(buf), { clear = true })
  vim.api.nvim_create_autocmd({ "CursorMoved", "ModeChanged", "BufLeave" }, {
    group = group,
    buffer = buf,
    callback = vim.schedule_wrap(function()
      M.hover(buf)
    end),
  })
  vim.schedule(function()
    M.hover(buf)
  end)
end

local function normalize(buf)
  return (buf == nil or buf == 0) and vim.api.nvim_get_current_buf() or buf
end

function M.detach(buf)
  buf = normalize(buf)
  if managers[buf] then
    managers[buf]:close()
    managers[buf] = nil
  end
  if hover and hover.buf == buf then
    hover_close()
  end
  pcall(vim.api.nvim_del_augroup_by_name, augroup_name(buf))
  vim.b[buf].patto_images_attached = nil
end

-- snacks keeps one kitty image per file, so a fullscreen placement of the
-- same file disturbs the thumbnail's cells; the viewer gets its own copy.
local function viewer_copy(src)
  if is_url(src) then
    return src
  end
  local uv = vim.uv or vim.loop
  local dir = vim.fn.stdpath("cache") .. "/patto/images"
  local link = dir .. "/view-" .. vim.fn.sha256(src) .. "." .. vim.fn.fnamemodify(src, ":e")
  if not uv.fs_lstat(link) then
    vim.fn.mkdir(dir, "p")
    if not uv.fs_symlink(src, link) then
      uv.fs_copyfile(src, link)
    end
  end
  return link
end

local function refresh_placements(buf)
  local manager = managers[buf]
  if manager and not manager.closed then
    for _, p in pairs(manager.imgs) do
      p._state = nil
      p:update()
    end
  end
end

-- Like Enter in patto-preview-tui: the image under the cursor fills the
-- screen in a float until q, <Esc> or <CR> closes it.
function M.view(buf)
  buf = normalize(buf)
  if not snacks_image() then
    return false
  end
  local m, row = image_at_cursor(buf, false)
  if not m then
    return false
  end
  local screen = { max_width = vim.o.columns, max_height = vim.o.lines }
  local src, state = resolve(buf, m.src, screen, function()
    if vim.api.nvim_get_current_buf() == buf and vim.api.nvim_win_get_cursor(0)[1] == row then
      M.view(buf)
    end
  end)
  if not src then
    if state == "pending" then
      vim.notify("patto: preparing image...", vim.log.levels.INFO)
    end
    return state == "pending"
  end
  local win = Snacks.win({
    position = "float",
    width = 0,
    height = 0,
    border = "none",
    backdrop = false,
    keys = { q = "close", ["<esc>"] = "close", ["<cr>"] = "close" },
    bo = { buftype = "nofile", bufhidden = "wipe" },
    on_close = function()
      vim.schedule(function()
        refresh_placements(buf)
      end)
    end,
  })
  Snacks.image.buf.attach(win.buf, { src = viewer_copy(src) })
  return true
end

function M.toggle(buf)
  buf = normalize(buf)
  if vim.b[buf].patto_images_attached then
    M.detach(buf)
  elseif not snacks_image() then
    vim.notify("patto: inline images need snacks.nvim with `image = { enabled = true }`", vim.log.levels.WARN)
  else
    M.attach(buf)
  end
end

function M.refresh(buf)
  buf = normalize(buf)
  if managers[buf] then
    managers[buf]:update()
  elseif vim.b[buf].patto_images_attached then
    M.hover(buf)
  end
end

local function ensure_commands(buf)
  if vim.b[buf].patto_images_commands then
    return
  end
  vim.b[buf].patto_images_commands = true
  vim.api.nvim_buf_create_user_command(buf, "PattoImagesToggle", function()
    M.toggle(buf)
  end, { desc = "Toggle inline image preview (snacks.nvim)" })
  vim.api.nvim_buf_create_user_command(buf, "PattoImagesRefresh", function()
    M.refresh(buf)
  end, { desc = "Re-scan the visible lines for images" })
  vim.api.nvim_buf_create_user_command(buf, "PattoImageView", function()
    if not M.view(buf) then
      vim.notify("patto: no image to view on this line", vim.log.levels.INFO)
    end
  end, { desc = "Show the image under the cursor full screen" })
  local key = vim.g.patto_image_keymap
  if key ~= false then
    key = key or "<CR>"
    vim.keymap.set("n", key, function()
      if image_at_cursor(buf, false) then
        return "<Cmd>PattoImageView<CR>"
      end
      return key
    end, { buffer = buf, expr = true, silent = true, desc = "View image under the cursor full screen" })
  end
end

function M.attach(buf)
  buf = normalize(buf)
  ensure_commands(buf)
  if vim.b[buf].patto_images_attached or vim.g.patto_image_enabled == false then
    return
  end
  local image = snacks_image()
  if not image then
    return
  end
  vim.b[buf].patto_images_attached = true
  image.terminal.detect(function()
    if not vim.api.nvim_buf_is_valid(buf) or not vim.b[buf].patto_images_attached or managers[buf] then
      return
    end
    if not image.supports_terminal() then
      return
    end
    if image.config.doc.inline and image.terminal.env().placeholders then
      managers[buf] = Inline.new(buf, options())
    elseif image.config.doc.float then
      attach_hover(buf)
    end
  end)
end

return M
