local M = {}

local LONGFORM_NEXT = {
  increment = { todo = "doing", doing = "done", done = "todo", paused = "doing", fallback = "doing" },
  decrement = { done = "doing", doing = "paused", paused = "todo", todo = "done", fallback = "todo" },
}

local SHORTHAND_NEXT = {
  increment = { ["!"] = "*", ["*"] = "-", ["-"] = "!" },
  decrement = { ["-"] = "*", ["*"] = "!", ["!"] = "-" },
}

local HISTORY_LIMIT = 50
local history_stack = {}

local function cycle_status(line_text, direction)
  local block_start, block_end = line_text:find("%{%@task%s+[^}]+%}")
  if block_start then
    local task_block = line_text:sub(block_start, block_end)
    local status_start, status_end, status = task_block:find("status=(%w+)")
    if status then
      local next_status = LONGFORM_NEXT[direction][status:lower()] or LONGFORM_NEXT[direction].fallback
      local new_block = task_block:sub(1, status_start - 1) .. "status=" .. next_status .. task_block:sub(status_end + 1)
      return line_text:sub(1, block_start - 1) .. new_block .. line_text:sub(block_end + 1)
    end
  end

  local marker_pos = line_text:find("[!%*%-]%d%d%d%d%-%d%d%-%d%d")
  if marker_pos then
    local marker = line_text:sub(marker_pos, marker_pos)
    return line_text:sub(1, marker_pos - 1) .. SHORTHAND_NEXT[direction][marker] .. line_text:sub(marker_pos + 1)
  end

  return nil
end

--- @param line_text string
--- @return string|nil the updated line, or nil if no task matched
function M.increment_task_string(line_text)
  return cycle_status(line_text, "increment")
end

--- @param line_text string
--- @return string|nil the updated line, or nil if no task matched
function M.decrement_task_string(line_text)
  return cycle_status(line_text, "decrement")
end

local function push_history(bufnr, row, prev_line_text)
  if #history_stack >= HISTORY_LIMIT then
    table.remove(history_stack, 1)
  end
  table.insert(history_stack, { bufnr = bufnr, row = row, text = prev_line_text })
end

local function write_buffer(bufnr)
  pcall(vim.api.nvim_buf_call, bufnr, function()
    vim.cmd("write")
  end)
end

local function replace_line_and_write(bufnr, row, new_line)
  vim.api.nvim_buf_set_lines(bufnr, row - 1, row, false, { new_line })
  write_buffer(bufnr)
  -- The LSP inserts started_at/completed_at/time_spent in response to the first
  -- write; a second write persists those properties.
  vim.defer_fn(function()
    if vim.api.nvim_buf_is_valid(bufnr) and vim.bo[bufnr].modified then
      write_buffer(bufnr)
    end
  end, 200)
end

local function change_status(direction, bufnr, row)
  bufnr = bufnr or vim.api.nvim_get_current_buf()
  row = row or vim.api.nvim_win_get_cursor(0)[1]

  if not bufnr or not vim.api.nvim_buf_is_valid(bufnr) then
    return
  end
  if not vim.api.nvim_buf_is_loaded(bufnr) then
    vim.fn.bufload(bufnr)
  end

  local lines = vim.api.nvim_buf_get_lines(bufnr, row - 1, row, false)
  if not lines or #lines == 0 then return end
  local line_text = lines[1]

  local new_line = cycle_status(line_text, direction)
  if new_line then
    push_history(bufnr, row, line_text)
    replace_line_and_write(bufnr, row, new_line)
  end
end

--- Increment task status on a line; defaults to the current buffer and cursor line.
--- @param bufnr integer|nil
--- @param row integer|nil 1-indexed row
function M.increment(bufnr, row)
  change_status("increment", bufnr, row)
end

--- Decrement task status on a line; defaults to the current buffer and cursor line.
--- @param bufnr integer|nil
--- @param row integer|nil 1-indexed row
function M.decrement(bufnr, row)
  change_status("decrement", bufnr, row)
end

--- Undo the last status modification.
--- @return boolean success
function M.undo()
  local last = table.remove(history_stack)
  if not last then
    vim.notify("No task changes to undo", vim.log.levels.WARN)
    return false
  end

  if not vim.api.nvim_buf_is_valid(last.bufnr) then
    vim.fn.bufload(last.bufnr)
  end
  if not vim.api.nvim_buf_is_valid(last.bufnr) then
    return false
  end

  replace_line_and_write(last.bufnr, last.row, last.text)
  return true
end

return M
