local fmt = require("patto.task_format")
local lsp = require("patto.lsp")
local review_timeframe = require("patto.review_timeframe")

local M = {}

local MARKDOWN_FLAVORS = { "standard", "obsidian", "github" }

local function open_loclist(entries, height)
  vim.fn.setloclist(0, entries)
  vim.cmd("botright lopen " .. height)
  vim.cmd("setlocal nowrap")
end

local function loclist_entries(results, text_of)
  local entries = {}
  for _, task in ipairs(lsp.collect_results(results)) do
    entries[#entries + 1] = fmt.loclist_entry(task, text_of(task))
  end
  return entries
end

local function list_tasks()
  lsp.execute_command(0, "experimental/aggregate_tasks", {}, function(results)
    open_loclist(loclist_entries(results, fmt.loclist_task_text), 8)
  end)
end

local function review_tasks(opts)
  local arg = opts.args ~= "" and opts.args or "today"
  local arguments = review_timeframe.parse(arg)
  if not arguments then
    vim.notify('LspPattoTasksReview: invalid argument "' .. arg
      .. '". Use today|yesterday|this_week|last_week|this_month|YYYY-MM-DD:YYYY-MM-DD', vim.log.levels.ERROR)
    return
  end
  lsp.execute_command(0, "experimental/tasks_review", arguments, function(results)
    local entries = loclist_entries(results, fmt.loclist_review_text)
    if #entries == 0 then
      vim.notify("No completed tasks found for: " .. arg, vim.log.levels.INFO)
      return
    end
    open_loclist(entries, 10)
  end)
end

local function configured_flavor(client)
  local settings = client.config.settings or {}
  return ((settings.patto or {}).markdown or {}).defaultFlavor or "standard"
end

local function copy_as_markdown(client, opts)
  local explicit_flavor = opts.args ~= "" and opts.args or nil
  local args = { vim.uri_from_bufnr(0), vim.NIL, vim.NIL, explicit_flavor }
  if opts.range == 2 then
    args[2], args[3] = opts.line1 - 1, opts.line2 - 1
  end
  lsp.execute_command(0, "patto/renderAsMarkdown", args, function(results)
    for _, res in pairs(results) do
      if res.result and res.result ~= vim.NIL then
        vim.fn.setreg("+", res.result)
        vim.fn.setreg('"', res.result)
        print("Copied as markdown (" .. (explicit_flavor or configured_flavor(client)) .. ")")
        return
      end
    end
  end)
end

local function fire_and_forget(command)
  return function()
    lsp.execute_command(0, command, {}, function() end)
  end
end

function M.attach(client, bufnr)
  local function command(name, fn, opts)
    vim.api.nvim_buf_create_user_command(bufnr, name, fn, opts)
  end

  command("LspPattoTasks", list_tasks, { desc = "Aggregate tasks in a workspace" })
  command("LspPattoIncrementTask", function() require("patto.tasks").increment() end,
    { desc = "Increment task status on the current line" })
  command("LspPattoDecrementTask", function() require("patto.tasks").decrement() end,
    { desc = "Decrement task status on the current line" })
  command("LspPattoTwoHopLinks", function() PattoShowTwoHopLinks() end,
    { desc = "Show two-hop links for the current buffer" })
  command("LspPattoScanWorkspace", fire_and_forget("experimental/scan_workspace"),
    { desc = "Scan the workspace" })
  command("LspPattoSnapshotPapers", fire_and_forget("patto/snapshotPapers"),
    { desc = "Take a snapshot of papers" })
  command("LspPattoTasksReview", review_tasks, {
    desc = "Review completed tasks (today|yesterday|this_week|last_week|this_month|YYYY-MM-DD:YYYY-MM-DD)",
    nargs = "?",
    complete = function() return vim.deepcopy(review_timeframe.NAMED) end,
  })
  command("LspPattoCopyAsMarkdown", function(opts) copy_as_markdown(client, opts) end, {
    desc = "Copy buffer/selection as markdown to clipboard",
    nargs = "?",
    range = true,
    complete = function() return vim.deepcopy(MARKDOWN_FLAVORS) end,
  })
end

return M
