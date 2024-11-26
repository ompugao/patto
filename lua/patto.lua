local util = require 'lspconfig.util'
local async = require 'lspconfig.async'

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
        vim.lsp.buf_request_all(0, 'workspace/executeCommand', {
          command = 'experimental/aggregate_tasks',
          arguments = {},
        }, function(results, _ctx, _config)
          local alllocs = {}
          for _, vres in pairs(results) do
            if vres.result == nil then
              goto continue
            end
            local locs = vim.tbl_map(function(item)
              local location_item = {}
              location_item.filename = vim.uri_to_fname(item.location.uri)
              location_item.lnum = item.location.range.start.line + 1
              location_item.col = item.location.range.start.character + 1
              location_item.text = item.text
              return location_item
            end, vres.result)

            for k,v in ipairs(locs) do
              alllocs[k] = v
            end 

            ::continue::
          end
          vim.fn.setloclist(0, alllocs)
          vim.cmd("botright lopen 8")
        end)
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

local configs = require('lspconfig.configs')
if not configs.patto_lsp then
  configs.patto_lsp = patto_lsp_config
end
