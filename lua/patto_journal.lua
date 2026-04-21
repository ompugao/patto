--- Patto Journal: virtual concatenated buffer for daily `.pn` files.
---
--- Opens a virtual buffer that shows daily journal files (YYYY-MM-DD.pn)
--- concatenated in reverse chronological order (newest first).
--- Edits are tracked via extmarks and written back to individual files on save.
---
--- Usage:
---   require('patto_journal').open('~/Documents/notes/journal/')
---   require('patto_journal').open_today('~/Documents/notes/journal/')
---
--- Commands (created when journal buffer opens):
---   :PattoJournal [dir] [count]     Open journal view
---   :PattoJournalToday [dir]        Open and jump to today
---   :PattoJournalGoto YYYY-MM-DD    Jump to date in current journal buffer
---
--- Configuration:
---   vim.g.patto_journal_dir    Default journal directory
---   vim.g.patto_journal_count  Number of recent days to show (default 30)
---   vim.g.patto_journal_order  "newest_first" (default) or "oldest_first"

local M = {}

local ns = vim.api.nvim_create_namespace('patto_journal')

-- Date pattern: YYYY-MM-DD
local DATE_PATTERN = '^(%d%d%d%d)%-(%d%d)%-(%d%d)$'

--- Highlight groups (set defaults if not already defined)
local function setup_highlights()
  vim.api.nvim_set_hl(0, 'PattoJournalSeparator', { default = true, link = 'Title' })
  vim.api.nvim_set_hl(0, 'PattoJournalToday', { default = true, link = 'Special' })
end

--- Parse a YYYY-MM-DD filename (without extension) and return a table {year, month, day}
--- or nil if invalid.
local function parse_date(name)
  local y, m, d = name:match(DATE_PATTERN)
  if not y then return nil end
  y, m, d = tonumber(y), tonumber(m), tonumber(d)
  if m < 1 or m > 12 or d < 1 or d > 31 then return nil end
  return { year = y, month = m, day = d, str = name }
end

--- Get the day-of-week name for a date string "YYYY-MM-DD".
local function day_of_week(date_str)
  local y, m, d = date_str:match(DATE_PATTERN)
  if not y then return '' end
  local t = os.time({ year = tonumber(y), month = tonumber(m), day = tonumber(d) })
  return os.date('%A', t)
end

--- Today's date as "YYYY-MM-DD".
local function today_str()
  return os.date('%Y-%m-%d')
end

--- Compare two date tables. Returns true if a > b (newer first).
local function date_gt(a, b)
  if a.year ~= b.year then return a.year > b.year end
  if a.month ~= b.month then return a.month > b.month end
  return a.day > b.day
end

--- Scan a directory for YYYY-MM-DD.pn files. Returns a sorted list of
--- {name = "YYYY-MM-DD", path = "/abs/path/YYYY-MM-DD.pn", date = {...}}.
local function scan_journal_dir(dir)
  dir = vim.fn.expand(dir)
  if vim.fn.isdirectory(dir) ~= 1 then
    return {}
  end
  local entries = {}
  local files = vim.fn.globpath(dir, '*.pn', false, true)
  for _, path in ipairs(files) do
    local basename = vim.fn.fnamemodify(path, ':t:r')
    local date = parse_date(basename)
    if date then
      table.insert(entries, { name = basename, path = path, date = date })
    end
  end
  return entries
end

--- Sort entries by date.
--- @param entries table[]
--- @param order string "newest_first" or "oldest_first"
local function sort_entries(entries, order)
  if order == 'oldest_first' then
    table.sort(entries, function(a, b) return date_gt(b.date, a.date) end)
  else
    table.sort(entries, function(a, b) return date_gt(a.date, b.date) end)
  end
end

--- Build the separator virtual line chunks for a date.
--- @return table[] chunks for virt_lines
local function make_separator(date_name, width)
  local dow = day_of_week(date_name)
  local label = string.format(' %s (%s) ', date_name, dow)
  local pad_char = '═'
  local left_pad = 3
  local right_pad = math.max(0, width - left_pad - vim.fn.strdisplaywidth(label) - 1)
  local line = string.rep(pad_char, left_pad) .. label .. string.rep(pad_char, right_pad)

  local hl = 'PattoJournalSeparator'
  if date_name == today_str() then
    hl = 'PattoJournalToday'
  end
  return { { line, hl } }
end

--- Construct the journal URI for a directory path.
local function journal_uri(dir)
  dir = vim.fn.fnamemodify(vim.fn.expand(dir), ':p')
  return 'patto-journal://' .. dir
end

--- Find or create the journal buffer for a directory.
--- @return number bufnr
local function get_or_create_buf(dir)
  local uri = journal_uri(dir)
  for _, buf in ipairs(vim.api.nvim_list_bufs()) do
    if vim.api.nvim_buf_is_valid(buf) and vim.api.nvim_buf_get_name(buf) == uri then
      return buf, false
    end
  end
  local buf = vim.api.nvim_create_buf(true, false)
  vim.api.nvim_buf_set_name(buf, uri)
  return buf, true
end

--- State stored per journal buffer (keyed by bufnr).
--- Each entry: { dir = string, files = {{name, path, extmark_id, date}}, order = string }
local journal_state = {}

--- Load journal files into a buffer.
--- @param buf number buffer handle
--- @param dir string journal directory (absolute)
--- @param opts table {count=number, order=string}
local function load_journal(buf, dir, opts)
  local count = opts.count or vim.g.patto_journal_count or 30
  local order = opts.order or vim.g.patto_journal_order or 'newest_first'

  local entries = scan_journal_dir(dir)
  sort_entries(entries, order)

  -- Limit to count
  if count > 0 and #entries > count then
    local limited = {}
    for i = 1, count do limited[i] = entries[i] end
    entries = limited
  end

  -- Ensure today's file exists (create empty if not)
  local today = today_str()
  local has_today = false
  for _, e in ipairs(entries) do
    if e.name == today then
      has_today = true
      break
    end
  end
  if not has_today then
    local today_path = vim.fn.fnamemodify(dir, ':p') .. today .. '.pn'
    -- Create the file on disk (empty)
    local f = io.open(today_path, 'a')
    if f then f:close() end
    local date = parse_date(today)
    if date then
      local new_entry = { name = today, path = today_path, date = date }
      if order == 'newest_first' then
        table.insert(entries, 1, new_entry)
      else
        table.insert(entries, new_entry)
      end
    end
  end

  -- Read all files and concatenate
  local all_lines = {}
  local file_records = {}  -- {name, path, start_line (0-indexed), line_count}

  for i, entry in ipairs(entries) do
    local start_line = #all_lines
    local lines = {}
    local f = io.open(entry.path, 'r')
    if f then
      local content = f:read('*a')
      f:close()
      if content and #content > 0 then
        -- Split by newlines, preserving structure
        for line in (content .. '\n'):gmatch('([^\n]*)\n') do
          table.insert(lines, line)
        end
        -- Remove trailing empty line from the split if content didn't end with newline
        if content:sub(-1) ~= '\n' and #lines > 0 and lines[#lines] == '' then
          table.remove(lines)
        end
      end
    end

    -- If file is empty, add one empty line so the section is editable
    if #lines == 0 then
      table.insert(lines, '')
    end

    -- Add a blank line separator between files (except before the first)
    if i > 1 then
      table.insert(all_lines, '')
      start_line = start_line + 1
    end

    for _, line in ipairs(lines) do
      table.insert(all_lines, line)
    end

    table.insert(file_records, {
      name = entry.name,
      path = entry.path,
      date = entry.date,
      start_line = start_line,
      line_count = #lines,
    })
  end

  -- Set buffer content
  vim.api.nvim_buf_set_option(buf, 'modifiable', true)
  vim.api.nvim_buf_set_lines(buf, 0, -1, false, all_lines)

  -- Clear old extmarks
  vim.api.nvim_buf_clear_namespace(buf, ns, 0, -1)

  -- Place extmarks with virtual text separators
  local win_width = 80
  if vim.api.nvim_get_current_win() then
    win_width = vim.api.nvim_win_get_width(vim.api.nvim_get_current_win())
  end

  for _, rec in ipairs(file_records) do
    local sep_chunks = make_separator(rec.name, win_width)
    rec.extmark_id = vim.api.nvim_buf_set_extmark(buf, ns, rec.start_line, 0, {
      virt_lines_above = true,
      virt_lines = { sep_chunks },
      right_gravity = false,
    })
  end

  -- Store state
  journal_state[buf] = {
    dir = vim.fn.fnamemodify(dir, ':p'),
    files = file_records,
    order = order,
  }

  vim.api.nvim_buf_set_option(buf, 'modified', false)
end

--- Save the journal buffer back to individual files.
--- @param buf number buffer handle
--- @return number files_written
local function save_journal(buf)
  local state = journal_state[buf]
  if not state then
    vim.notify('patto_journal: no journal state for this buffer', vim.log.levels.ERROR)
    return 0
  end

  -- Get all extmark positions (sorted by line)
  local marks = vim.api.nvim_buf_get_extmarks(buf, ns, 0, -1, {})
  -- marks: list of {id, row, col}

  -- Build a mapping from extmark_id to file record
  local id_to_file = {}
  for _, rec in ipairs(state.files) do
    id_to_file[rec.extmark_id] = rec
  end

  -- Sort marks by row position
  table.sort(marks, function(a, b) return a[2] < b[2] end)

  local total_lines = vim.api.nvim_buf_line_count(buf)
  local files_written = 0

  for idx, mark in ipairs(marks) do
    local rec = id_to_file[mark[1]]
    if rec then
      local start_line = mark[2]  -- 0-indexed
      local end_line
      if idx < #marks then
        end_line = marks[idx + 1][2]
        -- Skip the blank separator line between sections
        if end_line > 0 then
          local prev_line = vim.api.nvim_buf_get_lines(buf, end_line - 1, end_line, false)
          if prev_line[1] and prev_line[1] == '' then
            end_line = end_line - 1
          end
        end
      else
        end_line = total_lines
      end

      local lines = vim.api.nvim_buf_get_lines(buf, start_line, end_line, false)

      -- Remove trailing empty lines (don't write extra blank lines at end of file)
      while #lines > 0 and lines[#lines] == '' do
        table.remove(lines)
      end

      -- Write to file
      local content = table.concat(lines, '\n')
      if #lines > 0 then
        content = content .. '\n'
      end

      -- Atomic write: write to temp, then rename
      local tmp_path = rec.path .. '.tmp'
      local f = io.open(tmp_path, 'w')
      if f then
        f:write(content)
        f:close()
        os.rename(tmp_path, rec.path)
        files_written = files_written + 1
      else
        vim.notify('patto_journal: failed to write ' .. rec.path, vim.log.levels.ERROR)
      end

      -- Update the record's line info
      rec.start_line = start_line
      rec.line_count = end_line - start_line
    end
  end

  vim.api.nvim_buf_set_option(buf, 'modified', false)
  return files_written
end

--- Get the file record at the cursor position in a journal buffer.
--- @param buf number buffer handle
--- @return table|nil file record {name, path, date, extmark_id, start_line, line_count}
local function get_file_at_cursor(buf)
  local state = journal_state[buf]
  if not state then return nil end

  local cursor_line = vim.api.nvim_win_get_cursor(0)[1] - 1  -- 0-indexed
  local marks = vim.api.nvim_buf_get_extmarks(buf, ns, 0, -1, {})
  table.sort(marks, function(a, b) return a[2] < b[2] end)

  local id_to_file = {}
  for _, rec in ipairs(state.files) do
    id_to_file[rec.extmark_id] = rec
  end

  -- Find which section the cursor is in
  local current_rec = nil
  for _, mark in ipairs(marks) do
    if mark[2] <= cursor_line then
      current_rec = id_to_file[mark[1]]
    else
      break
    end
  end
  return current_rec
end

--- Jump to a specific date section in the journal buffer.
--- @param buf number
--- @param date_name string "YYYY-MM-DD"
--- @return boolean found
local function goto_date(buf, date_name)
  local state = journal_state[buf]
  if not state then return false end

  for _, rec in ipairs(state.files) do
    if rec.name == date_name then
      local mark = vim.api.nvim_buf_get_extmark_by_id(buf, ns, rec.extmark_id, {})
      if mark and #mark > 0 then
        vim.api.nvim_win_set_cursor(0, { mark[1] + 1, 0 })
        return true
      end
    end
  end
  return false
end

--- Jump to next/previous day section.
--- @param buf number
--- @param direction number 1 for next, -1 for previous
local function goto_adjacent_day(buf, direction)
  local state = journal_state[buf]
  if not state then return end

  local cursor_line = vim.api.nvim_win_get_cursor(0)[1] - 1
  local marks = vim.api.nvim_buf_get_extmarks(buf, ns, 0, -1, {})
  table.sort(marks, function(a, b) return a[2] < b[2] end)

  if direction > 0 then
    -- Find next section after cursor
    for _, mark in ipairs(marks) do
      if mark[2] > cursor_line then
        vim.api.nvim_win_set_cursor(0, { mark[2] + 1, 0 })
        return
      end
    end
  else
    -- Find previous section before cursor
    local prev_mark = nil
    for _, mark in ipairs(marks) do
      if mark[2] >= cursor_line then break end
      prev_mark = mark
    end
    if prev_mark then
      vim.api.nvim_win_set_cursor(0, { prev_mark[2] + 1, 0 })
    end
  end
end

--- Set up buffer-local options, commands, and keybindings for a journal buffer.
--- @param buf number
local function setup_journal_buffer(buf)
  -- Buffer options
  vim.api.nvim_buf_set_option(buf, 'filetype', 'patto')
  vim.api.nvim_buf_set_option(buf, 'buftype', 'acwrite')
  vim.api.nvim_buf_set_option(buf, 'swapfile', false)

  -- BufWriteCmd: intercept save
  vim.api.nvim_create_autocmd('BufWriteCmd', {
    buffer = buf,
    callback = function()
      local n = save_journal(buf)
      vim.notify(string.format('Journal saved: %d file(s) written', n), vim.log.levels.INFO)
    end,
  })

  -- Buffer-local keybindings
  vim.keymap.set('n', ']]', function() goto_adjacent_day(buf, 1) end,
    { buffer = buf, desc = 'Jump to next day' })
  vim.keymap.set('n', '[[', function() goto_adjacent_day(buf, -1) end,
    { buffer = buf, desc = 'Jump to previous day' })
  vim.keymap.set('n', 'gf', function()
    local rec = get_file_at_cursor(buf)
    if rec then
      vim.cmd('edit ' .. vim.fn.fnameescape(rec.path))
    end
  end, { buffer = buf, desc = 'Open source file for current section' })

  -- Override goto-definition to handle intra-journal navigation
  vim.keymap.set('n', 'gd', function()
    local state = journal_state[buf]
    if not state then
      vim.lsp.buf.definition()
      return
    end

    -- Build a set of loaded daily file names for quick lookup
    local loaded_dates = {}
    for _, rec in ipairs(state.files) do
      loaded_dates[rec.name] = true
    end

    -- Get the word under cursor to check if it's a date-based wiki link
    -- We'll use LSP goto-definition and intercept the result
    local params = vim.lsp.util.make_position_params()
    vim.lsp.buf_request(0, 'textDocument/definition', params, function(err, result, ctx, config)
      if err or not result then
        -- Fall back to default
        vim.lsp.buf.definition()
        return
      end

      -- Handle both Location and Location[] responses
      local locations = {}
      if result.uri then
        -- Single Location
        table.insert(locations, result)
      elseif result[1] and result[1].uri then
        -- Location[]
        locations = result
      elseif result.targetUri then
        -- LocationLink
        table.insert(locations, { uri = result.targetUri, range = result.targetRange })
      elseif result[1] and result[1].targetUri then
        -- LocationLink[]
        for _, l in ipairs(result) do
          table.insert(locations, { uri = l.targetUri, range = l.targetRange })
        end
      end

      if #locations == 0 then return end
      local target_uri = locations[1].uri
      local target_path = vim.uri_to_fname(target_uri)
      local target_basename = vim.fn.fnamemodify(target_path, ':t:r')

      -- Check if target is a daily file loaded in this journal buffer
      if loaded_dates[target_basename] then
        goto_date(buf, target_basename)
      else
        -- Use default LSP handler
        vim.lsp.util.jump_to_location(locations[1], 'utf-8')
      end
    end)
  end, { buffer = buf, desc = 'Goto definition (journal-aware)' })

  -- Command: jump to specific date
  vim.api.nvim_buf_create_user_command(buf, 'PattoJournalGoto', function(opts)
    local date = opts.args
    if not date or date == '' then
      vim.notify('Usage: :PattoJournalGoto YYYY-MM-DD', vim.log.levels.WARN)
      return
    end
    if not goto_date(buf, date) then
      vim.notify('Date ' .. date .. ' not found in journal buffer', vim.log.levels.WARN)
    end
  end, {
    nargs = 1,
    desc = 'Jump to a specific date in the journal buffer',
    complete = function(arglead)
      local state = journal_state[buf]
      if not state then return {} end
      local completions = {}
      for _, rec in ipairs(state.files) do
        if rec.name:find(arglead, 1, true) == 1 then
          table.insert(completions, rec.name)
        end
      end
      return completions
    end,
  })
end

--- Clean up when a journal buffer is deleted.
local function on_buf_delete(buf)
  journal_state[buf] = nil
end

--- Open the journal view for a directory.
--- @param dir string? journal directory (uses g:patto_journal_dir if nil)
--- @param opts table? {count=number, order=string}
function M.open(dir, opts)
  opts = opts or {}
  dir = dir or vim.g.patto_journal_dir
  if not dir or dir == '' then
    vim.notify('patto_journal: no directory specified. Set g:patto_journal_dir or pass a directory.', vim.log.levels.ERROR)
    return
  end
  dir = vim.fn.fnamemodify(vim.fn.expand(dir), ':p')
  if vim.fn.isdirectory(dir) ~= 1 then
    -- Create the directory
    vim.fn.mkdir(dir, 'p')
  end

  setup_highlights()

  local buf, is_new = get_or_create_buf(dir)

  -- Switch to the buffer
  vim.api.nvim_set_current_buf(buf)

  if is_new then
    setup_journal_buffer(buf)
    -- Clean up state when buffer is wiped
    vim.api.nvim_create_autocmd('BufWipeout', {
      buffer = buf,
      callback = function() on_buf_delete(buf) end,
    })
  end

  load_journal(buf, dir, opts)
end

--- Open the journal view and jump to today's entry.
--- @param dir string? journal directory
function M.open_today(dir, opts)
  M.open(dir, opts)
  local buf = vim.api.nvim_get_current_buf()
  if not goto_date(buf, today_str()) then
    vim.notify("patto_journal: today's entry not found", vim.log.levels.WARN)
  end
end

--- Register global commands.
function M.setup()
  vim.api.nvim_create_user_command('PattoJournal', function(opts)
    local args = vim.split(opts.args, '%s+', { trimempty = true })
    local dir = args[1] or nil
    local count = args[2] and tonumber(args[2]) or nil
    M.open(dir, { count = count })
  end, {
    nargs = '*',
    desc = 'Open patto journal view',
    complete = 'dir',
  })

  vim.api.nvim_create_user_command('PattoJournalToday', function(opts)
    local args = vim.split(opts.args, '%s+', { trimempty = true })
    local dir = args[1] or nil
    M.open_today(dir)
  end, {
    nargs = '?',
    desc = 'Open patto journal and jump to today',
    complete = 'dir',
  })
end

return M
