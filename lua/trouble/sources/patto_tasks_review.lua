---@diagnostic disable: inject-field
local fmt = require("patto.task_format")
local review_timeframe = require("patto.review_timeframe")
local tasks_source = require("patto.tasks_source")

local show_archived = false

local function resolved_at(task)
  return task.completed_at or task.archived_at
end

local M = tasks_source.new({
  mode = "patto_tasks_review",
  command = "experimental/tasks_review",
  arguments = function()
    local from, to = review_timeframe.recent_range()
    return { "custom", from, to, show_archived }
  end,
  fields = function(task)
    local bucket, order = fmt.classify_completed(resolved_at(task))
    return { completed_date_group = bucket, completed_date_group_order = order }
  end,
})

local function toggle_archived(view)
  show_archived = not show_archived
  vim.notify("patto: archived tasks " .. (show_archived and "shown" or "hidden"), vim.log.levels.INFO)
  view:refresh()
end

local keys = tasks_source.status_keys({ increment = "t", decrement = "T", undo = "u" })
keys.a = { action = toggle_archived, desc = "Toggle archived tasks" }

---@diagnostic disable-next-line: missing-fields
M.config = {
  formatters = {
    completed_date_group = function(ctx)
      return { text = ctx.item.completed_date_group or "Older" }
    end,
    task_completed_at = function(ctx)
      local task = ctx.item.item or {}
      if task.archived_at then return { text = "⊘ " .. task.archived_at, hl = "Comment" } end
      local s = task.completed_at or ""
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
      return fmt.date_to_time(resolved_at(item.item or {})) or 0
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
      desc = "Completed (and optionally archived) tasks grouped by recency",
      groups = {
        { "completed_date_group", format = "{completed_date_group}" },
      },
      sort = { "completed_date_group_order", "completed_at", "filename", "pos" },
      format = "{task_completed_at} {text}{task_time_spent} {filename}",
      win = { position = "bottom", size = 0.20 },
      keys = keys,
    },
  },
}

return M
