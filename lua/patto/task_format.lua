local M = {}

local DAY = 86400

function M.deadline_date(due)
  if type(due) ~= "table" then return nil end
  if due.Date then return due.Date end
  if due.DateTime then return string.match(due.DateTime, "^(%d%d%d%d%-%d%d%-%d%d)") end
  return nil
end

function M.date_to_time(s)
  if not s then return nil end
  local y, mo, d = string.match(s, "^(%d%d%d%d)%-(%d%d)%-(%d%d)")
  if not y then return nil end
  return os.time({ year = tonumber(y), month = tonumber(mo), day = tonumber(d),
                   hour = 12, min = 0, sec = 0 })
end

function M.datetime_to_time(dt)
  if not dt then return nil end
  local y, mo, d, h, mi = dt:match("^(%d+)-(%d+)-(%d+)T(%d+):(%d+)")
  if not y then return nil end
  return os.time({ year = tonumber(y), month = tonumber(mo), day = tonumber(d),
                   hour = tonumber(h), min = tonumber(mi), sec = 0 })
end

function M.started_at_clock(started_at)
  if type(started_at) ~= "table" or not started_at.DateTime then return nil end
  return string.match(started_at.DateTime, "T(%d%d:%d%d)")
end

function M.hours_minutes_text(h, m)
  if h > 0 and m > 0 then return string.format("%dh%dm", h, m) end
  if h > 0 then return string.format("%dh", h) end
  return string.format("%dm", m)
end

function M.minutes_text(total)
  return M.hours_minutes_text(math.floor(total / 60), total % 60)
end

function M.time_spent_text(ts)
  if type(ts) ~= "table" then return nil end
  return M.hours_minutes_text(ts.hours or 0, ts.minutes or 0)
end

function M.time_spent_minutes(ts)
  if type(ts) ~= "table" then return 0 end
  return (ts.hours or 0) * 60 + (ts.minutes or 0)
end

function M.live_minutes(started_at, now)
  if type(started_at) ~= "table" or not started_at.DateTime then return 0 end
  local start = M.datetime_to_time(started_at.DateTime)
  if not start then return 0 end
  return math.max(0, math.floor(((now or os.time()) - start) / 60))
end

function M.total_minutes(task, now)
  return M.time_spent_minutes(task.time_spent) + M.live_minutes(task.started_at, now)
end

function M.total_time_spent_text(task, now)
  local total = M.total_minutes(task, now)
  if total <= 0 then return nil end
  return M.minutes_text(total)
end

local function local_date(now)
  return os.date("*t", now) --[[@as table]]
end

local function day_start(t)
  return os.time({ year = t.year, month = t.month, day = t.day, hour = 0, min = 0, sec = 0 })
end

local function month_start(t)
  return os.time({ year = t.year, month = t.month, day = 1, hour = 0, min = 0, sec = 0 })
end

local function month_end(t)
  local year, month = t.year, t.month + 1
  if month > 12 then month = 1; year = year + 1 end
  return os.time({ year = year, month = month, day = 1, hour = 0, min = 0, sec = 0 }) - 1
end

local function saturday_end(t)
  return os.time({ year = t.year, month = t.month, day = t.day + (7 - t.wday),
                   hour = 23, min = 59, sec = 59 })
end

function M.classify_due(task, now)
  local due_str = M.deadline_date(task.due)
  if not due_str then return "No Deadline", 9999999999 end
  local due_ts = M.date_to_time(due_str)
  if not due_ts then return "Invalid", 9999999998 end

  local t = local_date(now or os.time())
  local diff_days = math.floor((due_ts - day_start(t)) / DAY)
  if diff_days < 0 then return "⚠Overdue", due_ts end
  if diff_days == 0 then return "Today", due_ts end
  if diff_days == 1 then return "Tomorrow", due_ts end
  if due_ts <= saturday_end(t) then return "  This Week", due_ts end
  if due_ts <= month_end(t) then return "This Month", due_ts end
  return "Later", due_ts
end

function M.classify_completed(date_str, now)
  local ts = M.date_to_time(date_str)
  if not ts then return "  Older", 1 end

  local t = local_date(now or os.time())
  local today = day_start(t)
  local this_week = today - ((t.wday - 2) % 7) * DAY
  if ts >= today then return "Today", 6 end
  if ts >= today - DAY then return "Yesterday", 5 end
  if ts >= this_week then return "This Week", 4 end
  if ts >= this_week - 7 * DAY then return "Last Week", 3 end
  if ts >= month_start(t) then return "This Month", 2 end
  return "Older", 1
end

local function append_bracketed_time_spent(parts, ts)
  if type(ts) ~= "table" then return end
  local h, m = ts.hours or 0, ts.minutes or 0
  if h > 0 or m > 0 then
    parts[#parts + 1] = "[" .. M.hours_minutes_text(h, m) .. "]"
  end
end

function M.loclist_task_text(task)
  local parts = {}
  local due = M.deadline_date(task.due)
  if due then parts[#parts + 1] = "[due:" .. due .. "]" end
  parts[#parts + 1] = task.text
  if task.status == "Doing" then
    parts[#parts + 1] = "[doing]"
  elseif task.status == "Paused" then
    parts[#parts + 1] = "[paused]"
  end
  append_bracketed_time_spent(parts, task.time_spent)
  return table.concat(parts, " ")
end

function M.loclist_review_text(task)
  local parts = {}
  if task.completed_at and task.completed_at ~= vim.NIL then
    parts[#parts + 1] = "[" .. task.completed_at .. "]"
  end
  parts[#parts + 1] = task.text
  append_bracketed_time_spent(parts, task.time_spent)
  return table.concat(parts, " ")
end

function M.loclist_entry(task, text)
  return {
    filename = vim.uri_to_fname(task.location.uri),
    lnum = task.location.range.start.line + 1,
    col = task.location.range.start.character + 1,
    text = text,
  }
end

return M
