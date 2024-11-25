local util = require 'lspconfig.util'
local async = require 'lspconfig.async'

local function aggregate_tasks(bufnr)
  bufnr = util.validate_bufnr(bufnr)
  local clients = util.get_lsp_clients { bufnr = bufnr, name = 'patto-lsp' }
  for _, client in ipairs(clients) do
    vim.notify 'Aggregating tasks in a workspace'
    client.request('experimental/aggregate_tasks',
    function(res)
      -- TODO: Show tasks in a quickfix window
      vim.notify 'Tasks aggregated'
    end,
    function(err)
      if err then
        error(tostring(err))
      end
      vim.notify 'Tasks aggregated'
    end, 0)
  end
end

patto_lsp_config = {
  default_config = {
    cmd = { 'patto-lsp' },
    filetypes = { 'patto' },
    single_file_support = true,
    root_dir = function(fname)
      return util.find_git_ancestor(fname)
    end,
    capabilities = {
    },
  },
  commands = {
    LspPattoTasks = {
      function()
        aggregate_tasks(0)
      end,
      description = 'Aggregate tasks in a workspace',
    },
  },
  docs = {
    description = [[
https://github.com/ompugao/patto
patto-lsp, a language server for Patto Note
    ]],
  },
}

require('lspconfig.configs').patto_lsp = patto_lsp_config
