## Advanced

### Task Management

![tasks](https://github.com/user-attachments/assets/e9945524-b430-496e-ae56-6a68bfd7c390)

#### Task syntax

Patto supports two syntaxes for tasks:

**Shorthand** — quick inline marker with an optional date:
```txt
!2024-12-31    Todo with deadline
*2024-12-31    In progress
-2024-12-31    Done
```

**Block form** — rich metadata via `{@task}` property:
```txt
buy milk {@task status=todo}
write report {@task status=doing due=2024-12-31 scheduled=2024-12-28}
send invoice {@task status=done completed_at=2024-03-15}
```

| Field | Values | Description |
|-------|--------|-------------|
| `status` | `todo` \| `doing` \| `done` | Task state (required) |
| `due` | `YYYY-MM-DD` | Hard deadline — when it must be done |
| `scheduled` | `YYYY-MM-DD` | Soft start date — when to begin working on it |
| `completed_at` | `YYYY-MM-DD` | Auto-inserted when task transitions to `done` |

#### Auto-completion tracking

When you change a task's status to `done` in your editor, the LSP server automatically inserts `completed_at=<today>` into the `{@task}` block via `workspace/applyEdit`. The date can be manually corrected afterwards.

#### Commands: pending tasks

View all non-done tasks sorted by deadline:

- **Vim/Neovim**: `:LspPattoTasks` — opens in location list
- **Vim/Neovim**: `:Trouble patto_tasks` — opens in [trouble.nvim](https://github.com/folke/trouble.nvim) grouped by deadline category
- **VS Code**: `Patto: Show Tasks` (command palette) — opens in sidebar tree view

#### Commands: review completed tasks

View tasks completed within a time window, sorted by `completed_at`:

- **Vim/Neovim**: `:LspPattoTasksReview [timeframe]`
  - `:LspPattoTasksReview` — today (default)
  - `:LspPattoTasksReview this_week` — current week (Mon–today)
  - `:LspPattoTasksReview 2024-03-01:2024-03-31` — custom date range
  Results open in the location list.

- **Vim/Neovim (trouble.nvim)**: `:Trouble patto_tasks_review` — completed tasks grouped by date.
  Change the timeframe programmatically:
  ```lua
  require("trouble.sources.patto_tasks_review").config.timeframe = "this_week"
  require("trouble").open({ mode = "patto_tasks_review" })
  ```

- **VS Code**: `Patto: Review Completed Tasks` — select timeframe interactively, results shown in a date-grouped QuickPick with jump-to-line.

#### CLI: task search

```sh
# Find all todo tasks
rg --vimgrep '.*@task.*todo' .

# Find tasks completed this month, sorted by date
rg --vimgrep 'completed_at=2024-03' . | \
  awk '{match($0, /completed_at=([0-9\-]+)/, m); print m[1], $0}' | sort
```

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
# Shell command to run. Placeholders substituted at runtime:
#   {file}     – absolute path to the current file
#   {line}     – source line of the focused item (Tab-selected link/image), or the
#                first visible line of the viewport if nothing is focused
#   {top_line} – first visible source line of the viewport (always)
# Omit cmd to fall back to $EDITOR +{line} {file}.
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

*Tmux + Neovim (single-pane toggle with viewport sync):*

Since users can follow links to other notes in the previewer, the editor
command must also open the current file in Neovim (not just restore the
viewport). The `--remote` flag opens `{file}`, then `--remote-expr`
schedules the viewport restore:

```toml
[editor]
cmd = '''nvim --server "$NVIM" --remote "{file}" && nvim --server "$NVIM" --remote-expr "v:lua.require('patto_preview_toggle').schedule_restore({top_line}, {line})"'''
action = "quit"
```

The toggle logic lives in `lua/patto_preview_toggle.lua`. Bind it:

```lua
vim.keymap.set("n", "<leader>p", require("patto_preview_toggle").toggle,
  { desc = "Toggle patto-preview-tui" })
```

Or with **lazy.nvim**:

```lua
{
  "ompugao/patto",
  ft = "patto",
  keys = {
    {
      "<leader>p",
      function() require("patto_preview_toggle").toggle() end,
      desc = "Toggle patto-preview-tui",
      ft = "patto",
    },
  },
}
```

*Tmux + Vim (single-pane toggle with viewport sync):*

Same workflow using `vim --servername` / `--remote` instead of Neovim's `--server`.
The toggle logic lives in `autoload/patto_preview_toggle.vim`. Bind it in your vimrc:

```vim
nnoremap <leader>p :call patto_preview_toggle#toggle()<CR>
```

Configure the TUI to call back into Vim via `--remote-expr`:

```toml
[editor]
cmd = '''vim --servername "$VIM_SERVERNAME" --remote "{file}" && vim --servername "$VIM_SERVERNAME" --remote-expr "patto_preview_toggle#schedule_restore({top_line}, {line})"'''
action = "quit"
```

`$VIM_SERVERNAME` is set automatically by `patto_preview_toggle#toggle()` when it
launches the TUI pane, so no extra shell configuration is needed.

<details>
<summary>Why is the configuration complicated?</summary>
Launch the TUI from Neovim/Vim in a zoomed split pane.
When `e` is pressed, the TUI's editor command calls `--remote-expr` to schedule a viewport restore via a one-shot `VimResized` autocmd.
When tmux unzooms, the resize event fires the autocmd and `winrestview` snaps the viewport to the exact position.
</details>

Customisable via `g:` variables:

| Variable | Default | Description |
|---|---|---|
| `g:patto_preview_tui_binary` | `"patto-preview-tui"` | Path to the binary |
| `g:patto_preview_tui_extra_args` | `[]` | Extra CLI arguments (list) |

The full round-trip:
1. `<leader>p` → Neovim launches the TUI in a zoomed Tmux pane, scrolled to the current viewport top.
2. Browse the preview (follow links, check backlinks, etc.).
3. Press `e` → TUI runs the editor command (`--remote-expr` schedules a `VimResized` autocmd) and exits.
4. Tmux unzooms → `VimResized` fires → `winrestview` (scrolloff-safe) snaps the viewport.

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

To build with chafa yourself:
```sh
# Install build dependencies
sudo apt install libchafa-dev libsysprof-capture-4-dev   # Debian / Ubuntu

# Dynamic linking — links against system libchafa.so (must be present at runtime)
cargo build --release --features preview-tui-chafa-dyn

# Static linking — bundles chafa into the binary (Linux only, no runtime dependency)
cargo build --release --features preview-tui-chafa-static
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
# Find all todo tasks
rg --vimgrep '.*@task.*todo' . | \
  awk '{match($0, /due=([0-9:\-T]+)/, m); print (RLENGTH>0 ? m[1] : "9999-99-99"), $0}' | \
  sort | cut -d' ' -f2-

# Find tasks completed this week
rg --vimgrep 'completed_at=' . | \
  awk '{match($0, /completed_at=([0-9\-]+)/, m); print m[1], $0}' | sort -r | head -20
```
</details>

