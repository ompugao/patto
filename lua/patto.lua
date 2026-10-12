local two_hop_links = require("patto.two_hop_links")

function PattoShowTwoHopLinks()
  two_hop_links.show()
end

function PattoOpenLinkUnderCursor()
  two_hop_links.open_under_cursor()
end

---@type vim.lsp.Config
return {
  cmd = { "patto-lsp" },
  filetypes = { "patto" },
  single_file_support = true,
  root_markers = { ".git" },
  capabilities = {
    offsetEncoding = { "utf-8" },
    textDocument = {
      foldingRange = {
        dynamicRegistration = false,
        lineFoldingOnly = true,
      },
    },
  },
  -- Override with: vim.lsp.config('patto_lsp', { settings = { patto = { markdown = { defaultFlavor = 'obsidian' } } } })
  settings = {
    patto = {
      markdown = {
        defaultFlavor = "standard",
      },
    },
  },
  -- Enable with: vim.lsp.config('patto_lsp', { lsp_folding = true })
  lsp_folding = false,
  on_attach = function(client, bufnr)
    require("patto.folding").attach(client, bufnr)
    require("patto.commands").attach(client, bufnr)
  end,
  docs = {
    description = [[
https://github.com/ompugao/patto
patto-lsp, a language server for Patto Note
    ]],
  },
}
