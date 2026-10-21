-- Inline image preview for `[@img ...]` lines, rendered through snacks.nvim.
-- snacks' own document mode needs a tree-sitter parser, which patto does not
-- have, so this module locates images itself and drives snacks' placement API.
-- The locator is a loose heuristic (the path token inside the brackets), not a
-- mirror of the patto grammar.
local M = {}

local managers = {}
local hover
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

local function options()
  local doc = Snacks.image.config.doc
  return {
    max_width = vim.g.patto_image_max_width or doc.max_width,
    max_height = vim.g.patto_image_max_height or doc.max_height,
    conceal = vim.g.patto_image_conceal == true,
  }
end

local function resolve(buf, src)
  local resolved = Snacks.image.doc.resolve(buf, src)
  if not Snacks.image.supports_file(resolved) then
    return
  end
  if not is_url(resolved) and not (vim.uv or vim.loop).fs_stat(resolved) then
    return
  end
  return resolved
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

function Inline:visible()
  local ret = {}
  for _, win in ipairs(vim.fn.win_findbuf(self.buf)) do
    local info = vim.fn.getwininfo(win)[1]
    for k, v in pairs(self:get(math.max(info.topline - 1, 1), info.botline)) do
      ret[k] = v
    end
  end
  return ret
end

function Inline:found()
  local found = {}
  for _, win in ipairs(vim.fn.win_findbuf(self.buf)) do
    local info = vim.fn.getwininfo(win)[1]
    local from, to = math.max(info.topline - 1, 1), info.botline
    local lines = vim.api.nvim_buf_get_lines(self.buf, from - 1, to, false)
    for _, m in ipairs(M.find(lines, from)) do
      local key = m.row .. ":" .. m.col
      if not found[key] then
        m.src = resolve(self.buf, m.src)
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

function Inline:place(m)
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
  for _, eid in ipairs(img.eids) do
    self.idx[eid] = img
  end
  self.imgs[img.id] = img
end

function Inline:update()
  if not vim.api.nvim_buf_is_valid(self.buf) then
    return
  end
  local visible = self:visible()
  for _, m in pairs(self:found()) do
    local img = self:take(visible, m)
    if img then
      img.opts.pos = { m.row, m.col }
      img.opts.range = { m.row, m.col, m.row, m.end_col }
      img:update()
    else
      self:place(m)
    end
  end
  for _, img in pairs(visible) do
    img:close()
    self.imgs[img.id] = nil
  end
end

function Inline:close()
  self.closed = true
  pcall(vim.api.nvim_del_augroup_by_name, augroup_name(self.buf))
  for _, img in pairs(self.imgs) do
    img:close()
  end
  self.imgs, self.idx = {}, {}
end

local function hover_close()
  if hover then
    hover.win:close()
    hover.img:close()
    hover = nil
  end
end

local function at_cursor(buf)
  local cursor = vim.api.nvim_win_get_cursor(0)
  local line = vim.api.nvim_buf_get_lines(buf, cursor[1] - 1, cursor[1], false)[1] or ""
  for _, m in ipairs(M.parse_line(line)) do
    if cursor[2] >= m.col and cursor[2] < m.end_col then
      return resolve(buf, m.src)
    end
  end
end

function M.hover(buf)
  if hover and (hover.buf ~= vim.api.nvim_get_current_buf() or vim.fn.mode() ~= "n") then
    return hover_close()
  end
  if hover and not hover.win:valid() then
    hover_close()
  end
  if vim.api.nvim_get_current_buf() ~= buf then
    return
  end
  local src = at_cursor(buf)
  if not src then
    return hover_close()
  end
  if hover and hover.img.img.src == src then
    return
  end
  hover_close()

  local o = options()
  local win = Snacks.win(Snacks.win.resolve(Snacks.image.config.doc, "snacks_image", { show = false, enter = false }))
  win:open_buf()
  local shown = false
  local placement = Snacks.image.placement.new(
    win.buf,
    src,
    Snacks.config.merge({}, Snacks.image.config.doc, {
      inline = false,
      max_width = o.max_width,
      max_height = o.max_height,
      on_update_pre = function(p)
        if shown or not hover or hover.img ~= p then
          return
        end
        shown = true
        local loc = p:state().loc
        win.opts.width, win.opts.height = loc.width, loc.height
        win:show()
      end,
    })
  )
  hover = { buf = buf, win = win, img = placement }
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
