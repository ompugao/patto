local M = {}

function M.command(binary, positional, flags, extra_args)
  local args = { binary }
  vim.list_extend(args, positional)
  vim.list_extend(args, flags)
  if type(extra_args) == "table" then
    for _, value in ipairs(extra_args) do
      args[#args + 1] = tostring(value)
    end
  end
  return args
end

local function is_wsl()
  local f = io.open("/proc/version", "r")
  if not f then return false end
  local content = f:read("*a")
  f:close()
  return content:find("microsoft") ~= nil
end

function M.browser_command(url)
  local os_name = vim.uv.os_uname().sysname
  if is_wsl() or os_name == "Windows_NT" then return { "cmd.exe", "/c", "start", url } end
  if os_name == "Linux" then return { "xdg-open", url } end
  if os_name == "Darwin" then return { "open", url } end
  return nil
end

function M.open_in_browser(url)
  local cmd = M.browser_command(url)
  if not cmd then
    vim.notify("Unsupported OS for default browser launch", vim.log.levels.WARN)
    return false
  end
  vim.defer_fn(function()
    vim.fn.jobstart(cmd, { detach = true })
  end, 500)
  return true
end

function M.is_port_open(host, port)
  local tcp = vim.uv.new_tcp()
  if not tcp then return false end
  local connected, done = false, false
  tcp:connect(host, port, function(err)
    connected = not err
    done = true
  end)
  vim.wait(200, function() return done end, 10)
  tcp:close()
  return connected
end

return M
