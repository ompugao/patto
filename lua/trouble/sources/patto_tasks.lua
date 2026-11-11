---@diagnostic disable: inject-field
local fmt = require("patto.task_format")
local tasks_source = require("patto.tasks_source")

local MODE = "patto_tasks"
local DONE_GROUP_ID = "$root#done"

local refresh_timer = nil ---@type uv_timer_t|nil
local refresh_interval = 60000
local show_resolved = true
local show_archived = false

local DONE_RANGES = { "week", "30d", "all" }
local DONE_LABELS = { week = "Done this week", ["30d"] = "Done in last 30 days", all = "Done (all)" }
local done_range = "week"

local function stop_refresh_timer()
  if not refresh_timer then return end
  refresh_timer:stop()
  refresh_timer:close()
  refresh_timer = nil
end

local function refresh_open_view()
  local ok, trouble = pcall(require, "trouble")
  if not ok then return end
  if trouble.is_open({ mode = MODE }) then
    trouble.refresh({ mode = MODE })
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

local function set_done_folded(view, folded)
  -- Trouble has no option to start a group folded and no hook after the first
  -- render, so the renderer's fold table is set directly. If that private field
  -- changes, the group is simply shown open.
  pcall(function() view.renderer._folded[DONE_GROUP_ID] = folded or nil end)
end

local initialized_views = setmetatable({}, { __mode = "k" })

local function fold_done_in_new_views()
  local ok, view_mod = pcall(require, "trouble.view")
  if not ok then return end
  for _, v in ipairs(view_mod.get({ mode = MODE })) do
    if v.view and not initialized_views[v.view] then
      initialized_views[v.view] = true
      set_done_folded(v.view, true)
    end
  end
end

local function resolved_since()
  if done_range == "all" then return vim.empty_dict() end
  if done_range == "30d" then return { from = fmt.days_before(29) } end
  return { timeframe = "this_week" }
end

local function request_arguments()
  local params = { open = true }
  if show_resolved then
    params.resolved = resolved_since()
    params.resolved.include_archived = show_archived
  end
  return { params }
end

local M = tasks_source.new({
  mode = MODE,
  command = "patto/tasks",
  arguments = request_arguments,
  fields = function(task)
    local group_key, group_order, rank = fmt.task_placement(task)
    return {
      group_key = group_key,
      group_order = group_order,
      rank = rank,
      resolved = fmt.is_resolved(task),
    }
  end,
  before_get = function()
    ensure_refresh_timer()
    fold_done_in_new_views()
  end,
})

local function define_highlights()
  local comment = vim.api.nvim_get_hl(0, { name = "Comment", link = false })
  vim.api.nvim_set_hl(0, "PattoTaskResolved", {
    fg = comment.fg, ctermfg = comment.ctermfg, strikethrough = true, default = true,
  })
end

define_highlights()
vim.api.nvim_create_autocmd("ColorScheme", {
  group = vim.api.nvim_create_augroup("patto_tasks_highlights", { clear = true }),
  callback = define_highlights,
})

local function resolved_items(node)
  local items = {}
  for _, child in ipairs(node and node.children or {}) do
    if child.item and child.item.resolved then items[#items + 1] = child.item end
  end
  return items
end

local function total_time_text(items)
  local minutes = 0
  for _, item in ipairs(items) do
    minutes = minutes + fmt.time_spent_minutes((item.item or {}).time_spent)
  end
  return minutes > 0 and fmt.minutes_text(minutes) or nil
end

local function done_header(node)
  local parent = node.parent
  local items = resolved_items(node)
  vim.list_extend(items, resolved_items(parent and parent:get(parent.id .. "#today")))
  local text = ("%s  ✓ %d"):format(DONE_LABELS[done_range], #items)
  local time = total_time_text(items)
  if time then text = text .. " · ⏱ " .. time end
  return text
end

local function open_group_header(key, node)
  local label = fmt.GROUP_LABELS[key] or key
  local done = #resolved_items(node)
  local total = node:count()
  if done == 0 then return ("%s  %d"):format(label, total) end
  return ("%s  %d left · ✓ %d done"):format(label, total - done, done)
end

local function toggle_archived(view)
  show_archived = not show_archived
  vim.notify("patto: archived tasks " .. (show_archived and "shown" or "hidden"), vim.log.levels.INFO)
  view:refresh()
end

local function widen_done_range(view)
  local next_index = 1
  for i, name in ipairs(DONE_RANGES) do
    if name == done_range then next_index = i % #DONE_RANGES + 1 end
  end
  done_range = DONE_RANGES[next_index]
  set_done_folded(view, false)
  vim.notify("patto: " .. DONE_LABELS[done_range], vim.log.levels.INFO)
  view:refresh()
end

local keys = tasks_source.status_keys({ increment = "<c-a>", decrement = "<c-x>", undo = "u" })
keys.a = { action = toggle_archived, desc = "Toggle archived tasks" }
keys.r = { action = widen_done_range, desc = "Change the range of done tasks" }

---@diagnostic disable-next-line: missing-fields
M.config = {
  formatters = {
    task_group = function(ctx)
      local key = ctx.item.group_key
      if key == "done" then return { text = done_header(ctx.node) } end
      return { text = open_group_header(key, ctx.node) }
    end,
    text = function(ctx)
      if ctx.item.resolved then return { text = ctx.item.text, hl = "PattoTaskResolved" } end
      return { text = ctx.item.text }
    end,
    task_due = function(ctx)
      local task = ctx.item.item or {}
      local s
      if ctx.item.resolved then
        s = ctx.item.group_key == "done" and fmt.resolved_at(task) or nil
      else
        s = fmt.deadline_date(task.due)
      end
      return { text = s and (" " .. s) or "", hl = "Comment" }
    end,
    task_status = function(ctx)
      local status = (ctx.item.item or {}).status
      if status == "Doing" then return { text = "◑ ", hl = "DiagnosticWarn" } end
      if status == "Paused" then return { text = "⏸ ", hl = "DiagnosticInfo" } end
      if status == "Done" then return { text = "✓ ", hl = "DiagnosticOk" } end
      if status == "Archived" then return { text = "⊘ ", hl = "Comment" } end
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
    group_order = function(item) return item.group_order end,
    rank = function(item) return item.rank end,
  },

  modes = {
    [MODE] = {
      mode = MODE,
      events = { "BufEnter", "BufWritePost", "InsertLeave" },
      source = MODE,
      desc = "Tasks to do, with recently resolved ones",
      groups = {
        { "group_key", format = "{task_group}" },
      },
      sort = { "group_order", "rank", "filename", "pos" },
      format = "{task_status}{task_due} {text}{task_time_spent}{task_started_at} {filename}",
      win = { position = "bottom", size = 0.25 },
      keys = keys,
    },
  },
}

---@param opts { refresh_interval?: integer, show_resolved?: boolean }|nil
function M.setup(opts)
  opts = opts or {}
  if opts.refresh_interval ~= nil then
    refresh_interval = opts.refresh_interval
  end
  if opts.show_resolved ~= nil then
    show_resolved = opts.show_resolved
  end
end

return M
