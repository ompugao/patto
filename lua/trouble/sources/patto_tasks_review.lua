---@diagnostic disable: inject-field
local Item = require("trouble.item")

---@type trouble.Source
local M = {}

---@diagnostic disable-next-line: missing-fields
M.config = {
  -- timeframe can be overridden when opening: trouble.open({ mode = "patto_tasks_review", timeframe = "this_week" })
  timeframe = "today",
  formatters = {
    completed_at = function(ctx)
      local task = ctx.item.item or {}
      return {
        text = task.completed_at or "",
        hl = "Comment",
      }
    end,
    completed_date_group = function(ctx)
      local task = ctx.item.item or {}
      return {
        text = task.completed_at or "Unknown",
      }
    end,
  },
  modes = {
    patto_tasks_review = {
      mode = "patto_tasks_review",
      events = { "BufWritePost" },
      source = "patto_tasks_review",
      desc = "Completed tasks grouped by date",
      groups = {
        { "completed_date_group", format = "📅 {completed_date_group}" },
      },
      sort = { "completed_date_group", "filename", "pos" },
      format = "{completed_at} {text} {filename}",
      win = {
        position = "bottom",
        size = 0.35,
      },
    },
  },
}

--- Fetch completed tasks from Patto LSP server
--- @param cb function Callback to receive items
function M.get(cb)
  local patto_bufnr = nil
  for _, bufnr in ipairs(vim.api.nvim_list_bufs()) do
    if vim.api.nvim_buf_is_loaded(bufnr) and vim.bo[bufnr].filetype == "patto" then
      patto_bufnr = bufnr
      break
    end
  end

  if not patto_bufnr then
    cb({})
    return
  end

  -- Read timeframe from config (can be set at open time)
  local timeframe = M.config.timeframe or "today"
  local arguments = {}
  if timeframe == "today" or timeframe == "this_week" then
    arguments = { timeframe }
  else
    -- Expect "YYYY-MM-DD:YYYY-MM-DD"
    local from, to = string.match(tostring(timeframe), "^(%d%d%d%d%-%d%d%-%d%d):(%d%d%d%d%-%d%d%-%d%d)$")
    if from and to then
      arguments = { "custom", from, to }
    else
      arguments = { "today" }
    end
  end

  vim.lsp.buf_request_all(patto_bufnr, "workspace/executeCommand", {
    command = "experimental/tasks_review",
    arguments = arguments,
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
          buf      = vim.fn.bufadd(vim.uri_to_fname(task.location.uri)),
          pos      = { row, col },
          end_pos  = { row, col },
          text     = task.text,
          filename = vim.uri_to_fname(task.location.uri),
          item     = task,
          source   = "patto_tasks_review",
        })
      end
      ::continue::
    end

    cb(items)
  end)
end

return M
