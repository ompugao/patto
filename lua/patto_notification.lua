---@type vim.lsp.Config
return {
  cmd = { "patto-notification" },
  filetypes = { "patto" },
  single_file_support = true,
  root_markers = { ".git" },
  capabilities = {
    offsetEncoding = { 'utf-8' },
  },
  docs = {
    description = [[
https://github.com/ompugao/patto
patto-notification, a standalone system notification LSP server for Patto Note.

This server tracks task transitions and triggers native desktop system notifications.
    ]],
  },
}
