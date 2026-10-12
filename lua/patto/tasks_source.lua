local Item = require("trouble.item")
local lsp = require("patto.lsp")

local M = {}

local function rename_views(mode)
  local ok, view_mod = pcall(require, "trouble.view")
  if not ok then return end
  for _, v in ipairs(view_mod.get({ mode = mode })) do
    local bufnr = v.view and v.view.win and v.view.win.buf
    if bufnr and vim.api.nvim_buf_is_valid(bufnr) then
      pcall(vim.api.nvim_buf_set_name, bufnr, mode)
      vim.bo[bufnr].syntax = "patto"
    end
  end
end

local function rename_views_on_enter(mode)
  local group = vim.api.nvim_create_augroup(mode .. "_trouble_bufname", { clear = true })
  vim.api.nvim_create_autocmd({ "BufEnter", "BufWinEnter", "FileType" }, {
    group = group,
    pattern = "*",
    callback = function(ev)
      if vim.bo[ev.buf].filetype == "trouble" then
        vim.schedule(function() rename_views(mode) end)
      end
    end,
  })
end

local function refresh_soon(view)
  vim.defer_fn(function() view:refresh() end, 250)
end

local function status_action(change)
  return function(view)
    local at = view:at()
    if at and at.item then
      require("patto.tasks")[change](at.item.buf, at.item.pos[1])
      refresh_soon(view)
    end
  end
end

function M.status_keys(keys)
  return {
    [keys.increment] = { action = status_action("increment"), desc = "Increment task status" },
    [keys.decrement] = { action = status_action("decrement"), desc = "Decrement task status" },
    [keys.undo] = {
      action = function(view)
        if require("patto.tasks").undo() then refresh_soon(view) end
      end,
      desc = "Undo last task status change",
    },
  }
end

function M.to_items(source, results, extra_fields)
  local items = {} ---@type trouble.Item[]
  for _, task in ipairs(lsp.collect_results(results)) do
    local row = task.location.range.start.line + 1
    local col = task.location.range.start.character + 1
    local filename = vim.uri_to_fname(task.location.uri)
    local fields = {
      buf = vim.fn.bufadd(filename),
      pos = { row, col },
      end_pos = { row, col },
      text = task.text,
      filename = filename,
      item = task,
      source = source,
    }
    for k, v in pairs(extra_fields(task)) do fields[k] = v end
    items[#items + 1] = Item.new(fields)
  end
  return items
end

---@class patto.TasksSourceSpec
---@field mode string trouble mode name; also used as the item source and buffer name
---@field command string workspace/executeCommand name
---@field arguments fun(): any[]
---@field fields fun(task: table): table extra item fields used by groups and sorters
---@field before_get? fun()

---@param spec patto.TasksSourceSpec
---@return trouble.Source
function M.new(spec)
  rename_views_on_enter(spec.mode)
  local source = {}
  function source.get(cb)
    if spec.before_get then spec.before_get() end
    local patto_bufnr = lsp.find_patto_bufnr()
    vim.schedule(function() rename_views(spec.mode) end)
    if not patto_bufnr then cb({}) return end
    lsp.execute_command(patto_bufnr, spec.command, spec.arguments(), function(results)
      cb(M.to_items(spec.mode, results, spec.fields))
    end)
  end
  return source
end

return M
