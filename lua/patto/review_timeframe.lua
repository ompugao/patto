local M = {}

M.NAMED = { "today", "yesterday", "this_week", "last_week", "this_month" }

function M.parse(arg)
  if vim.tbl_contains(M.NAMED, arg) then return { arg } end
  local from, to = string.match(arg, "^(%d%d%d%d%-%d%d%-%d%d):(%d%d%d%d%-%d%d%-%d%d)$")
  if from and to then return { "custom", from, to } end
  return nil
end

function M.recent_range(now)
  now = now or os.time()
  local t = os.date("*t", now) --[[@as table]]
  local days_since_monday = (t.wday - 2) % 7
  local last_week_start = now - (days_since_monday + 7) * 86400
  local month_start = os.time({ year = t.year, month = t.month, day = 1, hour = 0, min = 0, sec = 0 })
  return os.date("%Y-%m-%d", math.min(last_week_start, month_start)), os.date("%Y-%m-%d", now)
end

return M
