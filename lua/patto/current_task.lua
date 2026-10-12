--- Shows "Doing" and "Paused" patto tasks as fidget.nvim notifications.
---
---   require("patto.current_task").setup({
---     fidget           = true,   -- show via fidget.nvim
---     poll_interval    = 60000,  -- ms between LSP polls; 0 disables the timer
---     display_interval = 60000,  -- ms between display-only refreshes; 0 disables
---   })

local fmt = require("patto.task_format")
local lsp = require("patto.lsp")

local M = {}

local doing = {} ---@type table[]
local paused = {} ---@type table[]
local poll_timer = nil ---@type uv_timer_t|nil
local display_timer = nil ---@type uv_timer_t|nil
local fetching = false
local fidget_enabled = false
local fidget_keys = {}

local function task_display(task)
  if not task then return "" end
  local parts = { task.text }
  local spent = fmt.total_time_spent_text(task)
  if spent then parts[#parts + 1] = "⏱ " .. spent end
  local clock = fmt.started_at_clock(task.started_at)
  if clock then parts[#parts + 1] = "▶ " .. clock end
  return table.concat(parts, "  ")
end

local function task_key(task, i)
  local start = task.location and task.location.range and task.location.range.start
  return string.format("patto_task_%s_%d",
    task.location and task.location.uri or tostring(i),
    start and start.line or i)
end

local function wanted_notifications()
  local wanted = {}
  for i, task in ipairs(doing) do
    wanted[task_key(task, i)] = { task = task, annote = "◑ doing", level = vim.log.levels.INFO }
  end
  for i, task in ipairs(paused) do
    wanted[task_key(task, 1000 + i)] = { task = task, annote = "⏸ paused", level = vim.log.levels.HINT }
  end
  return wanted
end

local function expire_stale_notifications(fidget, wanted)
  for key in pairs(fidget_keys) do
    if not wanted[key] then
      fidget.notify(nil, nil, { key = key, ttl = 1, update_only = true, skip_history = true })
    end
  end
end

local function show_notifications(fidget, wanted)
  fidget_keys = {}
  for key, entry in pairs(wanted) do
    fidget_keys[key] = true
    fidget.notify(task_display(entry.task), entry.level, {
      key = key,
      annote = entry.annote,
      ttl = 9e9,
      skip_history = true,
    })
  end
end

local function fidget_refresh()
  if not fidget_enabled then return end
  local ok, fidget = pcall(require, "fidget")
  if not ok then return end
  local wanted = wanted_notifications()
  expire_stale_notifications(fidget, wanted)
  show_notifications(fidget, wanted)
end

local function split_by_status(tasks)
  local doing_tasks, paused_tasks = {}, {}
  for _, task in ipairs(tasks) do
    if task.status == "Doing" then
      doing_tasks[#doing_tasks + 1] = task
    elseif task.status == "Paused" then
      paused_tasks[#paused_tasks + 1] = task
    end
  end
  return doing_tasks, paused_tasks
end

local function patto_client(bufnr)
  return vim.lsp.get_clients({ bufnr = bufnr, name = "patto_lsp" })[1]
end

local function fetch()
  if fetching then return end
  local bufnr = lsp.find_patto_bufnr()
  if not bufnr then return end
  local client = patto_client(bufnr)
  if not client then return end

  fetching = true
  local ok = pcall(client.request, client, "workspace/executeCommand", {
    command = "experimental/aggregate_tasks",
    arguments = {},
  }, function(err, result)
    fetching = false
    if err or type(result) ~= "table" then return end
    doing, paused = split_by_status(result)
    vim.schedule(fidget_refresh)
  end, bufnr)
  if not ok then fetching = false end
end

local function has_running_task()
  for _, task in ipairs(doing) do
    if task.started_at then return true end
  end
  return false
end

local function refresh_elapsed_time()
  if has_running_task() then fidget_refresh() end
end

local function install_autocmds()
  local group = vim.api.nvim_create_augroup("PattoCurrentTask", { clear = true })
  vim.api.nvim_create_autocmd({ "BufWritePost", "InsertLeave" }, {
    group = group,
    pattern = "*.pn",
    callback = function() fetch() end,
  })
  vim.api.nvim_create_autocmd("LspAttach", {
    group = group,
    callback = function(ev)
      if vim.bo[ev.buf].filetype == "patto" then
        vim.defer_fn(fetch, 500)
      end
    end,
  })
end

local function restart_timer(timer, interval, callback)
  if interval <= 0 then return timer end
  if timer then timer:stop(); timer:close() end
  timer = vim.uv.new_timer()
  timer:start(interval, interval, vim.schedule_wrap(callback))
  return timer
end

--- Returns { doing = table[], paused = table[] }.
function M.get()
  return { doing = doing, paused = paused }
end

function M.refresh()
  fetch()
end

function M.debug()
  local bufnr = lsp.find_patto_bufnr()
  local clients = bufnr and vim.lsp.get_clients({ bufnr = bufnr, name = "patto_lsp" }) or {}
  local function texts(list)
    return table.concat(vim.tbl_map(function(t) return t.text end, list), " | ")
  end
  vim.notify(string.format(
    "[patto.current_task] bufnr=%s clients=%d fetching=%s\n  doing(%d): %s\n  paused(%d): %s",
    tostring(bufnr), #clients, tostring(fetching),
    #doing, #doing > 0 and texts(doing) or "none",
    #paused, #paused > 0 and texts(paused) or "none"
  ), vim.log.levels.INFO)
end

---@class PattoCurrentTaskOpts
---@field fidget boolean|nil
---@field poll_interval integer|nil ms between LSP polls; 0 = disable timer; default 60000
---@field display_interval integer|nil ms between display-only refreshes (no LSP); 0 = disable; default 60000

---@param opts PattoCurrentTaskOpts|nil
function M.setup(opts)
  opts = opts or {}
  fidget_enabled = opts.fidget == true
  install_autocmds()
  poll_timer = restart_timer(poll_timer, opts.poll_interval or 60000, fetch)
  -- Elapsed time of a running task changes every minute, but the task data does
  -- not, so this timer redraws from memory instead of polling the LSP again.
  display_timer = restart_timer(display_timer, opts.display_interval or 60000, refresh_elapsed_time)
end

return M
