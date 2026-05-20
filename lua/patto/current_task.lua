--- patto.current_task
--- Surfaces the currently active ("Doing") patto task in the winbar and/or
--- via a fidget.nvim corner notification, so you don't lose track of what
--- you're supposed to be working on.
---
--- Setup (in your Neovim config):
---
---   require("patto.current_task").setup({
---     winbar        = true,    -- show in winbar
---     fidget        = true,    -- show via fidget.nvim
---     poll_interval = 60000,   -- background poll interval in ms (0 = disable timer)
---   })
---
--- The module also exposes:
---   require("patto.current_task").get()      → task table or nil
---   require("patto.current_task").text()     → formatted string or ""
---   require("patto.current_task").refresh()  → manually trigger a fetch
---   require("patto.current_task").debug()    → print current state to messages

local M = {}

-- ── internal state ────────────────────────────────────────────────────────────

---@type table[]  all Doing tasks from LSP
local _current = {}

---@type uv_timer_t|nil
local _timer = nil

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

local function fmt_started_at(sa)
  if not sa or type(sa) ~= "table" then return "" end
  local dt = sa.DateTime
  if not dt then return "" end
  local hm = string.match(dt, "T(%d%d:%d%d)")
  return hm and ("▶ " .. hm) or ""
end

--- Build the display string for a task.
---@param task table
---@return string
local function task_display(task)
  if not task then return "" end
  local parts = { "▶", task.text }
  local ts = fmt_time_spent(task.time_spent)
  if ts ~= "" then parts[#parts+1] = ts end
  local sa = fmt_started_at(task.started_at)
  if sa ~= "" then parts[#parts+1] = sa end
  return table.concat(parts, "  ")
end

--- Build display string for all current tasks.
---@return string
local function all_tasks_display()
  if #_current == 0 then return "" end
  local entries = {}
  for _, task in ipairs(_current) do
    entries[#entries+1] = task_display(task)
  end
  return table.concat(entries, "  ┊  ")
end

-- ── winbar ────────────────────────────────────────────────────────────────────

local _winbar_enabled = false

local function winbar_refresh()
  if not _winbar_enabled then return end
  local text = all_tasks_display()
  if text ~= "" then
    local escaped = text:gsub("%%", "%%%%")
    vim.opt.winbar = "%#DiagnosticWarn# " .. escaped .. " %*"
  else
    vim.opt.winbar = "%#Comment# · no active task%*"
  end
end

-- ── fidget ────────────────────────────────────────────────────────────────────

local _fidget_enabled = false
local _FIDGET_GROUP   = "patto_current_task"
-- Track which keys are currently shown so we can clear stale ones
local _fidget_keys = {}

local function fidget_refresh()
  if not _fidget_enabled then return end
  local ok, fidget = pcall(require, "fidget")
  if not ok then return end

  -- Build set of keys we want active now
  local wanted = {}
  for i, task in ipairs(_current) do
    -- Use a stable per-task key based on its file+line position
    local loc = task.location and task.location.range and task.location.range.start
    local key = string.format("patto_task_%s_%d",
      task.location and task.location.uri or tostring(i),
      loc and loc.line or i)
    wanted[key] = task
  end

  -- Remove keys no longer active
  for key in pairs(_fidget_keys) do
    if not wanted[key] then
      fidget.notify(nil, nil, {
        key          = key,
        group        = _FIDGET_GROUP,
        ttl          = 1,
        update_only  = true,
        skip_history = true,
      })
    end
  end

  -- Upsert active tasks
  _fidget_keys = {}
  for key, task in pairs(wanted) do
    _fidget_keys[key] = true
    fidget.notify(task_display(task), vim.log.levels.INFO, {
      key          = key,
      group        = _FIDGET_GROUP,
      annote       = "doing",
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
    client.request("workspace/executeCommand", {
      command   = "experimental/aggregate_tasks",
      arguments = {},
    }, function(err, result)
      _fetching = false
      if err or not result or type(result) ~= "table" then return end

      local doing = {}
      for _, task in ipairs(result) do
        if task.status == "Doing" then
          doing[#doing+1] = task
        end
      end

      _current = doing
      vim.schedule(function()
        winbar_refresh()
        fidget_refresh()
      end)
    end, bufnr)
  end)

  if not ok_req then _fetching = false end
end

-- ── public API ────────────────────────────────────────────────────────────────

--- Returns all current Doing task objects as a list.
function M.get()
  return _current
end

--- Returns the formatted display string for all active tasks, or "" if none.
function M.text()
  return all_tasks_display()
end

--- Manually trigger a fetch.
function M.refresh()
  fetch()
end

--- Print current state to messages (for debugging).
function M.debug()
  local bufnr = find_patto_bufnr()
  local clients = bufnr and vim.lsp.get_clients({ bufnr = bufnr, name = "patto_lsp" }) or {}
  vim.notify(string.format(
    "[patto.current_task] patto_bufnr=%s  patto_lsp_clients=%d  fetching=%s  doing=%d task(s): %s",
    tostring(bufnr), #clients, tostring(_fetching),
    #_current,
    #_current > 0 and table.concat(vim.tbl_map(function(t) return t.text end, _current), " | ") or "none"
  ), vim.log.levels.INFO)
end

---@class PattoCurrentTaskOpts
---@field winbar       boolean|nil
---@field fidget       boolean|nil
---@field poll_interval integer|nil  ms between background polls; 0 = disable timer

---@param opts PattoCurrentTaskOpts|nil
function M.setup(opts)
  opts = opts or {}
  _winbar_enabled = opts.winbar == true
  _fidget_enabled = opts.fidget == true
  local interval  = opts.poll_interval
  if interval == nil then interval = 60000 end

  local ag = vim.api.nvim_create_augroup("PattoCurrentTask", { clear = true })

  -- Re-fetch on save / leaving insert in patto files
  vim.api.nvim_create_autocmd({ "BufWritePost", "InsertLeave" }, {
    group   = ag,
    pattern = "*.pn",
    callback = function() fetch() end,
  })

  -- Re-apply winbar when switching buffers (even non-patto ones)
  vim.api.nvim_create_autocmd("BufEnter", {
    group    = ag,
    callback = function() winbar_refresh() end,
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

  -- Background timer
  if interval > 0 then
    if _timer then _timer:stop(); _timer:close() end
    _timer = vim.uv.new_timer()
    _timer:start(interval, interval, vim.schedule_wrap(fetch))
  end
end

return M
