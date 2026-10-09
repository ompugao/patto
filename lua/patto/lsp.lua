local M = {}

function M.find_patto_bufnr()
  for _, bufnr in ipairs(vim.api.nvim_list_bufs()) do
    if vim.api.nvim_buf_is_loaded(bufnr) and vim.bo[bufnr].filetype == "patto" then
      return bufnr
    end
  end
  return nil
end

function M.execute_command(bufnr, command, arguments, handler)
  vim.lsp.buf_request_all(bufnr, "workspace/executeCommand", {
    command = command,
    arguments = arguments,
  }, handler)
end

function M.collect_results(results)
  local all = {}
  for _, response in pairs(results or {}) do
    if type(response.result) == "table" then
      for _, entry in ipairs(response.result) do
        all[#all + 1] = entry
      end
    end
  end
  return all
end

return M
