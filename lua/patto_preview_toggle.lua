--- Toggle patto-preview-tui in a zoomed tmux pane with viewport sync.
---
--- Usage:
---   vim.keymap.set("n", "<leader>p", require("patto_preview_toggle").toggle)
---
--- For viewport sync, set this in patto-preview-tui.toml:
---   [editor]
---   cmd = '''nvim --server "$NVIM" --remote-expr "v:lua.require('patto_preview_toggle').schedule_restore({top_line}, {line})"'''
---   action = "quit"

local M = {}

--- Apply winrestview while keeping scrolloff from shifting the viewport.
local function restore_view(topline, lnum)
  local so = vim.o.scrolloff
  local siso = vim.o.sidescrolloff
  local safe_lnum = math.max(lnum, topline + so)
  local last_line = vim.fn.line("$")
  if safe_lnum > last_line then safe_lnum = last_line end

  vim.o.scrolloff = 0
  vim.o.sidescrolloff = 0
  vim.fn.winrestview({ topline = topline, lnum = safe_lnum })
  vim.o.scrolloff = so
  vim.o.sidescrolloff = siso
end

--- Schedule viewport restoration after the next VimResized event.
--- Called via --remote-expr from the TUI's editor command.
--- The VimResized autocmd fires when tmux unzoom resizes the terminal,
--- and vim.schedule ensures we run after all VimResized handlers.
--- @param topline number first visible line (1-indexed)
--- @param lnum number cursor line (1-indexed)
--- @return string empty string (required by --remote-expr)
function M.schedule_restore(topline, lnum)
  vim.api.nvim_create_autocmd("VimResized", {
    once = true,
    callback = function()
      vim.schedule(function()
        restore_view(topline, lnum)
      end)
    end,
  })
  return ""
end

--- Open patto-preview-tui for the current buffer in a zoomed tmux pane.
function M.toggle()
  if not vim.env.TMUX then
    vim.notify("patto_preview_toggle: not inside tmux", vim.log.levels.WARN)
    return
  end

  local file = vim.fn.expand("%:p")
  if file == "" then
    vim.notify("patto_preview_toggle: no file in current buffer", vim.log.levels.WARN)
    return
  end

  local line = vim.fn.line("w0")
  local binary = vim.g.patto_preview_tui_binary or "patto-preview-tui"
  local extra = vim.g.patto_preview_tui_extra_args or {}

  local cmd_parts = {
    vim.fn.shellescape(binary),
    vim.fn.shellescape(file),
    "--goto-line", tostring(line),
  }
  for _, arg in ipairs(extra) do
    table.insert(cmd_parts, vim.fn.shellescape(tostring(arg)))
  end
  local tui_cmd = table.concat(cmd_parts, " ")

  -- Pass $NVIM so the TUI's editor command can reach this Neovim instance.
  vim.fn.system({
    "tmux", "split-window", "-Z",
    "-e", "NVIM=" .. vim.v.servername,
    tui_cmd,
  })
end

return M
