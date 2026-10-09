---@diagnostic disable: inject-field
local fmt = require("patto.task_format")
local tasks_source = require("patto.tasks_source")

local refresh_timer = nil ---@type uv_timer_t|nil
local refresh_interval = 60000

local function stop_refresh_timer()
  if not refresh_timer then return end
  refresh_timer:stop()
  refresh_timer:close()
  refresh_timer = nil
end

local function refresh_open_view()
  local ok, trouble = pcall(require, "trouble")
  if not ok then return end
  if trouble.is_open({ mode = "patto_tasks" }) then
    trouble.refresh({ mode = "patto_tasks" })
  else
    stop_refresh_timer()
  end
end

-- Live elapsed time is rendered from started_at, so the view has to be redrawn
-- periodically even when no buffer changed.
local function ensure_refresh_timer()
  if refresh_timer then return end
  refresh_timer = vim.uv.new_timer()
  refresh_timer:start(refresh_interval, refresh_interval, vim.schedule_wrap(refresh_open_view))
end

local M = tasks_source.new({
  mode = "patto_tasks",
  command = "experimental/aggregate_tasks",
  arguments = function() return {} end,
  fields = function(task)
    return { deadline_group = (fmt.classify_due(task)) }
  end,
  before_get = ensure_refresh_timer,
})

---@diagnostic disable-next-line: missing-fields
M.config = {
  formatters = {
    deadline_group = function(ctx)
      return { text = (fmt.classify_due(ctx.item.item or {})) }
    end,
    task_due = function(ctx)
      local s = fmt.deadline_date((ctx.item.item or {}).due) or ""
      return { text = s ~= "" and (" " .. s) or "", hl = "Comment" }
    end,
    task_status = function(ctx)
      local status = (ctx.item.item or {}).status
      if status == "Doing" then return { text = "◑ ", hl = "DiagnosticWarn" } end
      if status == "Paused" then return { text = "⏸ ", hl = "DiagnosticInfo" } end
      if status == "Done" then return { text = "✓ ", hl = "DiagnosticOk" } end
      return { text = "○ ", hl = "Comment" }
    end,
    task_time_spent = function(ctx)
      local s = fmt.total_time_spent_text(ctx.item.item or {})
      if not s then return { text = "" } end
      return { text = " ⏱ " .. s, hl = "DiagnosticInfo" }
    end,
    task_started_at = function(ctx)
      local hm = fmt.started_at_clock((ctx.item.item or {}).started_at)
      if not hm then return { text = "" } end
      return { text = " ▶ " .. hm, hl = "DiagnosticWarn" }
    end,
  },

  sorters = {
    deadline = function(item)
      local _, key = fmt.classify_due(item.item or {})
      return key
    end,
  },

  modes = {
    patto_tasks = {
      mode = "patto_tasks",
      events = { "BufEnter", "BufWritePost", "InsertLeave" },
      source = "patto_tasks",
      desc = "Tasks grouped by deadline",
      groups = {
        { "deadline_group", format = "{deadline_group}" },
      },
      sort = { "deadline", "filename", "pos" },
      format = "{task_status}{task_due} {text}{task_time_spent}{task_started_at} {filename}",
      win = { position = "bottom", size = 0.25 },
      keys = tasks_source.status_keys({ increment = "<c-a>", decrement = "<c-x>", undo = "u" }),
    },
  },
}

---@param opts { refresh_interval?: integer }|nil
function M.setup(opts)
  opts = opts or {}
  if opts.refresh_interval ~= nil then
    refresh_interval = opts.refresh_interval
  end
end

return M
