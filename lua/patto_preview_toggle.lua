--- Toggle patto-preview-tui in a zoomed tmux pane with viewport sync.
---
--- Usage:
---   vim.keymap.set("n", "<leader>p", require("patto_preview_toggle").toggle)
---
--- For viewport sync, set this in patto-preview-tui.toml:
---   [editor]
---   cmd = '''nvim --server "$NVIM" --remote "{file}" && nvim --server "$NVIM" --remote-expr "v:lua.require('patto_preview_toggle').schedule_restore({top_line}, {line})"'''
---   action = "quit"

local M = {}

local function restore_view(topline, lnum)
  local so = vim.o.scrolloff
  local siso = vim.o.sidescrolloff
  local safe_lnum = math.min(math.max(lnum, topline + so), vim.fn.line("$"))

  -- scrolloff would move topline while the cursor is placed, so it is turned
  -- off for the duration of winrestview.
  vim.o.scrolloff = 0
  vim.o.sidescrolloff = 0
  vim.fn.winrestview({ topline = topline, lnum = safe_lnum })
  vim.o.scrolloff = so
  vim.o.sidescrolloff = siso
end

--- Called via --remote-expr from the TUI's editor command. The tmux unzoom
--- resizes the terminal, so the restore runs after that VimResized.
--- @param topline number first visible line (1-indexed)
--- @param lnum number cursor line (1-indexed)
--- @return string empty string, required by --remote-expr
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

  local binary = vim.g.patto_preview_tui_binary or "patto-preview-tui"
  local parts = { vim.fn.shellescape(binary), vim.fn.shellescape(file), "--goto-line", tostring(vim.fn.line("w0")) }
  for _, arg in ipairs(vim.g.patto_preview_tui_extra_args or {}) do
    parts[#parts + 1] = vim.fn.shellescape(tostring(arg))
  end
  local tui_cmd = table.concat(parts, " ")

  -- $NVIM lets the TUI's editor command reach this Neovim instance.
  vim.fn.system({
    "tmux", "split-window", "-Z",
    "-e", "NVIM=" .. vim.v.servername,
    tui_cmd,
  })
end

return M
