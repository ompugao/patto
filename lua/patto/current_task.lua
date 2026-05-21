--- patto.current_task
--- Surfaces active ("Doing") and paused ("Paused") patto tasks via a
--- fidget.nvim corner notification, so you don't lose track of what
--- you're supposed to be working on.
---
--- Setup (in your Neovim config):
---
---   require("patto.current_task").setup({
---     fidget        = true,    -- show via fidget.nvim
---     poll_interval = 60000,   -- background poll interval in ms (0 = disable timer)
---   })
---
--- The module also exposes:
---   require("patto.current_task").get()      → { doing = {...}, paused = {...} }
---   require("patto.current_task").refresh()  → manually trigger a fetch
---   require("patto.current_task").debug()    → print current state to messages

local M = {}

-- ── internal state ────────────────────────────────────────────────────────────

---@type table[]  Doing tasks
local _doing  = {}
---@type table[]  Paused tasks
local _paused = {}

---@type uv_timer_t|nil
local _timer = nil
---@type uv_timer_t|nil  Cheap display-only refresh (no LSP call)
local _display_timer = nil

-- ── helpers ───────────────────────────────────────────────────────────────────

local function fmt_time_spent(ts)
  if not ts or type(ts) ~= "table" then return "" end
  local h, m = ts.hours or 0, ts.minutes or 0
  if     h > 0 and m > 0 then return string.format("⏱ %dh%dm", h, m)
  elseif h > 0            then return string.format("⏱ %dh",    h)
  elseif m > 0            then return string.format("⏱ %dm",    m)
  else                         return ""
  end
end

--- Parse a "YYYY-MM-DDTHH:MM" datetime string into an os.time timestamp.
local function parse_datetime(dt)
  if not dt then return nil end
  local y, mo, d, h, mi = dt:match("^(%d+)-(%d+)-(%d+)T(%d+):(%d+)")
  if not y then return nil end
  return os.time({ year = tonumber(y), month = tonumber(mo), day = tonumber(d),
                   hour = tonumber(h), min = tonumber(mi), sec = 0 })
end

--- Format total elapsed time = time_spent + live elapsed since started_at.
--- For Doing tasks (started_at present), adds the current session time.
--- For Paused/Todo tasks (no started_at), shows time_spent as-is.
local function fmt_total_time_spent(task)
  local base_minutes = 0
  local ts = task.time_spent
  if ts and type(ts) == "table" then
    base_minutes = (ts.hours or 0) * 60 + (ts.minutes or 0)
  end

  local live_minutes = 0
  local sa = task.started_at
  if sa and type(sa) == "table" and sa.DateTime then
    local start_ts = parse_datetime(sa.DateTime)
    if start_ts then
      live_minutes = math.max(0, math.floor((os.time() - start_ts) / 60))
    end
  end

  local total = base_minutes + live_minutes
  if total <= 0 then return "" end
  local h = math.floor(total / 60)
  local m = total % 60
  if     h > 0 and m > 0 then return string.format("⏱ %dh%dm", h, m)
  elseif h > 0            then return string.format("⏱ %dh",    h)
  else                         return string.format("⏱ %dm",    m)
  end
end

local function fmt_started_at(sa)
  if not sa or type(sa) ~= "table" then return "" end
  local dt = sa.DateTime
  if not dt then return "" end
  local hm = string.match(dt, "T(%d%d:%d%d)")
  return hm and ("▶ " .. hm) or ""
end

---@param task table
---@return string
local function task_display(task)
  if not task then return "" end
  local parts = { task.text }
  local ts = fmt_total_time_spent(task)
  if ts ~= "" then parts[#parts+1] = ts end
  local sa = fmt_started_at(task.started_at)
  if sa ~= "" then parts[#parts+1] = sa end
  return table.concat(parts, "  ")
end

local function task_key(task, i)
  local loc = task.location and task.location.range and task.location.range.start
  return string.format("patto_task_%s_%d",
    task.location and task.location.uri or tostring(i),
    loc and loc.line or i)
end

-- ── fidget ────────────────────────────────────────────────────────────────────

local _fidget_enabled = false
local _fidget_keys    = {}  -- key → true, for stale-cleanup

local function fidget_refresh()
  if not _fidget_enabled then return end
  local ok, fidget = pcall(require, "fidget")
  if not ok then return end

  -- Build wanted set: key → { task, annote, level }
  local wanted = {}
  for i, task in ipairs(_doing) do
    wanted[task_key(task, i)] = {
      task   = task,
      annote = "◑ doing",
      level  = vim.log.levels.INFO,
    }
  end
  for i, task in ipairs(_paused) do
    wanted[task_key(task, 1000 + i)] = {
      task   = task,
      annote = "⏸ paused",
      level  = vim.log.levels.HINT,
    }
  end

  -- Clear stale keys
  for key in pairs(_fidget_keys) do
    if not wanted[key] then
      fidget.notify(nil, nil, {
        key          = key,
        ttl          = 1,
        update_only  = true,
        skip_history = true,
      })
    end
  end

  -- Upsert active tasks
  _fidget_keys = {}
  for key, entry in pairs(wanted) do
    _fidget_keys[key] = true
    fidget.notify(task_display(entry.task), entry.level, {
      key          = key,
      annote       = entry.annote,
      ttl          = 9e9,
      skip_history = true,
    })
  end
end

-- ── LSP fetch ─────────────────────────────────────────────────────────────────

local function find_patto_bufnr()
  for _, bufnr in ipairs(vim.api.nvim_list_bufs()) do
    if vim.api.nvim_buf_is_loaded(bufnr) and vim.bo[bufnr].filetype == "patto" then
      return bufnr
    end
  end
  return nil
end

local _fetching = false

local function fetch()
  if _fetching then return end
  local bufnr = find_patto_bufnr()
  if not bufnr then return end

  local clients = vim.lsp.get_clients({ bufnr = bufnr, name = "patto_lsp" })
  if #clients == 0 then return end
  local client = clients[1]

  _fetching = true
  local ok_req = pcall(function()
    client:request("workspace/executeCommand", {
      command   = "experimental/aggregate_tasks",
      arguments = {},
    }, function(err, result)
      _fetching = false
      if err or not result or type(result) ~= "table" then return end

      local doing, paused = {}, {}
      for _, task in ipairs(result) do
        if task.status == "Doing" then
          doing[#doing+1] = task
        elseif task.status == "Paused" then
          paused[#paused+1] = task
        end
      end

      _doing  = doing
      _paused = paused
      vim.schedule(fidget_refresh)
    end, bufnr)
  end)

  if not ok_req then _fetching = false end
end

-- ── public API ────────────────────────────────────────────────────────────────

--- Returns { doing = table[], paused = table[] }.
function M.get()
  return { doing = _doing, paused = _paused }
end

--- Manually trigger a fetch.
function M.refresh()
  fetch()
end

--- Print current state to messages (for debugging).
function M.debug()
  local bufnr  = find_patto_bufnr()
  local clients = bufnr and vim.lsp.get_clients({ bufnr = bufnr, name = "patto_lsp" }) or {}
  local fmt = function(list) return table.concat(vim.tbl_map(function(t) return t.text end, list), " | ") end
  vim.notify(string.format(
    "[patto.current_task] bufnr=%s clients=%d fetching=%s\n  doing(%d): %s\n  paused(%d): %s",
    tostring(bufnr), #clients, tostring(_fetching),
    #_doing,  #_doing  > 0 and fmt(_doing)  or "none",
    #_paused, #_paused > 0 and fmt(_paused) or "none"
  ), vim.log.levels.INFO)
end

---@class PattoCurrentTaskOpts
---@field fidget        boolean|nil
---@field poll_interval         integer|nil  ms between LSP polls; 0 = disable timer
---@field display_interval      integer|nil  ms between display-only refreshes (no LSP); 0 = disable; default 60000

---@param opts PattoCurrentTaskOpts|nil
function M.setup(opts)
  opts = opts or {}
  _fidget_enabled = opts.fidget == true
  local interval         = opts.poll_interval
  if interval == nil then interval = 60000 end
  local display_interval = opts.display_interval
  if display_interval == nil then display_interval = 60000 end

  local ag = vim.api.nvim_create_augroup("PattoCurrentTask", { clear = true })

  vim.api.nvim_create_autocmd({ "BufWritePost", "InsertLeave" }, {
    group    = ag,
    pattern  = "*.pn",
    callback = function() fetch() end,
  })

  -- Primary trigger: fetch once the patto LSP has attached
  vim.api.nvim_create_autocmd("LspAttach", {
    group    = ag,
    callback = function(ev)
      if vim.bo[ev.buf].filetype == "patto" then
        vim.defer_fn(fetch, 500)
      end
    end,
  })

  -- Background LSP poll timer
  if interval > 0 then
    if _timer then _timer:stop(); _timer:close() end
    _timer = vim.uv.new_timer()
    _timer:start(interval, interval, vim.schedule_wrap(fetch))
  end

  -- Display-only refresh timer: re-renders fidget with current in-memory task
  -- data so the live elapsed time stays up to date without hitting the LSP.
  if display_interval > 0 then
    if _display_timer then _display_timer:stop(); _display_timer:close() end
    _display_timer = vim.uv.new_timer()
    _display_timer:start(display_interval, display_interval,
      vim.schedule_wrap(function()
        -- Only re-render when there are active Doing tasks (started_at present).
        local has_doing = false
        for _, t in ipairs(_doing) do
          if t.started_at then has_doing = true; break end
        end
        if has_doing then fidget_refresh() end
      end))
  end
end

return M
