## Advanced

### Task Management

![tasks](https://github.com/user-attachments/assets/e9945524-b430-496e-ae56-6a68bfd7c390)

Commands: `:LspPattoTasks` or `:Trouble patto_tasks` ([trouble.nvim](https://github.com/folke/trouble.nvim))

### Markdown Import
```sh
$ patto-markdown-importer -f note.md -o note.pn
$ patto-markdown-importer -d path/to/markdown_dir -o path/to/patto_dir  # batch mode
```
### Markdown Export

```sh
$ patto-markdown-renderer -f note.pn -o note.md
$ patto-markdown-renderer -f note.pn --flavor obsidian  # autodetect [[wikilinks]]
$ patto-markdown-renderer -f note.pn --flavor github
```

### Zotero Integration

Build with `--features zotero` (enabled by default) and configure `~/.config/patto/patto-lsp.toml`:
```toml
[zotero]
user_id = "1234567"
api_key = "your_key"
endpoint = "http://127.0.0.1:23119/api/" # for communication with zotero on localhost
```

### Terminal Preview (`patto-preview-tui`)

A full-featured terminal UI preview — no browser needed.

```sh
patto-preview-tui note.pn
patto-preview-tui note.pn --dir /path/to/workspace
patto-preview-tui note.pn --lsp-port 9527   # override TCP LSP port (default: 9527)
```

**Keybindings:**

| Key | Action |
|-----|--------|
| `j` / `k` or `↓` / `↑` | Scroll one line |
| `Ctrl-F` / `Space` / `PageDown` | Page down |
| `Ctrl-B` / `PageUp` | Page up |
| `Ctrl-D` | Half-page down |
| `Ctrl-U` | Half-page up |
| `g` / `G` | Jump to top / bottom |
| `Tab` / `Shift-Tab` | Cycle focus through links & images |
| `Enter` | Open focused link / note / fullscreen image |
| `b` | Toggle backlinks popup |
| `e` | Open current line in editor (see [Editor integration](#editor-integration)) |
| `+` / `-` | Increase / decrease image display height |
| `r` / `Ctrl-L` | Reload file |
| `Backspace` / `Ctrl-O` | Navigate back |
| `q` / `Esc` | Quit (or close fullscreen image) |

#### Editor integration

Press `e` to open the current file at the current line in your editor. Behaviour is configured in `~/.config/patto/patto-preview-tui.toml`:

```toml
[editor]
# Shell command to run. {file} and {line} are substituted at runtime.
# Omit to fall back to $EDITOR +{line} {file}.
cmd = 'nvim +{line} "{file}"'

# What the TUI does after launching the command:
#   "suspend"    (default) – pause the TUI, wait for the editor to exit, then resume
#   "quit"       – fire the command and immediately exit the TUI
#   "background" – run the command in the background and keep the TUI running
action = "suspend"
```

**Workflow examples:**

*Standalone terminal (default — no config needed):*
```toml
[editor]
cmd    = 'nvim +{line} "{file}"'
action = "suspend"
```
Press `e` → TUI hides, Neovim opens at the right line. Quit Neovim → TUI resumes.

*Tmux + Neovim RPC (single-pane toggle):*

Launch the TUI from Neovim in a zoomed split pane (e.g. `tmux split-window -Z`).
```toml
[editor]
cmd    = 'nvim --server "$NVIM" --remote-send "<cmd>e +{line} {file}<CR>"'
action = "quit"
```
Press `e` → tells the existing Neovim server to jump to the file/line, then the TUI quits. Because the pane was zoomed, Tmux automatically returns focus to the Neovim pane behind it.

Add this to your `init.lua` to bind a key that opens the TUI for the current buffer:

```lua
-- Open patto-preview-tui for the current file in a zoomed tmux pane.
-- Press <leader>p to toggle; press 'e' inside the TUI to jump back here.
vim.keymap.set("n", "<leader>p", function()
  local file = vim.fn.expand("%:p")
  local line = vim.fn.line("w0")  -- first line of the current viewport
  -- Build the patto-preview-tui command, starting at the top of the viewport.
  local tui_cmd = string.format("patto-preview-tui %q --goto-line %d", file, line)
  -- Open a new zoomed pane in the current tmux window.
  -- -Z  zooms the pane so the TUI fills the terminal; when it quits,
  --     tmux automatically returns focus to this Neovim pane.
  -- -e  passes $NVIM so the TUI's 'e' binding can reach back via RPC.
  vim.fn.system({
    "tmux", "split-window", "-Z",
    "-e", "NVIM=" .. vim.v.servername,
    tui_cmd,
  })
end, { desc = "Open patto-preview-tui for current file" })
```

Or with **lazy.nvim**, add the keymap inside the patto plugin spec:

```lua
{
  "ompugao/patto",
  ft = "patto",
  keys = {
    {
      "<leader>p",
      function()
        local file = vim.fn.expand("%:p")
        local line = vim.fn.line("w0")  -- first line of the current viewport
        local tui_cmd = string.format("patto-preview-tui %q --goto-line %d", file, line)
        vim.fn.system({
          "tmux", "split-window", "-Z",
          "-e", "NVIM=" .. vim.v.servername,
          tui_cmd,
        })
      end,
      desc = "Open patto-preview-tui for current file",
    },
  },
  config = function()
    require("patto")
    vim.lsp.config("patto_lsp", {})
    vim.lsp.config("patto_preview_tui", {})
    vim.lsp.enable({ "patto_lsp", "patto_preview_tui" })
  end,
}

With the config above (`action = "quit"`), the full round-trip is:
1. `<leader>p` → Neovim spawns a zoomed Tmux pane running `patto-preview-tui`, scrolled to the top of the current viewport.
2. Browse the preview (follow links, check backlinks, etc.).
3. Press `e` → TUI sends an RPC command to the Neovim server to jump to the viewed line, then exits.
4. Tmux unzooms and Neovim is back in focus at the target line.

*VS Code (or any GUI editor):*
```toml
[editor]
cmd    = 'code --goto "{file}:{line}"'
action = "background"
```
Press `e` → VS Code opens the file while the TUI keeps running.

*Emacs client:*
```toml
[editor]
cmd    = 'emacsclient +{line} "{file}"'
action = "suspend"
```

You can also jump to a specific line on startup with `--goto-line` (`-g`):
```sh
patto-preview-tui note.pn --goto-line 42
```

**Image protocol** is auto-detected (kitty, iTerm2, sixel, halfblocks). Override with `--protocol`:
```sh
patto-preview-tui note.pn --protocol iterm2   # force iTerm2 protocol
patto-preview-tui note.pn --protocol kitty    # force kitty protocol
```

#### Building from source

Basic build (no native image-library dependencies):
```sh
cargo build --release --features preview-tui
```

Image display uses auto-detected terminal protocols (kitty, iTerm2, sixel, halfblocks). Pre-built Linux binaries from [GitHub Releases](https://github.com/ompugao/patto/releases) include [chafa](https://hpjansson.org/chafa/) statically linked for improved halfblocks rendering.

To build with static chafa yourself (Linux only):
```sh
# Install build dependencies
sudo apt install libchafa-dev libsysprof-capture-4-dev   # Debian / Ubuntu

# Build with chafa-static
cargo build --release --features preview-tui-chafa
```

### Google Calendar Sync

Sync task deadlines to Google Calendar with **[patto-gcal-sync](https://github.com/ompugao/patto-gcal-sync)** - a separate tool that keeps your Patto tasks in sync with Google Calendar events.


<details>
<summary>FAQ & Tips</summary>

**Why not Markdown?** Different parsers behave inconsistently (e.g., code fences in lists work in GitHub but not Obsidian).


- item
- ```python
  print('hello')
  ```
- item3

**Templates?** Use your editor's snippet engine ([LuaSnip](https://github.com/L3MON4D3/LuaSnip), [vim-vsnip](https://github.com/hrsh7th/vim-vsnip), etc.)

**CLI task search:**
```sh
rg --vimgrep '.*@task.*todo' . | \
  awk '{match($0, /due=([0-9:\-T]+)/, m); print (RLENGTH>0 ? m[1] : "9999-99-99"), $0}' | \
  sort | cut -d' ' -f2-
```
</details>

