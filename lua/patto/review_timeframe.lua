local M = {}

M.NAMED = { "today", "yesterday", "this_week", "last_week", "this_month" }

function M.parse(arg)
  if vim.tbl_contains(M.NAMED, arg) then return { timeframe = arg } end
  local from, to = string.match(arg, "^(%d%d%d%d%-%d%d%-%d%d):(%d%d%d%d%-%d%d%-%d%d)$")
  if from and to then return { from = from, to = to } end
  return nil
end

return M
