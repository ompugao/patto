--- Convenience wrapper around the Trouble.nvim source in
--- lua/trouble/sources/patto_tasks.lua, which Trouble v3 discovers by itself.

local M = {}

local MODE = "patto_tasks"

local function with_trouble(fn, missing_message)
  local ok, trouble = pcall(require, "trouble")
  if ok then
    fn(trouble)
  elseif missing_message then
    vim.notify(missing_message, vim.log.levels.ERROR)
  end
end

function M.open()
  with_trouble(function(trouble) trouble.open({ mode = MODE, focus = true }) end, "trouble.nvim is not installed")
end

function M.toggle()
  with_trouble(function(trouble) trouble.toggle({ mode = MODE }) end, "trouble.nvim is not installed")
end

function M.close()
  with_trouble(function(trouble) trouble.close({ mode = MODE }) end)
end

function M.refresh()
  with_trouble(function(trouble)
    if trouble.is_open({ mode = MODE }) then
      trouble.refresh({ mode = MODE })
    end
  end)
end

--- @deprecated Trouble v3 discovers sources itself; kept so existing configs that call it keep working.
function M.setup() end

return M
