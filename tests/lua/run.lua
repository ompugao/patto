-- Headless checks for the Neovim plugin's pure functions.
-- Run from the repository root:  nvim -l tests/lua/run.lua

local root = vim.fs.normalize(vim.fn.fnamemodify(debug.getinfo(1, "S").source:sub(2), ":p:h:h:h"))
package.path = root .. "/lua/?.lua;" .. root .. "/lua/?/init.lua;" .. package.path
vim.opt.runtimepath:prepend(root)

if not pcall(require, "trouble.item") then
  package.preload["trouble.item"] = function()
    return { new = function(fields) return fields end }
  end
end

local passed, failed = 0, {}

local function check(name, actual, expected)
  if vim.deep_equal(actual, expected) then
    passed = passed + 1
  else
    failed[#failed + 1] = string.format("%s\n  expected: %s\n  actual:   %s", name, vim.inspect(expected), vim.inspect(actual))
  end
end

local fmt = require("patto.task_format")
local review_timeframe = require("patto.review_timeframe")
local tasks = require("patto.tasks")
local launch = require("patto.preview_launch")

local now = os.time({ year = 2026, month = 10, day = 9, hour = 15, min = 30, sec = 0 }) -- a Friday

check("deadline_date Date", fmt.deadline_date({ Date = "2026-10-10" }), "2026-10-10")
check("deadline_date DateTime", fmt.deadline_date({ DateTime = "2026-10-10T09:00" }), "2026-10-10")
check("deadline_date nil", fmt.deadline_date(nil), nil)
check("deadline_date NIL", fmt.deadline_date(vim.NIL), nil)

check("hours_minutes_text both", fmt.hours_minutes_text(1, 30), "1h30m")
check("hours_minutes_text hours", fmt.hours_minutes_text(2, 0), "2h")
check("hours_minutes_text minutes", fmt.hours_minutes_text(0, 7), "7m")
check("hours_minutes_text zero", fmt.hours_minutes_text(0, 0), "0m")
check("time_spent_text table", fmt.time_spent_text({ hours = 0, minutes = 90 }), "90m")
check("time_spent_text missing", fmt.time_spent_text(nil), nil)
check("minutes_text normalizes", fmt.minutes_text(90), "1h30m")

local doing = { time_spent = { hours = 1, minutes = 15 }, started_at = { DateTime = "2026-10-09T14:05" } }
check("total_minutes adds live time", fmt.total_minutes(doing, now), 75 + 85)
check("total_time_spent_text", fmt.total_time_spent_text(doing, now), "2h40m")
check("total_time_spent_text none", fmt.total_time_spent_text({}, now), nil)
check("total_time_spent_text zero", fmt.total_time_spent_text({ time_spent = { hours = 0, minutes = 0 } }, now), nil)
check("live_minutes never negative", fmt.live_minutes({ DateTime = "2026-10-09T16:00" }, now), 0)
check("started_at_clock", fmt.started_at_clock({ DateTime = "2026-10-09T14:05" }), "14:05")
check("started_at_clock missing", fmt.started_at_clock(nil), nil)

local function due(date) return { due = { Date = date } } end
check("classify_due overdue", { fmt.classify_due(due("2026-10-01"), now) }, { "⚠Overdue", fmt.date_to_time("2026-10-01") })
check("classify_due today", (fmt.classify_due(due("2026-10-09"), now)), "Today")
check("classify_due tomorrow", (fmt.classify_due(due("2026-10-10"), now)), "Tomorrow")
check("classify_due this week (Saturday seen from Thursday)", (fmt.classify_due({ due = { DateTime = "2026-10-10T08:00" } }, now - 86400)), "  This Week")
check("classify_due next Sunday is not this week", (fmt.classify_due(due("2026-10-11"), now - 86400)), "This Month")
check("classify_due this month", (fmt.classify_due(due("2026-10-25"), now)), "This Month")
check("classify_due later", (fmt.classify_due(due("2026-11-01"), now)), "Later")
check("classify_due none", { fmt.classify_due({}, now) }, { "No Deadline", 9999999999 })
check("classify_due invalid", { fmt.classify_due(due("soon"), now) }, { "Invalid", 9999999998 })

check("classify_completed today", { fmt.classify_completed("2026-10-09", now) }, { "Today", 6 })
check("classify_completed yesterday", { fmt.classify_completed("2026-10-08", now) }, { "Yesterday", 5 })
check("classify_completed this week", { fmt.classify_completed("2026-10-05", now) }, { "This Week", 4 })
check("classify_completed last week", { fmt.classify_completed("2026-09-28", now) }, { "Last Week", 3 })
check("classify_completed this month (seen from Monday the 19th)", { fmt.classify_completed("2026-10-01", now + 10 * 86400) }, { "This Month", 2 })
check("classify_completed older", { fmt.classify_completed("2025-01-01", now) }, { "Older", 1 })
check("classify_completed missing", { fmt.classify_completed(nil, now) }, { "  Older", 1 })

local function loc(path, line)
  return { uri = vim.uri_from_fname(path), range = { start = { line = line, character = 2 } } }
end
check("loclist_task_text full", fmt.loclist_task_text({
  text = "write report", status = "Doing", due = { DateTime = "2026-10-09T18:00" }, time_spent = { hours = 1, minutes = 15 },
}), "[due:2026-10-09] write report [doing] [1h15m]")
check("loclist_task_text paused zero time", fmt.loclist_task_text({
  text = "wait", status = "Paused", time_spent = { hours = 0, minutes = 0 },
}), "wait [paused]")
check("loclist_task_text plain", fmt.loclist_task_text({ text = "todo", status = "Todo" }), "todo")
check("loclist_review_text", fmt.loclist_review_text({
  text = "shipped", completed_at = "2026-10-09", time_spent = { minutes = 30 },
}), "[2026-10-09] shipped [30m]")
check("loclist_review_text NIL completed_at", fmt.loclist_review_text({ text = "x", completed_at = vim.NIL }), "x")
check("loclist_entry", fmt.loclist_entry({ location = loc("/tmp/notes/a.pn", 3) }, "t"),
  { filename = "/tmp/notes/a.pn", lnum = 4, col = 3, text = "t" })

check("review parse named", review_timeframe.parse("this_week"), { "this_week" })
check("review parse custom", review_timeframe.parse("2026-01-01:2026-01-31"), { "custom", "2026-01-01", "2026-01-31" })
check("review parse invalid", review_timeframe.parse("bogus"), nil)
check("review recent_range", { review_timeframe.recent_range(now) }, { "2026-09-28", "2026-10-09" })
check("review recent_range month start wins", { review_timeframe.recent_range(now + 10 * 86400) }, { "2026-10-01", "2026-10-19" })

check("increment longform todo", tasks.increment_task_string("\tx {@task status=todo due=2026-10-10}"), "\tx {@task status=doing due=2026-10-10}")
check("increment longform keeps case-insensitive match", tasks.increment_task_string("\tx {@task status=Doing}"), "\tx {@task status=done}")
check("increment longform unknown status", tasks.increment_task_string("\tx {@task status=weird}"), "\tx {@task status=doing}")
check("increment longform without status falls to shorthand", tasks.increment_task_string("\tx {@task due=2026-10-10} !2026-10-10"), "\tx {@task due=2026-10-10} *2026-10-10")
check("increment shorthand cycle", {
  tasks.increment_task_string("a !2026-10-10"), tasks.increment_task_string("a *2026-10-10"), tasks.increment_task_string("a -2026-10-10"),
}, { "a *2026-10-10", "a -2026-10-10", "a !2026-10-10" })
check("decrement longform", {
  tasks.decrement_task_string("{@task status=done}"), tasks.decrement_task_string("{@task status=doing}"),
  tasks.decrement_task_string("{@task status=paused}"), tasks.decrement_task_string("{@task status=todo}"),
}, { "{@task status=doing}", "{@task status=paused}", "{@task status=todo}", "{@task status=done}" })
check("decrement shorthand cycle", {
  tasks.decrement_task_string("a -2026-10-10"), tasks.decrement_task_string("a *2026-10-10"), tasks.decrement_task_string("a !2026-10-10"),
}, { "a *2026-10-10", "a !2026-10-10", "a -2026-10-10" })
check("no task on line", tasks.increment_task_string("plain"), nil)

check("launch.command", launch.command("bin", { "/root" }, { "--port", "3000" }, { "--x", 1 }), { "bin", "/root", "--port", "3000", "--x", "1" })
check("launch.command ignores non-table extras", launch.command("bin", {}, {}, "nope"), { "bin" })
check("launch.browser_command ends with the url", launch.browser_command("http://localhost:1")[#launch.browser_command("http://localhost:1")], "http://localhost:1")

for _, name in ipairs({
  "patto", "patto.commands", "patto.folding", "patto.two_hop_links", "patto.lsp", "patto.tasks", "patto.task_format",
  "patto.review_timeframe", "patto.tasks_source", "patto.current_task", "patto.foldtext", "patto.preview_launch",
  "patto_preview", "patto_preview_toggle", "patto_preview_tui", "patto_tasks.trouble",
  "trouble.sources.patto_tasks", "trouble.sources.patto_tasks_review",
}) do
  local ok, err = pcall(require, name)
  check("require " .. name, ok and true or err, true)
end

local patto_lsp = require("patto")
check("patto_lsp config keeps its keys", {
  cmd = patto_lsp.cmd, filetypes = patto_lsp.filetypes, lsp_folding = patto_lsp.lsp_folding,
  root_markers = patto_lsp.root_markers, settings = patto_lsp.settings,
}, {
  cmd = { "patto-lsp" }, filetypes = { "patto" }, lsp_folding = false, root_markers = { ".git" },
  settings = { patto = { markdown = { defaultFlavor = "standard" } } },
})

local trouble_mode = require("trouble.sources.patto_tasks").config.modes.patto_tasks
check("patto_tasks mode format", trouble_mode.format, "{task_status}{task_due} {text}{task_time_spent}{task_started_at} {filename}")
check("patto_tasks mode keys", vim.tbl_keys(trouble_mode.keys) and #vim.tbl_keys(trouble_mode.keys), 3)
local review_mode = require("trouble.sources.patto_tasks_review").config.modes.patto_tasks_review
check("patto_tasks_review sort", review_mode.sort, { "completed_date_group_order", "completed_at", "filename", "pos" })

local ok_setup, setup_err = pcall(function()
  require("patto.current_task").setup()
  require("trouble.sources.patto_tasks").setup()
  require("patto_tasks.trouble").setup()
end)
check("setup() with defaults", ok_setup and true or setup_err, true)

for _, line in ipairs(failed) do io.stderr:write("FAIL " .. line .. "\n") end
io.stdout:write(string.format("%d passed, %d failed\n", passed, #failed))
os.exit(#failed == 0 and 0 or 1)
