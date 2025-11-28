local loop = vim.loop

local opened_browser = {}

local function is_wsl()
  local f = io.open("/proc/version", "r")
  if f then
    local content = f:read("*a")
    f:close()
    return content:find("microsoft") ~= nil
  end
  return false
end

local function relative_filepath(filepath, root_dir)
  if not filepath or filepath == "" then
    return nil
  end

  if vim.fs and vim.fs.relpath then
    local ok, rel = pcall(vim.fs.relpath, filepath, root_dir)
    if ok and rel and rel ~= "" then
      return rel
    end
  end

  if root_dir and root_dir ~= "" then
    local prefix = root_dir
    if not prefix:match('[\\/]$') then
      prefix = prefix .. '/'
    end
    if filepath:sub(1, #prefix) == prefix then
      return filepath:sub(#prefix + 1)
    end
  end

  return filepath
end

local function should_open_browser()
  if vim.g.patto_enable_open_browser == nil then
    return false
  end
  return vim.g.patto_enable_open_browser
end

local function maybe_open_browser(root_dir, port, filepath)
  if not port or opened_browser[root_dir] or not should_open_browser() then
    return
  end

  local rel = relative_filepath(filepath, root_dir)
  local url_param = ''
  if rel and rel ~= '' then
    url_param = '?note=' .. rel
  end

  local os_name = loop.os_uname().sysname
  local cmd
  if is_wsl() or os_name == "Windows_NT" then
    cmd = { "cmd.exe", "/c", "start", "http://localhost:" .. port .. url_param }
  elseif os_name == "Linux" then
    cmd = { "xdg-open", "http://localhost:" .. port .. url_param }
  elseif os_name == "Darwin" then
    cmd = { "open", "http://localhost:" .. port .. url_param }
  else
    vim.notify("Unsupported OS for default browser launch", vim.log.levels.WARN)
  end

  if cmd then
    opened_browser[root_dir] = true
    vim.defer_fn(function()
      vim.fn.jobstart(cmd, { detach = true })
    end, 500)
  end
end

local function build_cmd(root_dir)
  local binary = vim.g.patto_preview_binary or "patto-preview"
  local port = vim.g.patto_preview_port or 3000
  local args = { binary }

  if root_dir and root_dir ~= "" then
    table.insert(args, root_dir)
  end

  if port then
    table.insert(args, "--port")
    table.insert(args, tostring(port))
  end

  table.insert(args, "--preview-lsp-stdio")

  local extra = vim.g.patto_preview_extra_args
  if type(extra) == "table" then
    for _, value in ipairs(extra) do
      table.insert(args, tostring(value))
    end
  end

  return args, port
end

local function on_new_config(new_config, root_dir)
  local cmd, port = build_cmd(root_dir)
  new_config.cmd = cmd
  new_config.cmd_cwd = root_dir
  new_config._patto_preview_port = port
end

local function on_attach(client, bufnr)
  local root_dir = client.config.root_dir
  if not root_dir then
    return
  end

  maybe_open_browser(root_dir, client.config._patto_preview_port, vim.api.nvim_buf_get_name(bufnr))
end

---@type vim.lsp.Config
return {
  cmd = { "patto-preview", "--preview-lsp-stdio" },
  filetypes = { "patto" },
  single_file_support = true,
  root_markers = { ".git" },
  flags = {
    allow_incremental_sync = true,
  },
  capabilities = {
    offsetEncoding = { 'utf-8' },
  },
  on_attach = on_attach,
  on_new_config = on_new_config,
  docs = {
    description = [[
https://github.com/ompugao/patto
patto-preview, a preview+LSP bridge for Patto Note.

This config launches `patto-preview` with the `--preview-lsp-stdio` flag so the
preview UI stays in sync with unsaved buffers. Customize the preview port or
binary via:
  let g:patto_preview_port = 3030
  let g:patto_preview_binary = '/path/to/patto-preview'
    ]],
  },
}
