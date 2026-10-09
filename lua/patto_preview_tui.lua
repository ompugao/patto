local launch = require("patto.preview_launch")

local DEFAULT_PORT = 9527
local warned = false

local function warn_not_running()
  if warned then return end
  warned = true
  vim.schedule(function()
    vim.notify("patto-preview-tui is not running. Start it first", vim.log.levels.INFO)
    vim.defer_fn(function() warned = false end, 5000)
  end)
end

-- No-op RPC client that satisfies Neovim's LSP client interface.
-- Responds to 'initialize' with a minimal valid result so no asserts fire,
-- then immediately signals on_exit so Neovim cleans up the client.
local function noop_rpc(dispatchers)
  vim.schedule(function()
    if dispatchers.on_exit then
      dispatchers.on_exit(0, 0)
    end
  end)
  return {
    is_closing = function() return true end,
    terminate = function() end,
    request = function(_, _, callback)
      callback(nil, { capabilities = {} })
      return true, 1
    end,
    notify = function() return true end,
  }
end

---@type vim.lsp.Config
return {
  cmd = function(dispatchers)
    local port = vim.g.patto_preview_tui_port or DEFAULT_PORT
    if not launch.is_port_open("127.0.0.1", port) then
      warn_not_running()
      return noop_rpc(dispatchers)
    end
    return vim.lsp.rpc.connect("127.0.0.1", port)(dispatchers)
  end,
  filetypes = { "patto" },
  single_file_support = true,
  root_markers = { ".git" },
  flags = {
    allow_incremental_sync = true,
  },
  capabilities = {
    offsetEncoding = { "utf-8" },
  },
  docs = {
    description = [[
https://github.com/ompugao/patto
patto-preview-tui, a terminal preview with a TCP LSP bridge for Patto Note.

This config connects to the `patto-preview-tui` TCP LSP server so the
terminal preview stays in sync with unsaved buffers. The TUI must be
running before opening a .pn file. Customize the port via:
  let g:patto_preview_tui_port = 9527
    ]],
  },
}
