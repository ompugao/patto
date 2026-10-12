local M = {}

local function set_fold_options()
  vim.wo.foldmethod = "expr"
  vim.wo.foldexpr = "v:lua.vim.lsp.foldexpr()"
  vim.wo.foldtext = "v:lua.require('patto.foldtext').foldtext()"
  vim.wo.foldlevel = 99
end

function M.attach(client, bufnr)
  if not client.config.lsp_folding then return end
  set_fold_options()
  local group = vim.api.nvim_create_augroup("patto_lsp_fold", { clear = false })
  vim.api.nvim_create_autocmd({ "BufEnter", "BufWinEnter" }, {
    group = group,
    buffer = bufnr,
    callback = set_fold_options,
  })
end

return M
