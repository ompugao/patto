---@diagnostic disable: inject-field
local Item = require("trouble.item")

---@type trouble.Source
local M = {}

-- ── helpers ──────────────────────────────────────────────────────────────────

--- Classify a "YYYY-MM-DD" string into a recency bucket.
--- Returns: label (string), sort order (number, higher = more recent)
local function classify_completed(date_str)
  if not date_str or date_str == "" then return "  Older", 1 end
  local y, mo, d = string.match(date_str, "^(%d%d%d%d)%-(%d%d)%-(%d%d)")
  if not y then return "  Older", 1 end

  local ts   = os.time({ year = tonumber(y), month = tonumber(mo), day = tonumber(d),
                          hour = 12, min = 0, sec = 0 })
  local now  = os.time()
  local t    = os.date("*t", now) --[[@as table]]
  local today_start = os.time({ year = t.year, month = t.month, day = t.day,
                                  hour = 0, min = 0, sec = 0 })
  local yest_start   = today_start - 86400
  local days_since_mon = (t.wday - 2) % 7
  local this_week_start = today_start - days_since_mon * 86400
  local last_week_start = this_week_start - 7 * 86400
  local month_start = os.time({ year = t.year, month = t.month, day = 1,
                                  hour = 0, min = 0, sec = 0 })

  if ts >= today_start      then return "Today",      6
  elseif ts >= yest_start   then return "Yesterday",  5
  elseif ts >= this_week_start then return "This Week", 4
  elseif ts >= last_week_start then return "Last Week", 3
  elseif ts >= month_start  then return "This Month", 2
  else                           return "Older",      1
  end
end

-- ── formatters ───────────────────────────────────────────────────────────────

---@diagnostic disable-next-line: missing-fields
M.config = {
  formatters = {
    -- Group header
    completed_date_group = function(ctx)
      return { text = ctx.item.completed_date_group or "  Older" }
    end,

    -- completed_at chip:  ✓ 2026-05-19
    task_completed_at = function(ctx)
      local s = (ctx.item.item or {}).completed_at or ""
      return { text = s ~= "" and ("✓ " .. s) or "", hl = "DiagnosticOk" }
    end,

    -- time_spent chip:  ⏱ 1h30m
    task_time_spent = function(ctx)
      local ts = (ctx.item.item or {}).time_spent
      if not ts or type(ts) ~= "table" then return { text = "" } end
      local h, m = ts.hours or 0, ts.minutes or 0
      local s
      if     h > 0 and m > 0 then s = string.format("⏱ %dh%dm", h, m)
      elseif h > 0            then s = string.format("⏱ %dh",    h)
      else                         s = string.format("⏱ %dm",    m)
      end
      return { text = " " .. s, hl = "DiagnosticInfo" }
    end,
  },

  sorters = {
    completed_at = function(item)
      local s = (item.item or {}).completed_at
      if not s then return 0 end
      local y, mo, d = string.match(s, "^(%d%d%d%d)%-(%d%d)%-(%d%d)")
      if not y then return 0 end
      return os.time({ year = tonumber(y), month = tonumber(mo), day = tonumber(d),
                       hour = 12, min = 0, sec = 0 })
    end,
    completed_date_group_order = function(item)
      return item.completed_date_group_order or 1
    end,
  },

  modes = {
    patto_tasks_review = {
      mode   = "patto_tasks_review",
      events = { "BufWritePost" },
      source = "patto_tasks_review",
      desc   = "Completed tasks grouped by recency",
      groups = {
        { "completed_date_group", format = "{completed_date_group}" },
      },
      sort   = { "completed_date_group_order", "completed_at", "filename", "pos" },
      -- Format: label | completed chip | time chip | file
      format = "{task_completed_at} {text}{task_time_spent} {filename}",
      win    = { position = "bottom", size = 0.20 },
    },
  },
}

-- ── source.get ───────────────────────────────────────────────────────────────

function M.get(cb, ctx)
  vim.schedule(function()
    local ok_view, view_mod = pcall(require, "trouble.view")
    if ok_view then
      local views = view_mod.get({ mode = "patto_tasks_review" })
      for _, v in ipairs(views) do
        if v.view and v.view.win and v.view.win.buf then
          local bufnr = v.view.win.buf
          if vim.api.nvim_buf_is_valid(bufnr) then
            pcall(vim.api.nvim_buf_set_name, bufnr, "patto_tasks_review")
            vim.bo[bufnr].syntax = "patto"
          end
        end
      end
    end
  end)

  local patto_bufnr = nil
  for _, bufnr in ipairs(vim.api.nvim_list_bufs()) do
    if vim.api.nvim_buf_is_loaded(bufnr) and vim.bo[bufnr].filetype == "patto" then
      patto_bufnr = bufnr
      break
    end
  end

  if not patto_bufnr then cb({}) return end

  -- Compute range: from (start of last week OR start of this month, whichever earlier) through today.
  local now   = os.time()
  local t     = os.date("*t", now) --[[@as table]]
  local today_str = os.date("%Y-%m-%d", now)
  local days_since_mon   = (t.wday - 2) % 7
  local last_week_start  = now - (days_since_mon + 7) * 86400
  local month_start      = os.time({ year = t.year, month = t.month, day = 1,
                                      hour = 0, min = 0, sec = 0 })
  local from_str = os.date("%Y-%m-%d", math.min(last_week_start, month_start))

  vim.lsp.buf_request_all(patto_bufnr, "workspace/executeCommand", {
    command   = "experimental/tasks_review",
    arguments = { "custom", from_str, today_str },
  }, function(results)
    local items = {} ---@type trouble.Item[]

    if not results then cb(items) return end

    for _, vres in pairs(results) do
      if vres.result == nil then goto continue end

      for _, task in ipairs(vres.result) do
        local row = task.location.range.start.line      + 1
        local col = task.location.range.start.character + 1
        local bucket, order = classify_completed(task.completed_at)

        items[#items + 1] = Item.new({
          buf                        = vim.fn.bufadd(vim.uri_to_fname(task.location.uri)),
          pos                        = { row, col },
          end_pos                    = { row, col },
          text                       = task.text,
          filename                   = vim.uri_to_fname(task.location.uri),
          item                       = task,
          source                     = "patto_tasks_review",
          completed_date_group       = bucket,
          completed_date_group_order = order,
        })
      end
      ::continue::
    end

    cb(items)

    -- Auto-resize the trouble window to fit item count
    vim.schedule(function()
      local unique_groups = {}
      for _, item in ipairs(items) do
        unique_groups[item.completed_date_group] = true
      end
      local group_count = 0
      for _ in pairs(unique_groups) do group_count = group_count + 1 end
      local target = math.max(#items + group_count, 1)
      local max_size = math.floor(vim.o.lines * 0.35)
      target = math.min(target, max_size)
      for _, win in ipairs(vim.api.nvim_list_wins()) do
        local buf = vim.api.nvim_win_get_buf(win)
        if vim.bo[buf].filetype == "trouble" then
          vim.api.nvim_win_set_height(win, target)
          break
        end
      end
    end)
  end)
end

local function rename_trouble_buffers()
  local ok_view, view_mod = pcall(require, "trouble.view")
  if not ok_view then return end

  local views = view_mod.get({ mode = "patto_tasks_review" })
  for _, v in ipairs(views) do
    if v.view and v.view.win and v.view.win.buf then
      local bufnr = v.view.win.buf
      if vim.api.nvim_buf_is_valid(bufnr) then
        pcall(vim.api.nvim_buf_set_name, bufnr, "patto_tasks_review")
        vim.bo[bufnr].syntax = "patto"
      end
    end
  end
end

local group = vim.api.nvim_create_augroup("patto_tasks_review_trouble_bufname", { clear = true })
vim.api.nvim_create_autocmd({ "BufEnter", "BufWinEnter", "FileType" }, {
  group = group,
  pattern = "*",
  callback = function(ev)
    if vim.bo[ev.buf].filetype == "trouble" then
      vim.schedule(rename_trouble_buffers)
    end
  end,
})

return M
