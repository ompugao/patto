local M = {}

--- Custom foldtext for patto buffers.
--- Format: <indent><trimmed first line>  <N lines folded>
---
--- Tabs in foldtext are replaced with a single space by Neovim, so we expand
--- leading tabs manually to spaces using the buffer's tabstop setting so that
--- the visible text does not move when a fold is opened or closed.
function M.foldtext()
  local line = vim.fn.getline(vim.v.foldstart)
  local tabs = #(line:match('^\t*'))
  local text = vim.fn.trim(line)
  local n = vim.v.foldend - vim.v.foldstart
  local ts = vim.bo.tabstop or 4
  local indent = string.rep(' ', tabs * ts)
  return indent .. text .. '  +-- ' .. n .. ' lines folded'
end

return M
