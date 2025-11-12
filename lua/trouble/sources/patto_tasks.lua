---@diagnostic disable: inject-field
local Item = require("trouble.item")

---@type trouble.Source
local M = {}

---@diagnostic disable-next-line: missing-fields
M.config = {
  modes = {
    patto_tasks = {
      mode = "patto_tasks",
      events = { "BufEnter", "BufWritePost" },
      source = "patto_tasks",
      groups = {
        { "filename", format = "{file_icon} {filename} {count}" },
      },
      sort = { "filename", "pos" },
      format = "{text} {pos}",
      win = {
        position = "bottom",
        size = 0.3,
      },
    },
  },
}

--- Fetch tasks from Patto LSP server
--- @param cb function Callback to receive items
function M.get(cb)
  -- Find any patto buffer to make LSP request from
  local patto_bufnr = nil
  for _, bufnr in ipairs(vim.api.nvim_list_bufs()) do
    if vim.api.nvim_buf_is_loaded(bufnr) and vim.bo[bufnr].filetype == 'patto' then
      patto_bufnr = bufnr
      break
    end
  end

  if not patto_bufnr then
    cb({})
    return
  end

  vim.lsp.buf_request_all(patto_bufnr, 'workspace/executeCommand', {
    command = 'experimental/aggregate_tasks',
    arguments = {},
  }, function(results, _ctx, _config)
    local items = {} ---@type trouble.Item[]
    
    if not results then
      cb(items)
      return
    end

    for _, vres in pairs(results) do
      if vres.result == nil then
        goto continue
      end
      
      for _, task in ipairs(vres.result) do
        local row = task.location.range.start.line + 1
        local col = task.location.range.start.character + 1
        items[#items + 1] = Item.new({
          buf = vim.fn.bufadd(vim.uri_to_fname(task.location.uri)),
          pos = { row, col },
          end_pos = { row, col },
          text = task.text,
          filename = vim.uri_to_fname(task.location.uri),
          item = task,
          source = "patto_tasks",
        })
      end
      ::continue::
    end
    
    cb(items)
  end)
end

return M
