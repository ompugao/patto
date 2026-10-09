local launch = require("patto.preview_launch")

local opened_browser = {}

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
    if not prefix:match("[\\/]$") then
      prefix = prefix .. "/"
    end
    if filepath:sub(1, #prefix) == prefix then
      return filepath:sub(#prefix + 1)
    end
  end

  return filepath
end

local function preview_url(root_dir, port, filepath)
  local url = "http://localhost:" .. port
  local rel = relative_filepath(filepath, root_dir)
  if rel and rel ~= "" then
    url = url .. "?note=" .. rel
  end
  return url
end

local function maybe_open_browser(root_dir, port, filepath)
  if not port or opened_browser[root_dir] or not vim.g.patto_enable_open_browser then
    return
  end
  if launch.open_in_browser(preview_url(root_dir, port, filepath)) then
    opened_browser[root_dir] = true
  end
end

local function build_cmd(root_dir)
  local port = vim.g.patto_preview_port or 3000
  local positional = {}
  if root_dir and root_dir ~= "" then
    positional[1] = root_dir
  end
  local cmd = launch.command(
    vim.g.patto_preview_binary or "patto-preview",
    positional,
    { "--port", tostring(port), "--preview-lsp-stdio" },
    vim.g.patto_preview_extra_args
  )
  return cmd, port
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
    offsetEncoding = { "utf-8" },
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
