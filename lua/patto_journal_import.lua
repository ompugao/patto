--- Patto Journal Import: split a single journal file into daily files.
---
--- Scans a .pn file for date-like top-level lines (YYYY-MM-DD) and splits
--- the content into separate daily files in a destination directory.
---
--- Usage:
---   :PattoJournalImport source.pn ~/Documents/notes/journal/
---
--- Or from Lua:
---   require('patto_journal_import').import('journal.pn', '~/Documents/notes/journal/')

local M = {}

local DATE_PATTERN = '^(%d%d%d%d%-?%d%d%-?%d%d)'

--- Normalize a date string to YYYY-MM-DD format.
local function normalize_date(s)
  -- Already YYYY-MM-DD
  if s:match('^%d%d%d%d%-%d%d%-%d%d$') then
    return s
  end
  -- YYYYMMDD
  if s:match('^%d%d%d%d%d%d%d%d$') then
    return s:sub(1, 4) .. '-' .. s:sub(5, 6) .. '-' .. s:sub(7, 8)
  end
  return nil
end

--- Check if a line at indent 0 looks like a date header.
--- Returns the normalized date string or nil.
local function detect_date_header(line)
  -- Must be at indent level 0 (no leading whitespace)
  if line:match('^%s') then return nil end
  -- Try to match a date at the start of the line
  local date_part = line:match(DATE_PATTERN)
  if date_part then
    return normalize_date(date_part)
  end
  return nil
end

--- Import a single journal.pn file into a directory of daily files.
--- @param source string path to the source .pn file
--- @param dest_dir string path to the destination directory
--- @param opts table? options: {dry_run=bool, overwrite=bool}
--- @return table report {files_created=number, files_skipped=number, sections=table}
function M.import(source, dest_dir, opts)
  opts = opts or {}
  source = vim.fn.expand(source)
  dest_dir = vim.fn.fnamemodify(vim.fn.expand(dest_dir), ':p')

  if vim.fn.filereadable(source) ~= 1 then
    vim.notify('patto_journal_import: cannot read ' .. source, vim.log.levels.ERROR)
    return { files_created = 0, files_skipped = 0, sections = {} }
  end

  -- Create dest dir if needed
  if vim.fn.isdirectory(dest_dir) ~= 1 then
    vim.fn.mkdir(dest_dir, 'p')
  end

  -- Read source file
  local f = io.open(source, 'r')
  if not f then
    vim.notify('patto_journal_import: cannot open ' .. source, vim.log.levels.ERROR)
    return { files_created = 0, files_skipped = 0, sections = {} }
  end
  local content = f:read('*a')
  f:close()

  local lines = {}
  for line in (content .. '\n'):gmatch('([^\n]*)\n') do
    table.insert(lines, line)
  end
  -- Remove trailing empty line artifact
  if #lines > 0 and lines[#lines] == '' and not content:match('\n$') then
    -- Actually keep it if content ends with newline
  end

  -- Scan for date headers at indent 0
  local sections = {}  -- {date, start_line, end_line, lines}
  local current_date = nil
  local current_lines = {}
  local preamble = {}

  for i, line in ipairs(lines) do
    local date = detect_date_header(line)
    if date then
      -- Save previous section
      if current_date then
        table.insert(sections, {
          date = current_date,
          lines = current_lines,
        })
      end
      current_date = date
      -- The date header line itself becomes part of the section content
      -- but typically users don't want the bare date as content in the daily file
      -- since the filename already encodes the date. We'll include indented content only.
      current_lines = {}
      -- If the line has more than just the date, include the rest
      local rest = line:sub(#date + 1):match('^%s*(.*)')
      if rest and rest ~= '' then
        table.insert(current_lines, rest)
      end
    elseif current_date then
      table.insert(current_lines, line)
    else
      table.insert(preamble, line)
    end
  end
  -- Save last section
  if current_date then
    table.insert(sections, {
      date = current_date,
      lines = current_lines,
    })
  end

  -- Write sections to files
  local files_created = 0
  local files_skipped = 0

  for _, section in ipairs(sections) do
    local dest_path = dest_dir .. section.date .. '.pn'

    if vim.fn.filereadable(dest_path) == 1 and not opts.overwrite then
      files_skipped = files_skipped + 1
      section.status = 'skipped (exists)'
    elseif opts.dry_run then
      section.status = 'would create'
    else
      -- Trim trailing empty lines
      local trimmed = section.lines
      while #trimmed > 0 and trimmed[#trimmed] == '' do
        table.remove(trimmed)
      end

      local file_content = table.concat(trimmed, '\n')
      if #trimmed > 0 then
        file_content = file_content .. '\n'
      end

      local out = io.open(dest_path, 'w')
      if out then
        out:write(file_content)
        out:close()
        files_created = files_created + 1
        section.status = 'created'
      else
        section.status = 'error: cannot write'
      end
    end
  end

  -- Handle preamble (content before any date header)
  if #preamble > 0 then
    -- Trim trailing empty lines from preamble
    while #preamble > 0 and preamble[#preamble] == '' do
      table.remove(preamble)
    end
    if #preamble > 0 then
      local preamble_path = dest_dir .. '_preamble.pn'
      if not opts.dry_run then
        local out = io.open(preamble_path, 'w')
        if out then
          out:write(table.concat(preamble, '\n') .. '\n')
          out:close()
        end
      end
    end
  end

  local report = {
    files_created = files_created,
    files_skipped = files_skipped,
    sections = sections,
  }

  if opts.dry_run then
    vim.notify(string.format(
      'patto_journal_import: dry run — would create %d files, skip %d',
      #sections - files_skipped, files_skipped
    ), vim.log.levels.INFO)
  else
    vim.notify(string.format(
      'patto_journal_import: created %d files, skipped %d',
      files_created, files_skipped
    ), vim.log.levels.INFO)
  end

  return report
end

--- Register the :PattoJournalImport command.
function M.setup()
  vim.api.nvim_create_user_command('PattoJournalImport', function(opts)
    local args = vim.split(opts.args, '%s+', { trimempty = true })
    if #args < 2 then
      vim.notify('Usage: :PattoJournalImport source.pn dest_dir/ [--dry-run] [--overwrite]',
        vim.log.levels.WARN)
      return
    end
    local source = args[1]
    local dest = args[2]
    local import_opts = {}
    for i = 3, #args do
      if args[i] == '--dry-run' then import_opts.dry_run = true end
      if args[i] == '--overwrite' then import_opts.overwrite = true end
    end
    M.import(source, dest, import_opts)
  end, {
    nargs = '+',
    desc = 'Import a single journal.pn into daily files',
    complete = 'file',
  })
end

return M
