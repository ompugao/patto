---@diagnostic disable: inject-field
local Item = require("trouble.item")

---@type trouble.Source
local M = {}

-- ── helpers ──────────────────────────────────────────────────────────────────

--- Extract a plain "YYYY-MM-DD" string from a Deadline enum value.
local function deadline_date_str(d)
  if not d then return nil end
  if type(d) == "table" then
    if d.Date     then return d.Date end
    if d.DateTime then return string.match(d.DateTime, "^(%d%d%d%d%-%d%d%-%d%d)") end
  end
  return nil
end

--- Parse a "YYYY-MM-DD" string into an os.time timestamp (noon that day).
local function date_ts(s)
  if not s then return nil end
  local y, mo, d = string.match(s, "^(%d%d%d%d)%-(%d%d)%-(%d%d)")
  if not y then return nil end
  return os.time({ year = tonumber(y), month = tonumber(mo), day = tonumber(d),
                   hour = 12, min = 0, sec = 0 })
end

--- Parse a "YYYY-MM-DDTHH:MM" datetime string into an os.time timestamp.
local function parse_datetime(dt)
  if not dt then return nil end
  local y, mo, d, h, mi = dt:match("^(%d+)-(%d+)-(%d+)T(%d+):(%d+)")
  if not y then return nil end
  return os.time({ year = tonumber(y), month = tonumber(mo), day = tonumber(d),
                   hour = tonumber(h), min = tonumber(mi), sec = 0 })
end

--- Compute total elapsed minutes = time_spent + live elapsed since started_at.
local function total_time_spent_minutes(task)
  local base = 0
  local ts = task.time_spent
  if ts and type(ts) == "table" then
    base = (ts.hours or 0) * 60 + (ts.minutes or 0)
  end
  local live = 0
  local sa = task.started_at
  if sa and type(sa) == "table" and sa.DateTime then
    local start_ts = parse_datetime(sa.DateTime)
    if start_ts then
      live = math.max(0, math.floor((os.time() - start_ts) / 60))
    end
  end
  return base + live
end

--- Classify a task's due date into a bucket label and numeric sort key.
local function classify_due(task)
  local due_str = deadline_date_str(task.due)
  if not due_str then
    return "No Deadline", 9999999999
  end

  local due_ts = date_ts(due_str)
  if not due_ts then
    return "Invalid", 9999999998
  end

  local now   = os.time()
  local t     = os.date("*t", now) --[[@as table]]
  local today_start = os.time({ year = t.year, month = t.month, day = t.day,
                                 hour = 0, min = 0, sec = 0 })
  local diff_days   = math.floor((due_ts - today_start) / 86400)

  if diff_days < 0  then return "⚠Overdue",    due_ts end
  if diff_days == 0 then return "Today",      due_ts end
  if diff_days == 1 then return "Tomorrow",   due_ts end

  local days_until_sat = 7 - t.wday   -- wday: 1=Sun … 7=Sat
  local week_end = os.time({ year = t.year, month = t.month,
                              day  = t.day + days_until_sat,
                              hour = 23, min = 59, sec = 59 })
  if due_ts <= week_end then return "  This Week", due_ts end

  local nm_year, nm = t.year, t.month + 1
  if nm > 12 then nm = 1; nm_year = nm_year + 1 end
  local month_end = os.time({ year = nm_year, month = nm, day = 1,
                               hour = 0, min = 0, sec = 0 }) - 1
  if due_ts <= month_end then return "This Month", due_ts end

  return "Later", due_ts
end

-- ── formatters ───────────────────────────────────────────────────────────────

---@diagnostic disable-next-line: missing-fields
M.config = {
  formatters = {
    -- Group header: bucket label
    deadline_group = function(ctx)
      local label, _ = classify_due(ctx.item.item or {})
      return { text = label }
    end,

    -- Due-date chip:  2026-06-01
    task_due = function(ctx)
      local task = ctx.item.item or {}
      local s = deadline_date_str(task.due) or ""
      return { text = s ~= "" and (" " .. s) or "", hl = "Comment" }
    end,

    -- Status icon: ○ todo  ◑ doing  ✓ done
    task_status = function(ctx)
      local status = (ctx.item.item or {}).status
      if     status == "Doing"  then return { text = "◑ ", hl = "DiagnosticWarn" }
      elseif status == "Paused" then return { text = "⏸ ", hl = "DiagnosticInfo" }
      elseif status == "Done"   then return { text = "✓ ", hl = "DiagnosticOk"   }
      else                           return { text = "○ ", hl = "Comment"        }
      end
    end,

    -- time_spent chip:  ⏱ 1h30m  (total = accumulated + live session if Doing)
    task_time_spent = function(ctx)
      local task = ctx.item.item or {}
      local total = total_time_spent_minutes(task)
      if total <= 0 then return { text = "" } end
      local h = math.floor(total / 60)
      local m = total % 60
      local s
      if     h > 0 and m > 0 then s = string.format("⏱ %dh%dm", h, m)
      elseif h > 0            then s = string.format("⏱ %dh",    h)
      else                         s = string.format("⏱ %dm",    m)
      end
      return { text = " " .. s, hl = "DiagnosticInfo" }
    end,

    -- started_at chip:  ▶ HH:MM  (visible only while clocked in)
    task_started_at = function(ctx)
      local sa = (ctx.item.item or {}).started_at
      if not sa or type(sa) ~= "table" then return { text = "" } end
      local dt = sa.DateTime
      if not dt then return { text = "" } end
      local hm = string.match(dt, "T(%d%d:%d%d)")
      if not hm then return { text = "" } end
      return { text = " ▶ " .. hm, hl = "DiagnosticWarn" }
    end,
  },

  sorters = {
    deadline = function(item)
      local _, key = classify_due(item.item or {})
      return key
    end,
  },

  modes = {
    patto_tasks = {
      mode   = "patto_tasks",
      events = { "BufEnter", "BufWritePost", "InsertLeave" },
      source = "patto_tasks",
      desc   = "Tasks grouped by deadline",
      groups = {
        { "deadline_group", format = "{deadline_group}" },
      },
      sort   = { "deadline", "filename", "pos" },
      -- Format: status icon | label | due chip | time chips | file
      format = "{task_status}{task_due} {text}{task_time_spent}{task_started_at} {filename}",
      win = { position = "bottom", size = 0.25 },
    },
  },
}

-- ── source.get ───────────────────────────────────────────────────────────────

--- Timer that periodically refreshes Trouble so live elapsed time stays current.
---@type uv_timer_t|nil
local _refresh_timer = nil
local _refresh_interval = 60000  -- ms; update via M.setup({ refresh_interval = N })

--- Start the display-refresh timer (idempotent).
local function ensure_refresh_timer()
  if _refresh_timer then return end
  _refresh_timer = vim.uv.new_timer()
  _refresh_timer:start(_refresh_interval, _refresh_interval, vim.schedule_wrap(function()
    local ok, trouble = pcall(require, "trouble")
    if not ok then return end
    -- Only refresh if the patto_tasks view is open.
    if trouble.is_open({ mode = "patto_tasks" }) then
      trouble.refresh({ mode = "patto_tasks" })
    else
      -- No open view — stop the timer to avoid unnecessary work.
      if _refresh_timer then
        _refresh_timer:stop()
        _refresh_timer:close()
        _refresh_timer = nil
      end
    end
  end))
end

function M.get(cb)
  ensure_refresh_timer()
  local patto_bufnr = nil
  for _, bufnr in ipairs(vim.api.nvim_list_bufs()) do
    if vim.api.nvim_buf_is_loaded(bufnr) and vim.bo[bufnr].filetype == "patto" then
      patto_bufnr = bufnr
      break
    end
  end

  if not patto_bufnr then cb({}) return end

  vim.lsp.buf_request_all(patto_bufnr, "workspace/executeCommand", {
    command   = "experimental/aggregate_tasks",
    arguments = {},
  }, function(results)
    local items = {} ---@type trouble.Item[]

    if not results then cb(items) return end

    for _, vres in pairs(results) do
      if vres.result == nil then goto continue end

      for _, task in ipairs(vres.result) do
        local row = task.location.range.start.line      + 1
        local col = task.location.range.start.character + 1
        local bucket, _ = classify_due(task)

        items[#items + 1] = Item.new({
          buf          = vim.fn.bufadd(vim.uri_to_fname(task.location.uri)),
          pos          = { row, col },
          end_pos      = { row, col },
          text         = task.text,
          filename     = vim.uri_to_fname(task.location.uri),
          item         = task,
          source       = "patto_tasks",
          deadline_group = bucket,
        })
      end
      ::continue::
    end

    cb(items)
  end)
end

--- Configure the patto_tasks trouble source.
---@param opts { refresh_interval?: integer }|nil
function M.setup(opts)
  opts = opts or {}
  if opts.refresh_interval ~= nil then
    _refresh_interval = opts.refresh_interval
  end
end

return M
