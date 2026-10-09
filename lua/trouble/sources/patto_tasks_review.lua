---@diagnostic disable: inject-field
local fmt = require("patto.task_format")
local review_timeframe = require("patto.review_timeframe")
local tasks_source = require("patto.tasks_source")

local M = tasks_source.new({
  mode = "patto_tasks_review",
  command = "experimental/tasks_review",
  arguments = function()
    local from, to = review_timeframe.recent_range()
    return { "custom", from, to }
  end,
  fields = function(task)
    local bucket, order = fmt.classify_completed(task.completed_at)
    return { completed_date_group = bucket, completed_date_group_order = order }
  end,
})

---@diagnostic disable-next-line: missing-fields
M.config = {
  formatters = {
    completed_date_group = function(ctx)
      return { text = ctx.item.completed_date_group or "  Older" }
    end,
    task_completed_at = function(ctx)
      local s = (ctx.item.item or {}).completed_at or ""
      return { text = s ~= "" and ("✓ " .. s) or "", hl = "DiagnosticOk" }
    end,
    task_time_spent = function(ctx)
      local s = fmt.time_spent_text((ctx.item.item or {}).time_spent)
      if not s then return { text = "" } end
      return { text = " ⏱ " .. s, hl = "DiagnosticInfo" }
    end,
  },

  sorters = {
    completed_at = function(item)
      return fmt.date_to_time((item.item or {}).completed_at) or 0
    end,
    completed_date_group_order = function(item)
      return item.completed_date_group_order or 1
    end,
  },

  modes = {
    patto_tasks_review = {
      mode = "patto_tasks_review",
      events = { "BufWritePost" },
      source = "patto_tasks_review",
      desc = "Completed tasks grouped by recency",
      groups = {
        { "completed_date_group", format = "{completed_date_group}" },
      },
      sort = { "completed_date_group_order", "completed_at", "filename", "pos" },
      format = "{task_completed_at} {text}{task_time_spent} {filename}",
      win = { position = "bottom", size = 0.20 },
      keys = tasks_source.status_keys({ increment = "t", decrement = "T", undo = "u" }),
    },
  },
}

return M
