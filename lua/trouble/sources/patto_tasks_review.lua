---@diagnostic disable: inject-field
local Item = require("trouble.item")

---@type trouble.Source
local M = {}

--- Classify a "YYYY-MM-DD" completed_at string into a display bucket.
--- Buckets (most recent first): Today, Yesterday, This Week, Last Week, This Month, Older
--- Returns: bucket label (string), sort key (number, lower = more recent)
local function classify_date(date_str)
  if not date_str or date_str == "" then
    return "Older", 1
  end
  local y, mo, d = string.match(date_str, "^(%d%d%d%d)%-(%d%d)%-(%d%d)$")
  if not y then
    return "Older", 1
  end

  local completed_ts = os.time({ year = tonumber(y), month = tonumber(mo), day = tonumber(d), hour = 12, min = 0, sec = 0 })

  local now = os.time()
  local t = os.date("*t", now) --[[@as table]]
  local today_start  = os.time({ year = t.year, month = t.month, day = t.day,     hour = 0, min = 0, sec = 0 })
  local today_end    = today_start + 86400 - 1
  local yest_start   = today_start - 86400
  local yest_end     = today_start - 1

  -- Monday of this week
  local days_since_mon = (t.wday - 2) % 7  -- wday: 1=Sun, 2=Mon
  local this_week_start = today_start - days_since_mon * 86400
  local last_week_start = this_week_start - 7 * 86400
  local last_week_end   = this_week_start - 1

  -- First day of this month
  local month_start = os.time({ year = t.year, month = t.month, day = 1, hour = 0, min = 0, sec = 0 })

  if completed_ts >= today_start and completed_ts <= today_end then
    return "📅 Today", 6
  elseif completed_ts >= yest_start and completed_ts <= yest_end then
    return "📅 Yesterday", 5
  elseif completed_ts >= this_week_start then
    return "📅 This Week", 4
  elseif completed_ts >= last_week_start and completed_ts <= last_week_end then
    return "📅 Last Week", 3
  elseif completed_ts >= month_start then
    return "📅 This Month", 2
  else
    return "📅 Older", 1
  end
end

---@diagnostic disable-next-line: missing-fields
M.config = {
  formatters = {
    completed_at = function(ctx)
      return {
        text = (ctx.item.item or {}).completed_at or "",
        hl = "Comment",
      }
    end,
    completed_date_group = function(ctx)
      return {
        text = ctx.item.completed_date_group or "Older",
      }
    end,
  },
  sorters = {
    completed_at = function(item)
      local date_str = (item.item or {}).completed_at
      if not date_str or date_str == "" then return 0 end
      local y, mo, d = string.match(date_str, "^(%d%d%d%d)%-(%d%d)%-(%d%d)$")
      if not y then return 0 end
      return os.time({ year = tonumber(y), month = tonumber(mo), day = tonumber(d), hour = 12, min = 0, sec = 0 })
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
      desc = "Completed tasks grouped by recency (today/yesterday/this week/last week/this month)",
      groups = {
        { "completed_date_group", format = "{completed_date_group}" },
      },
      sort = { "completed_date_group_order", "completed_at", "filename", "pos" },
      format = "{completed_at} {text} {filename}",
      win = {
        position = "bottom",
        size = 0.20,
      },
    },
  },
}

--- Fetch completed tasks from Patto LSP server
--- Computes "recent" range client-side (start of last week or month, whichever earlier, through today)
--- and sends as custom date range to the backend.
--- @param cb function Callback to receive items
--- @param ctx table Trouble source context
function M.get(cb, ctx)
  local patto_bufnr = nil
  for _, bufnr in ipairs(vim.api.nvim_list_bufs()) do
    if vim.api.nvim_buf_is_loaded(bufnr) and vim.bo[bufnr].filetype == "patto" then
      patto_bufnr = bufnr
      break
    end
  end

  if not patto_bufnr then
    cb({})
    return
  end

  -- Compute "recent" range: from (start of last week OR start of this month, whichever earlier) through today
  local now = os.time()
  local t = os.date("*t", now) --[[@as table]]
  local today_str = os.date("%Y-%m-%d", now)
  local days_since_mon = (t.wday - 2) % 7  -- wday: 1=Sun, 2=Mon
  local last_week_start_ts = now - (days_since_mon + 7) * 86400
  local month_start_ts = os.time({ year = t.year, month = t.month, day = 1, hour = 0, min = 0, sec = 0 })
  local from_ts = math.min(last_week_start_ts, month_start_ts)
  local from_str = os.date("%Y-%m-%d", from_ts)

  vim.lsp.buf_request_all(patto_bufnr, "workspace/executeCommand", {
    command = "experimental/tasks_review",
    arguments = { "custom", from_str, today_str },
  }, function(results, _ctx, _config)
    local items = {} ---@type trouble.Item[]

    if not results then
      cb(items)
      return
    end

    for _, vres in pairs(results) do
      if vres.result == nil then
        goto continue
      end

      for _, task in ipairs(vres.result) do
        local row = task.location.range.start.line + 1
        local col = task.location.range.start.character + 1
        local bucket, order = classify_date(task.completed_at)

        items[#items + 1] = Item.new({
          buf                       = vim.fn.bufadd(vim.uri_to_fname(task.location.uri)),
          pos                       = { row, col },
          end_pos                   = { row, col },
          text                      = task.text,
          filename                  = vim.uri_to_fname(task.location.uri),
          item                      = task,
          source                    = "patto_tasks_review",
          completed_date_group      = bucket,
          completed_date_group_order = order,
        })
      end
      ::continue::
    end

    cb(items)
  end)
end

return M
