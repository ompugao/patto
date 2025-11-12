---@diagnostic disable: inject-field
local Item = require("trouble.item")

---@type trouble.Source
local M = {}

--- Parse deadline from task and return a category and sort key
--- @param task table The task from LSP
--- @return string category The deadline category (Overdue, Today, This Week, etc.)
--- @return number sort_key Numeric sort key for ordering
local function parse_deadline(task)
  if not task.due then
    return "No Deadline", 9999999999
  end
  
  -- Extract date from Deadline enum variants
  local date_str = nil
  if type(task.due) == "table" then
    if task.due.Date then
      date_str = task.due.Date
    elseif task.due.DateTime then
      date_str = string.match(task.due.DateTime, "^%d+%-%d+%-%d+")
    elseif task.due.Uninterpretable then
      return "Uninterpretable", 9999999998
    end
  end
  
  if not date_str then
    return "Invalid Deadline", 9999999997
  end
  
  -- Parse YYYY-MM-DD
  local year, month, day = string.match(date_str, "^(%d+)%-(%d+)%-(%d+)")
  if not year then
    return "Invalid Deadline", 9999999997
  end
  
  local due_time = os.time({year = tonumber(year), month = tonumber(month), day = tonumber(day), hour = 0, min = 0, sec = 0})
  local now = os.time()
  local today_start = os.time(os.date("*t", now))
  
  local diff_days = math.floor((due_time - today_start) / 86400)
  local sort_key = due_time
  
  if diff_days < 0 then
    return "⚠️  Overdue", sort_key
  elseif diff_days == 0 then
    return "📅 Today", sort_key
  elseif diff_days == 1 then
    return "📆 Tomorrow", sort_key
  elseif diff_days <= 7 then
    return "📋 This Week", sort_key
  elseif diff_days <= 30 then
    return "📌 This Month", sort_key
  else
    return "📦 Later", sort_key
  end
end

---@diagnostic disable-next-line: missing-fields
M.config = {
  formatters = {
    deadline_group = function(ctx)
      local category, _ = parse_deadline(ctx.item.item or {})
      return {
        text = category,
      }
    end,
    deadline_date = function(ctx)
      local task = ctx.item.item
      if not task or not task.due then
        return { text = "" }
      end
      
      local due_str = ""
      if type(task.due) == "table" then
        if task.due.Date then
          due_str = task.due.Date
        elseif task.due.DateTime then
          due_str = string.match(task.due.DateTime, "^[^T]+")
        elseif task.due.Uninterpretable then
          due_str = task.due.Uninterpretable
        end
      end
      
      return {
        text = due_str,
        hl = "Comment",
      }
    end,
  },
  sorters = {
    deadline = function(item)
      -- Return the numeric sort key (timestamp) for sorting by deadline
      local _, sort_key = parse_deadline(item.item or {})
      return sort_key
    end,
  },
  modes = {
    patto_tasks = {
      mode = "patto_tasks",
      events = { "BufEnter", "BufWritePost" },
      source = "patto_tasks",
      desc = "Tasks grouped by deadline",
      groups = {
        { "deadline_group", format = "{deadline_group}" },
        { "filename", format = "{file_icon} {filename} {count}" },
      },
      sort = { "deadline", "filename", "pos" },
      format = "{deadline_date} {text}",
      win = {
        position = "bottom",
        size = 0.4,
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
        local deadline_group, _ = parse_deadline(task)
        
        items[#items + 1] = Item.new({
          buf = vim.fn.bufadd(vim.uri_to_fname(task.location.uri)),
          pos = { row, col },
          end_pos = { row, col },
          text = task.text,
          filename = vim.uri_to_fname(task.location.uri),
          item = task,
          source = "patto_tasks",
          deadline_group = deadline_group,
        })
      end
      ::continue::
    end
    
    cb(items)
  end)
end

return M
