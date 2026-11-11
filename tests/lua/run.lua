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

check("review parse named", review_timeframe.parse("this_week"), { timeframe = "this_week" })
check("review parse custom", review_timeframe.parse("2026-01-01:2026-01-31"), { from = "2026-01-01", to = "2026-01-31" })
check("review parse invalid", review_timeframe.parse("bogus"), nil)

check("increment longform todo", tasks.increment_task_string("\tx {@task status=todo due=2026-10-10}"), "\tx {@task status=doing due=2026-10-10}")
check("increment longform keeps case-insensitive match", tasks.increment_task_string("\tx {@task status=Doing}"), "\tx {@task status=done}")
check("increment longform unknown status", tasks.increment_task_string("\tx {@task status=weird}"), "\tx {@task status=doing}")
check("decrement longform", {
  tasks.decrement_task_string("{@task status=done}"), tasks.decrement_task_string("{@task status=doing}"),
  tasks.decrement_task_string("{@task status=paused}"), tasks.decrement_task_string("{@task status=todo}"),
}, { "{@task status=doing}", "{@task status=paused}", "{@task status=todo}", "{@task status=done}" })
check("no task on line", tasks.increment_task_string("plain"), nil)

check("launch.command", launch.command("bin", { "/root" }, { "--port", "3000" }, { "--x", 1 }), { "bin", "/root", "--port", "3000", "--x", "1" })
check("launch.command ignores non-table extras", launch.command("bin", {}, {}, "nope"), { "bin" })
check("launch.browser_command ends with the url", launch.browser_command("http://localhost:1")[#launch.browser_command("http://localhost:1")], "http://localhost:1")

local images = require("patto.images")
local function srcs(line)
  return vim.tbl_map(function(m) return m.src end, images.parse_line(line))
end
check("images.parse_line local path", images.parse_line("[@img ./a.png]"), { { col = 0, end_col = 14, src = "./a.png", kind = "img" } })
check("images.parse_line path then quoted alt", srcs('[@img ./a.png "alt text"]'), { "./a.png" })
check("images.parse_line quoted alt then path", srcs('[@img "alt" ./a.png]'), { "./a.png" })
check("images.parse_line unquoted alt then path", srcs("[@img My Caption ./path/to/image.png]"), { "./path/to/image.png" })
check("images.parse_line parent dir path", srcs("[@img ../v1.0/img.jpg]"), { "../v1.0/img.jpg" })
check("images.parse_line url then alt", srcs("[@img https://x/y.png alt]"), { "https://x/y.png" })
check("images.parse_line alt then url", srcs("[@img alt https://x/y.png]"), { "https://x/y.png" })
check("images.parse_line quoted alt then url", srcs('[@img "alt" https://x/y.png]'), { "https://x/y.png" })
check("images.parse_line bracket inside quoted alt", srcs('[@img "a ] b" ./a.png]'), { "./a.png" })
check("images.parse_line two images", images.parse_line('x [@img ./a.png] y [@img ./b.jpg "B"] z'), {
  { col = 2, end_col = 16, src = "./a.png", kind = "img" },
  { col = 19, end_col = 37, src = "./b.jpg", kind = "img" },
})
check("images.parse_line nested in decoration", images.parse_line("[* see [@img ./a.png]]"), { { col = 7, end_col = 21, src = "./a.png", kind = "img" } })
check("images.parse_line ignores @image", srcs("[@image ./a.png]"), {})
check("images.parse_line ignores empty", srcs("[@img]"), {})
check("images.parse_line ignores bare path", srcs("[@img a.png]"), {})
check("images.parse_line ignores plain text", srcs("plain text"), {})
check("images.parse_line embed url then title", images.parse_line("[@embed https://youtu.be/abc a talk]"), {
  { col = 0, end_col = 36, src = "https://youtu.be/abc", kind = "embed" },
})
check("images.parse_line embed title then url", srcs("[@embed a talk https://youtu.be/abc]"), { "https://youtu.be/abc" })
check("images.parse_line embed url then url", srcs("[@embed https://a.com/x https://b.com/y]"), { "https://a.com/x" })
check("images.parse_line embed local pdf", srcs("[@embed ./doc.pdf slides]"), { "./doc.pdf" })
check("images.parse_line embed and image on one line", vim.tbl_map(function(m) return m.kind end,
  images.parse_line("[@embed https://x.com/a/status/1] [@img ./a.png]")), { "embed", "img" })
check("images.parse_line ignores other commands", srcs("[@quote https://a.com/x] [@img ./a.png]"), { "./a.png" })
check("images.find adds rows", images.find({ "", "[@img ./a.png]" }, 10), { { row = 11, col = 0, end_col = 14, src = "./a.png", kind = "img" } })

local embeds = require("patto.embeds")
check("embeds.kind youtube", embeds.kind("https://www.youtube.com/watch?v=abc&t=1"), "youtube")
check("embeds.kind pdf with query", embeds.kind("https://a.com/x.PDF?dl=1"), "pdf")
check("embeds.kind local pdf", embeds.kind("./doc.pdf"), "pdf")
check("embeds.kind google photos", embeds.kind("https://photos.app.goo.gl/xyz"), "google_photos")
check("embeds.kind goo.gl outside photos", embeds.kind("https://goo.gl/maps/xyz"), "other")
check("embeds.kind twitter", embeds.kind("https://x.com/u/status/1"), "twitter")
check("embeds.kind dropbox is not twitter", embeds.kind("https://dropbox.com/s/1"), "other")
check("embeds.kind speakerdeck", embeds.kind("https://speakerdeck.com/u/deck"), "speakerdeck")
check("embeds.kind slideshare", embeds.kind("https://www.slideshare.net/u/deck"), "slideshare")
check("embeds.kind local non-pdf", embeds.kind("./a.txt"), nil)
check("embeds.youtube_id watch", embeds.youtube_id("https://youtube.com/watch?t=3&v=abc"), "abc")
check("embeds.youtube_id short", embeds.youtube_id("https://youtu.be/abc?si=x"), "abc")
check("embeds.youtube_id embed", embeds.youtube_id("https://www.youtube.com/embed/abc"), "abc")
check("embeds.youtube_id channel", embeds.youtube_id("https://www.youtube.com/@someone"), nil)
check("embeds.og_meta property first", embeds.og_meta('<meta property="og:image" content="https://a/b.png">', "og:image"), "https://a/b.png")
check("embeds.og_meta content first", embeds.og_meta("<meta content='https://a/b.png' property='og:image' />", "og:image"), "https://a/b.png")
check("embeds.og_meta name and entities", embeds.og_meta('<meta name="twitter:image" content="https://a/b?x=1&amp;y=&#39;2&#x27;">', "twitter:image"), "https://a/b?x=1&y='2'")
check("embeds.og_meta skips other tags", embeds.og_meta('<meta property="og:image:width" content="1"><meta property="og:image" content="https://a/c.png">', "og:image"), "https://a/c.png")
check("embeds.og_meta missing", embeds.og_meta("<html></html>", "og:image"), nil)
check("embeds.absolute protocol relative", embeds.absolute("//cdn/x.png", "https://a.com/p/q"), "https://cdn/x.png")
check("embeds.absolute root relative", embeds.absolute("/x.png", "https://a.com/p/q"), "https://a.com/x.png")
check("embeds.absolute path relative", embeds.absolute("x.png", "https://a.com/p/q"), "https://a.com/p/x.png")
check("embeds.absolute absolute", embeds.absolute("http://b/x.png", "https://a.com/"), "http://b/x.png")
check("embeds.google_photos_thumb", embeds.google_photos_thumb("https://lh3.googleusercontent.com/pw/AB=w600-h315-p-k"), "https://lh3.googleusercontent.com/pw/AB=w1280-h720-no")
check("embeds.tweet_thumb photo", embeds.tweet_thumb({ tweet = { media = { photos = { { url = "https://p/1.jpg" } } } } }), "https://p/1.jpg")
check("embeds.tweet_thumb video", embeds.tweet_thumb({ tweet = { media = { videos = { { thumbnail_url = "https://p/v.jpg" } } } } }), "https://p/v.jpg")
check("embeds.tweet_thumb text only", embeds.tweet_thumb({ tweet = { text = "hi" } }), nil)
check("embeds.tweet_thumb error", embeds.tweet_thumb(nil), nil)

for _, name in ipairs({
  "patto", "patto.commands", "patto.folding", "patto.two_hop_links", "patto.lsp", "patto.tasks", "patto.task_format",
  "patto.review_timeframe", "patto.tasks_source", "patto.current_task", "patto.foldtext", "patto.preview_launch",
  "patto.images", "patto.embeds",
  "patto_preview", "patto_preview_toggle", "patto_preview_tui", "patto_tasks.trouble",
  "trouble.sources.patto_tasks",
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
check("patto_tasks mode groups by placement", trouble_mode.groups, { { "group_key", format = "{task_group}" } })
check("patto_tasks mode sort", trouble_mode.sort, { "group_order", "rank", "filename", "pos" })

local function placement(task)
  local key, order, rank = fmt.task_placement(task, now)
  return { key, order, rank > 1e10 }
end
check("placement open task due today", placement({ status = "Todo", due = { Date = "2026-10-09" } }), { "today", 2, false })
check("placement overdue task", placement({ status = "Doing", due = { Date = "2026-10-01" } }), { "overdue", 1, false })
check("placement task without deadline", placement({ status = "Todo", due = { Uninterpretable = "" } }), { "no_deadline", 7, false })
check("placement task done today goes after open tasks in today",
  placement({ status = "Done", completed_at = "2026-10-09", due = { Date = "2026-10-01" } }), { "today", 2, true })
check("placement task archived earlier goes to done",
  placement({ status = "Archived", archived_at = "2026-10-06" }), { "done", 9, true })
check("placement newer resolved task ranks first",
  select(3, fmt.task_placement({ status = "Done", completed_at = "2026-10-08" }, now))
    < select(3, fmt.task_placement({ status = "Done", completed_at = "2026-10-06" }, now)), true)
check("days_before 29", fmt.days_before(29, now), "2026-09-10")

local tasks_trouble = require("trouble.sources.patto_tasks")
local formatters = tasks_trouble.config.formatters

local function fake_group(root, key, items)
  local node = { id = root.id .. "#" .. key, parent = root, children = {} }
  for _, item in ipairs(items) do node.children[#node.children + 1] = { item = item } end
  function node.count(self) return #self.children end
  root.index[node.id] = node
  return node
end
local fake_root = { id = "$root", index = {} }
function fake_root.get(self, id) return self.index[id] end
local function row(status, minutes)
  return {
    resolved = status == "Done" or status == "Archived",
    item = { status = status, time_spent = minutes and { hours = 0, minutes = minutes } or nil },
  }
end
local today_group = fake_group(fake_root, "today",
  { row("Todo"), row("Doing"), row("Todo"), row("Done", 60), row("Done"), row("Archived", 15) })
local tomorrow_group = fake_group(fake_root, "tomorrow", { row("Todo") })
local done_group = fake_group(fake_root, "done", { row("Done", 90), row("Archived", 45) })

check("today header counts what is left and what is done, not archived tasks",
  formatters.task_group({ item = { group_key = "today" }, node = today_group }).text, "Today  3 left · ✓ 2 done")
check("header without resolved tasks shows only the count",
  formatters.task_group({ item = { group_key = "tomorrow" }, node = tomorrow_group }).text, "Tomorrow  1")
check("done header counts done tasks, including today's, but not archived ones",
  formatters.task_group({ item = { group_key = "done" }, node = done_group }).text, "Done this week  ✓ 3 · ⏱ 2h30m")
check("resolved rows are struck through",
  formatters.text({ item = { text = "ship", resolved = true } }), { text = "ship", hl = "PattoTaskResolved" })
check("open rows keep the default text highlight", formatters.text({ item = { text = "write" } }), { text = "write" })
check("resolved highlight is struck through", vim.api.nvim_get_hl(0, { name = "PattoTaskResolved" }).strikethrough, true)
check("resolved row in done shows its date",
  formatters.task_due({ item = { resolved = true, group_key = "done", item = { completed_at = "2026-10-06" } } }).text, " 2026-10-06")
check("resolved row in today hides the date",
  formatters.task_due({ item = { resolved = true, group_key = "today", item = { completed_at = "2026-10-09" } } }).text, "")

local patto_lsp = require("patto.lsp")
local sent = {}
patto_lsp.find_patto_bufnr = function() return 1 end
patto_lsp.execute_command = function(_, command, arguments, handler)
  sent[#sent + 1] = { command = command, params = arguments[1] }
  handler({})
end
local fake_view = { refresh = function() end }
local function last_params()
  tasks_trouble.get(function() end)
  return sent[#sent].params
end

check("viewer asks for open tasks and this week's resolved tasks", last_params(),
  { open = true, resolved = { timeframe = "this_week", include_archived = false } })
check("viewer uses the single tasks command", sent[#sent].command, "patto/tasks")
trouble_mode.keys.a.action(fake_view)
check("a adds archived tasks", last_params().resolved.include_archived, true)
trouble_mode.keys.a.action(fake_view)
trouble_mode.keys.r.action(fake_view)
check("r widens to the last 30 days", last_params().resolved.from, fmt.days_before(29))
check("done header follows the range",
  formatters.task_group({ item = { group_key = "done" }, node = done_group }).text, "Done in last 30 days  ✓ 3 · ⏱ 2h30m")
trouble_mode.keys.r.action(fake_view)
check("r then shows every resolved task", last_params().resolved, { include_archived = false })
trouble_mode.keys.r.action(fake_view)
check("r wraps back to this week", last_params().resolved.timeframe, "this_week")
tasks_trouble.setup({ show_resolved = false })
check("show_resolved = false asks for open tasks only", last_params(), { open = true })
tasks_trouble.setup({ show_resolved = true })
check("the separate review mode is gone", (pcall(require, "trouble.sources.patto_tasks_review")), false)

local ok_setup, setup_err = pcall(function()
  require("patto.current_task").setup()
  require("trouble.sources.patto_tasks").setup()
  require("patto_tasks.trouble").setup()
end)
check("setup() with defaults", ok_setup and true or setup_err, true)

for _, line in ipairs(failed) do io.stderr:write("FAIL " .. line .. "\n") end
io.stdout:write(string.format("%d passed, %d failed\n", passed, #failed))
os.exit(#failed == 0 and 0 or 1)
