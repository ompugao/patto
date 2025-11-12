--- Trouble.nvim integration for Patto tasks
--- Convenience wrapper for the Trouble.nvim source
---
--- The actual source is defined at lua/trouble/sources/patto_tasks.lua
--- which is auto-discovered by Trouble.nvim v3
---
--- @module patto.trouble

local M = {}

--- Open the Patto tasks in trouble.nvim
--- @return nil
function M.open()
  local ok, trouble = pcall(require, "trouble")
  if not ok then
    vim.notify("trouble.nvim is not installed", vim.log.levels.ERROR)
    return
  end

  trouble.open({ mode = "patto_tasks", focus = true })
end

--- Toggle the Patto tasks window in trouble.nvim
--- @return nil
function M.toggle()
  local ok, trouble = pcall(require, "trouble")
  if not ok then
    vim.notify("trouble.nvim is not installed", vim.log.levels.ERROR)
    return
  end

  trouble.toggle({ mode = "patto_tasks" })
end

--- Close the Patto tasks window in trouble.nvim
--- @return nil
function M.close()
  local ok, trouble = pcall(require, "trouble")
  if not ok then
    return
  end

  trouble.close({ mode = "patto_tasks" })
end

--- Refresh the Patto tasks in trouble.nvim
--- @return nil
function M.refresh()
  local ok, trouble = pcall(require, "trouble")
  if not ok then
    return
  end

  if trouble.is_open({ mode = "patto_tasks" }) then
    trouble.refresh({ mode = "patto_tasks" })
  end
end

--- Setup function for backward compatibility
--- Not needed in Trouble.nvim v3 (auto-discovery)
--- @deprecated
function M.setup()
  -- No-op - Trouble.nvim v3 auto-discovers sources
end

return M

