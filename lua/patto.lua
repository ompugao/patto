local util = require 'lspconfig.util'
local async = require 'lspconfig.async'

local function open_scratch_buffer(name)
  local h = math.floor(vim.api.nvim_win_get_height(0) * 0.3)
  -- Check if buffer already exists
  for _, buf in ipairs(vim.api.nvim_list_bufs()) do
    if vim.api.nvim_buf_get_name(buf) == name then
      -- Find window containing the buffer and switch to it
      for _, win in ipairs(vim.api.nvim_list_wins()) do
        if vim.api.nvim_win_get_buf(win) == buf then
          vim.api.nvim_set_current_win(win)
          return buf
        end
      end
      -- If buffer exists but not visible, open in a split
      vim.cmd(string.format("botright %dsplit", h))
      vim.api.nvim_set_current_buf(buf)
      return buf
    end
  end

  -- Create new buffer
  local buf = vim.api.nvim_create_buf(false, true) -- No file, scratch buffer
  vim.api.nvim_buf_set_name(buf, name)

  -- Open in a split window
  vim.cmd(string.format("botright %dsplit", h))
  vim.api.nvim_win_set_buf(0, buf)

  -- Set buffer options
  vim.api.nvim_buf_set_option(buf, 'buftype', 'nofile')
  vim.api.nvim_buf_set_option(buf, 'swapfile', false)
  -- vim.api.nvim_buf_set_option(buf, 'readonly', true)
  vim.api.nvim_buf_set_option(buf, 'buflisted', false)
  vim.api.nvim_buf_set_option(buf, 'modified', false)
  vim.api.nvim_buf_set_option(buf, 'wrap', false)
  vim.api.nvim_buf_set_option(buf, 'number', false)
  vim.api.nvim_buf_set_option(buf, 'relativenumber', false)
  vim.api.nvim_buf_set_option(buf, 'spell', false)
  vim.api.nvim_buf_set_option(buf, 'signcolumn', 'no')

  return buf
end

function PattoShowTwoHopLinks()
  -- Get the current buffer's URI
  local uri = vim.uri_from_bufnr(0)

  -- Request two-hop links from the LSP server
  vim.lsp.buf_request(0, 'workspace/executeCommand', {
    command = 'experimental/retrieve_two_hop_notes',
    arguments = {uri},
  }, function(err, result, ctx, config)
    if err then
      -- print("Error: " .. err.message)
      return
    end

    -- Check if the "[2hop links]" buffer already exists
    local bufnr = open_scratch_buffer("patto://[2hop links]")
    local ns = vim.api.nvim_create_namespace('links')

    -- clear content
    for m, _, _ in ipairs(vim.api.nvim_buf_get_extmarks(bufnr, ns, 0, -1, {})) do
      vim.api.nvim_buf_del_extmark(bufnr, ns, m)
    end
    vim.api.nvim_buf_set_lines(bufnr, 0, -1, false, {})
    vim.api.nvim_buf_set_option(bufnr, 'modified', false)

    if result == nil or #result == 0 then
      print("No 2hop links")
      return
    end

    -- Populate the scratch buffer with the two-hop links
    local lines = {}
    local props = {}
    for _, group in ipairs(result) do
      local nearest_node = group[1]
      local two_hop_links = group[2]

      local nearest_node_filename = vim.uri_to_fname(nearest_node):match("^.+/(.+)$")
      table.insert(lines, nearest_node_filename)
      table.insert(props, {line = #lines, url = nearest_node})
      for _, link in ipairs(two_hop_links) do
        local link_filename = vim.uri_to_fname(link):match("^.+/(.+)$")
        table.insert(lines, '  - ' .. link_filename)
        table.insert(props, {line = #lines, url = link})
      end
    end
    vim.api.nvim_buf_set_lines(bufnr, 0, -1, false, lines)

    -- Add text properties for each link
    for _, prop in ipairs(props) do
      vim.api.nvim_buf_set_extmark(bufnr, ns, prop.line - 1, 0, {
        virt_text = {{vim.uri_decode(prop.url), 'Comment'}},
        virt_text_pos = 'eol_right_align',
        virt_text_hide = true,
        hl_mode = 'combine',
      })
    end
    vim.api.nvim_buf_set_option(bufnr, 'modified', false)

    -- Add key mapping to open the link under the cursor
    vim.api.nvim_buf_set_keymap(bufnr, 'n', '<CR>', ':lua PattoOpenLinkUnderCursor()<CR>', { noremap = true, silent = true })
  end)
end

function PattoOpenLinkUnderCursor()
  -- Get the link under the cursor
  local line = vim.api.nvim_get_current_line()
  local link = line:match('%s*-*%s*(.*)')
  -- Open the link in a new buffer
  if link and link ~= '' then
    local bufnr = vim.api.nvim_get_current_buf()
    local extmarks = vim.api.nvim_buf_get_extmarks(bufnr, vim.api.nvim_create_namespace('links'), 0, -1, {details = true})
    for _, extmark in ipairs(extmarks) do
      if extmark[2] == vim.fn.line('.') - 1 then
        vim.api.nvim_command('edit ' .. extmark[4].virt_text[1][1])
        return
      end
    end
  end
end

-- Add a key mapping to show two-hop links
-- vim.api.nvim_set_keymap('n', '<leader>th', '<cmd>lua PattoShowTwoHopLinks()<CR>', { noremap = true, silent = true })

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
    LspPattoTwoHopLinks = {
      PattoShowTwoHopLinks,
      description = 'Show two-hop links for the current buffer'
    }
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
