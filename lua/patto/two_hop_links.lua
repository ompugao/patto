local M = {}

local BUFFER_NAME = "patto://[2hop links]"
local NAMESPACE = vim.api.nvim_create_namespace("links")

local function find_buffer(name)
  for _, buf in ipairs(vim.api.nvim_list_bufs()) do
    if vim.api.nvim_buf_get_name(buf) == name then return buf end
  end
  return nil
end

local function focus_buffer(buf, split_height)
  for _, win in ipairs(vim.api.nvim_list_wins()) do
    if vim.api.nvim_win_get_buf(win) == buf then
      vim.api.nvim_set_current_win(win)
      return
    end
  end
  vim.cmd(string.format("botright %dsplit", split_height))
  vim.api.nvim_set_current_buf(buf)
end

local function create_scratch_buffer(name, split_height)
  local buf = vim.api.nvim_create_buf(false, true)
  vim.api.nvim_buf_set_name(buf, name)
  vim.cmd(string.format("botright %dsplit", split_height))
  vim.api.nvim_win_set_buf(0, buf)
  vim.bo[buf].buftype = "nofile"
  vim.bo[buf].swapfile = false
  vim.bo[buf].buflisted = false
  vim.bo[buf].modified = false
  vim.wo[0].wrap = false
  vim.wo[0].number = false
  vim.wo[0].relativenumber = false
  vim.wo[0].spell = false
  vim.wo[0].signcolumn = "no"
  return buf
end

local function open_scratch_buffer(name)
  local split_height = math.floor(vim.api.nvim_win_get_height(0) * 0.3)
  local existing = find_buffer(name)
  if existing then
    focus_buffer(existing, split_height)
    return existing
  end
  return create_scratch_buffer(name, split_height)
end

local function file_name(uri)
  return vim.uri_to_fname(uri):match("^.+/(.+)$")
end

local function link_lines(groups)
  local lines, urls = {}, {}
  for _, group in ipairs(groups) do
    local nearest_node, two_hop_links = group[1], group[2]
    lines[#lines + 1] = file_name(nearest_node)
    urls[#lines] = nearest_node
    for _, link in ipairs(two_hop_links) do
      lines[#lines + 1] = "  - " .. file_name(link)
      urls[#lines] = link
    end
  end
  return lines, urls
end

local function render(bufnr, groups)
  vim.api.nvim_buf_clear_namespace(bufnr, NAMESPACE, 0, -1)
  vim.api.nvim_buf_set_lines(bufnr, 0, -1, false, {})
  vim.bo[bufnr].modified = false

  if groups == nil or #groups == 0 then
    print("No 2hop links")
    return
  end

  local lines, urls = link_lines(groups)
  vim.api.nvim_buf_set_lines(bufnr, 0, -1, false, lines)
  for line, url in ipairs(urls) do
    vim.api.nvim_buf_set_extmark(bufnr, NAMESPACE, line - 1, 0, {
      virt_text = { { vim.uri_decode(url), "Comment" } },
      virt_text_pos = "eol_right_align",
      virt_text_hide = true,
      hl_mode = "combine",
    })
  end
  vim.bo[bufnr].modified = false
  vim.api.nvim_buf_set_keymap(bufnr, "n", "<CR>", ":lua PattoOpenLinkUnderCursor()<CR>", { noremap = true, silent = true })
end

function M.show()
  vim.lsp.buf_request(0, "workspace/executeCommand", {
    command = "experimental/retrieve_two_hop_notes",
    arguments = { vim.uri_from_bufnr(0) },
  }, function(err, result)
    if err then return end
    render(open_scratch_buffer(BUFFER_NAME), result)
  end)
end

function M.open_under_cursor()
  local link = vim.api.nvim_get_current_line():match("%s*-*%s*(.*)")
  if not link or link == "" then return end
  local bufnr = vim.api.nvim_get_current_buf()
  local row = vim.fn.line(".") - 1
  for _, extmark in ipairs(vim.api.nvim_buf_get_extmarks(bufnr, NAMESPACE, 0, -1, { details = true })) do
    if extmark[2] == row then
      vim.api.nvim_command("edit " .. extmark[4].virt_text[1][1])
      return
    end
  end
end

return M
