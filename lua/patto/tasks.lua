local M = {}

--- Helper to cycle status value (increment)
local function next_status_inc(status)
  status = status:lower()
  if status == "todo" then return "doing"
  elseif status == "doing" then return "done"
  elseif status == "done" then return "todo"
  elseif status == "paused" then return "doing"
  end
  return "doing" -- default fallback
end

--- Helper to cycle status value (decrement)
local function next_status_dec(status)
  status = status:lower()
  if status == "done" then return "doing"
  elseif status == "doing" then return "paused"
  elseif status == "paused" then return "todo"
  elseif status == "todo" then return "done"
  end
  return "todo" -- default fallback
end

--- Increment the status of a task string.
--- @param line_text string
--- @return string|nil: the updated line, or nil if no task matched
function M.increment_task_string(line_text)
  -- 1. Try longform {@task ... status=XXX ...}
  local start_idx, end_idx = line_text:find("%{%@task%s+[^}]+%}")
  if start_idx then
    local task_block = line_text:sub(start_idx, end_idx)
    local s_start, s_end, status = task_block:find("status=(%w+)")
    if status then
      local next_s = next_status_inc(status)
      local new_task_block = task_block:sub(1, s_start - 1) .. "status=" .. next_s .. task_block:sub(s_end + 1)
      return line_text:sub(1, start_idx - 1) .. new_task_block .. line_text:sub(end_idx + 1)
    end
  end

  -- 2. Try shorthand: !YYYY-MM-DD or *YYYY-MM-DD or -YYYY-MM-DD
  local s_start, s_end = line_text:find("[!%*%-]%d%d%d%d%-%d%d%-%d%d")
  if s_start then
    local char = line_text:sub(s_start, s_start)
    local next_char = "!"
    if char == "!" then next_char = "*"
    elseif char == "*" then next_char = "-"
    elseif char == "-" then next_char = "!"
    end
    return line_text:sub(1, s_start - 1) .. next_char .. line_text:sub(s_start + 1)
  end

  return nil
end

--- Decrement the status of a task string.
--- @param line_text string
--- @return string|nil: the updated line, or nil if no task matched
function M.decrement_task_string(line_text)
  -- 1. Try longform {@task ... status=XXX ...}
  local start_idx, end_idx = line_text:find("%{%@task%s+[^}]+%}")
  if start_idx then
    local task_block = line_text:sub(start_idx, end_idx)
    local s_start, s_end, status = task_block:find("status=(%w+)")
    if status then
      local next_s = next_status_dec(status)
      local new_task_block = task_block:sub(1, s_start - 1) .. "status=" .. next_s .. task_block:sub(s_end + 1)
      return line_text:sub(1, start_idx - 1) .. new_task_block .. line_text:sub(end_idx + 1)
    end
  end

  -- 2. Try shorthand: !YYYY-MM-DD or *YYYY-MM-DD or -YYYY-MM-DD
  local s_start, s_end = line_text:find("[!%*%-]%d%d%d%d%-%d%d%-%d%d")
  if s_start then
    local char = line_text:sub(s_start, s_start)
    local next_char = "!"
    if char == "-" then next_char = "*"
    elseif char == "*" then next_char = "!"
    elseif char == "!" then next_char = "-"
    end
    return line_text:sub(1, s_start - 1) .. next_char .. line_text:sub(s_start + 1)
  end

  return nil
end

--- Core helper to modify task status on a specific line of a buffer.
--- @param mode "inc"|"dec"
--- @param bufnr integer|nil
--- @param row integer|nil 1-indexed row
local history_stack = {}

--- Limit history to 50 entries
local function push_history(bufnr, row, prev_line_text)
  if #history_stack >= 50 then
    table.remove(history_stack, 1)
  end
  table.insert(history_stack, {
    bufnr = bufnr,
    row = row,
    text = prev_line_text,
  })
end

--- Core helper to modify task status on a specific line of a buffer.
--- @param mode "inc"|"dec"
--- @param bufnr integer|nil
--- @param row integer|nil 1-indexed row
local function toggle_status_impl(mode, bufnr, row)
  bufnr = bufnr or vim.api.nvim_get_current_buf()
  if not row then
    local cursor = vim.api.nvim_win_get_cursor(0)
    row = cursor[1]
  end

  if not bufnr or not vim.api.nvim_buf_is_valid(bufnr) then
    return
  end

  if not vim.api.nvim_buf_is_loaded(bufnr) then
    vim.fn.bufload(bufnr)
  end

  local lines = vim.api.nvim_buf_get_lines(bufnr, row - 1, row, false)
  if not lines or #lines == 0 then return end
  local line_text = lines[1]

  local new_line
  if mode == "inc" then
    new_line = M.increment_task_string(line_text)
  else
    new_line = M.decrement_task_string(line_text)
  end

  if new_line then
    -- Record history before modifying
    push_history(bufnr, row, line_text)

    vim.api.nvim_buf_set_lines(bufnr, row - 1, row, false, { new_line })
    pcall(vim.api.nvim_buf_call, bufnr, function()
      vim.cmd("write")
    end)
    -- Defer a second write to capture the LSP auto-inserted properties (started_at, completed_at, time_spent)
    vim.defer_fn(function()
      if vim.api.nvim_buf_is_valid(bufnr) and vim.api.nvim_buf_get_option(bufnr, "modified") then
        pcall(vim.api.nvim_buf_call, bufnr, function()
          vim.cmd("write")
        end)
      end
    end, 200)
  end
end

--- Increment task status on a specific line of a buffer.
--- If no arguments are provided, defaults to current buffer and cursor line.
--- @param bufnr integer|nil
--- @param row integer|nil 1-indexed row
function M.increment(bufnr, row)
  toggle_status_impl("inc", bufnr, row)
end

--- Decrement task status on a specific line of a buffer.
--- If no arguments are provided, defaults to current buffer and cursor line.
--- @param bufnr integer|nil
--- @param row integer|nil 1-indexed row
function M.decrement(bufnr, row)
  toggle_status_impl("dec", bufnr, row)
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

  if vim.api.nvim_buf_is_valid(last.bufnr) then
    vim.api.nvim_buf_set_lines(last.bufnr, last.row - 1, last.row, false, { last.text })
    pcall(vim.api.nvim_buf_call, last.bufnr, function()
      vim.cmd("write")
    end)
    -- Defer a second write to capture the LSP auto-inserted properties (started_at, completed_at, time_spent)
    vim.defer_fn(function()
      if vim.api.nvim_buf_is_valid(last.bufnr) and vim.api.nvim_buf_get_option(last.bufnr, "modified") then
        pcall(vim.api.nvim_buf_call, last.bufnr, function()
          vim.cmd("write")
        end)
      end
    end, 200)
    return true
  end
  return false
end

return M
