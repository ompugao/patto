local util = require 'lspconfig.util'
local async = require 'lspconfig.async'

local function aggregate_tasks(bufnr)
  bufnr = util.validate_bufnr(bufnr)
  local clients = util.get_lsp_clients { bufnr = bufnr, name = 'patto-lsp' }
  for _, client in ipairs(clients) do
    vim.notify 'Aggregating tasks in a workspace'
    client.request('experimental/aggregate_tasks',
    function(res)
      -- TODO: Show tasks in a location window
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
        local bufpath = vim.api.nvim_buf_get_name(0)
        vim.lsp.buf_request(0, 'workspace/executeCommand', {
          command = 'experimental/aggregate_tasks',
          arguments = {},
        }, function(_, result, _, _)
          if not result then
            return
          end
          local locs = vim.tbl_map(function(item)
            local location_item = {}
            location_item.filename = vim.uri_to_fname(item.location.uri)
            location_item.lnum = item.location.range.start.line + 1
            location_item.col = item.location.range.start.character + 1
            location_item.text = item.text
            return location_item
          end, result)
          if #locs == 0 then
            vim.cmd("echo 'No tasks found'")
            return
          end
          vim.fn.setloclist(0, locs)
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

require('lspconfig.configs').patto_lsp = patto_lsp_config
