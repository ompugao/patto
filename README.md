# <picture>  <source media="(prefers-color-scheme: dark)" srcset="assets/patto_icon_dark.png">  <source media="(prefers-color-scheme: light)" srcset="assets/patto_icon_light.png">  <img width="50" height="50" alt="patto_logo" src="assets/patto_icon_dark.png" /></picture> Patto Note

A simple plain-text format for note-taking, outlining, and task management. Inspired by [Cosense/Scrapbox](https://scrapbox.io), powered by LSP.



![demo.gif](https://github.com/user-attachments/assets/8e1772dc-e4a7-4b22-bf83-c434e73726fb)

## What is Patto?

A **line-oriented** text format where newlines create lines and tabs create nesting. Perfect for:
- 📝 Quick outlining and note-taking
- ✅ Task management with deadlines
- 🔗 [Zettelkasten](https://zettelkasten.de/introduction/)-style linked notes

## Features

- Wiki-style links `[note name]` with backlinks and 2-hop visualization
- Tasks with deadlines: `!2024-12-31` or `{@task due=2024-12-31}` (sorted by Overdue, Today, This Week)
- Rich task metadata: `scheduled`, `completed_at` with auto-tracking when a task is marked done
- Real-time preview and LSP-powered autocomplete
- Works with Vim, Neovim, VS Code

## Syntax

### Basic
```txt
Plain text
	Tab to nest
		Tab twice for deeper nesting
    Anchored text  #anchor

[* bold]  [/ italic]  [` code `]
```

### Links & Tasks
```txt
[other note]                     Link to note
[note#anchor]                    Link to the anchored line in note
[https://example.com Title]     External link

!2024-12-31    Todo with deadline (shorthand)
*2024-12-31    In progress
-2024-12-31    Done

{@task status=todo due=2024-12-31}                       Block form, todo
{@task status=doing due=2024-12-31 scheduled=2024-12-30} Include scheduled date
{@task status=done completed_at=2024-03-15}              Done — completed_at auto-inserted
```

### Blocks
```txt
[@code python]
	print("hello")

[@quote]
	Quoted text

[@table]
	header	col1	col2
	row1	a	b

[@math]
    \sum_{n=0}^{10} = 55
```

### Content embeddings
```txt
[@img http://example.com/img "image alt"]    Remote image
[@img ./local-image.png "image alt"]         Local image (must use ./ prefix)
[@embed https://www.youtube.com/watch?v=dQw4w9WgXcQ Youtube Alt]
[@embed https://twitter.com/... Tweet]
[@embed https://speakerdeck.com/... Slide]
[@embed ./document.pdf PDF Title]            Local PDF embedding (must use ./ prefix)
```

<details>
<summary>See full rendered example</summary>
<img width="906" alt="rendered-example" src="https://github.com/user-attachments/assets/60e18d5f-f92d-4a50-9a3e-f0c60fc0ba2b" />
</details>

## Installation

**Install with cargo:**
```sh
cargo install patto
```

**Or download from:** [GitHub Releases](https://github.com/ompugao/patto/releases)

### Editor Setup

<details>
<summary><b>Neovim (nvim-lspconfig)</b></summary>

```vim
Plug 'neovim/nvim-lspconfig'
Plug 'hrsh7th/nvim-cmp'
Plug 'ompugao/patto'

lua << EOF
  require('patto')
  vim.lsp.config('patto_lsp', {})
  vim.lsp.config('patto_preview', {})        -- browser preview (optional)
  vim.lsp.config('patto_preview_tui', {})    -- terminal preview live sync (optional)
  vim.lsp.enable({'patto_lsp', 'patto_preview', 'patto_preview_tui'})
EOF
```

Inline image preview: install [snacks.nvim](https://github.com/folke/snacks.nvim) with `image = { enabled = true }` and `[@img ...]` lines render in the buffer, with thumbnails for `[@embed ...]` links. See [docs/advanced-usage.md](./docs/advanced-usage.md#inline-images-in-neovim-snacksnvim).
</details>

<details>
<summary><b>Vim (vim-lsp)</b></summary>

```vim
Plug 'prabirshrestha/vim-lsp'
Plug 'ompugao/patto', {'for': 'patto'}
```
</details>

<details>
<summary><b>VS Code</b></summary>

Install from [VS Marketplace](https://marketplace.visualstudio.com/items?itemName=ompugao.patto-language-server)
</details>

### Usage

1. Create a `.pn` file
2. Type `[` for link completion, `@` for blocks
3. Use `:LspPattoTasks` to view pending tasks, `:LspPattoTasksReview` to review completed tasks (Vim/Neovim)

## Advanced

See **[docs/advanced-usage.md](./docs/advanced-usage.md)** for detailed documentation on:
- Task management (pending tasks, completion review, auto-tracking `completed_at`)
- Markdown import / export
- Zotero integration
- Terminal preview (`patto-preview-tui`) — keybindings, editor integration, image protocols
- Inline images in Neovim (snacks.nvim)
- Google Calendar sync

## Recent Updates

- **v0.7.2** - Patto Notes mobile app: select and copy text in the note viewer, including selections across many lines, and Select all covers the whole note; fix: the editor opens scrolled to the requested line; dependency updates
- **v0.7.1** - Neovim shows inline thumbnails for `[@img]` and `[@embed]` lines via snacks.nvim, with a fullscreen viewer on Enter; Patto Notes mobile app gets one quick-note launcher shortcut per workspace, home-screen pinning and the target workspace in the Inbox header; Markdown import converts `$...$`/`$$...$$` to patto math; fixes: block bodies indented under their line in patto output, patto-preview-tui keeps its place near code, image and math blocks when syncing with Neovim; CI caches cargo builds and release runners move to macOS 15; dependency updates
- **v0.7.0** - Patto Notes mobile app: Inbox tab with a chat-like composer, launcher shortcut and share target for quick posts; insert images, files and links from the device or clipboard into a per-workspace attachment folder; a conflicting sync pauses on a side branch and is resolved in-app with a three-way merge; editor shows the note structure, edits by block and places the caret with the trackpad; @embed cards for tweets, decks, link previews and Google Photos opened in-app. Preview and patto-preview-tui show Google Photos thumbnails for @embed share links; tweets are detected by host, so Dropbox links no longer render as tweets. Whole-codebase refactor into small modules with new parser, renderer, Lua and VS Code tests; fixes: parser error ranges in UTF-16 columns, back links under a canonical Windows root, renamed notes in the watcher, invalid anchor renames, note completion limited to the workspace, started_at/time_spent kept when writing patto, Markdown import keeps list items with their tasks; dependency updates
- **v0.6.0** - New Patto Notes mobile app (Flutter + Rust core): browse, edit and sync notes over git, several workspaces, tasks view, and full-text search across notes plus find-in-note; patto splits into a lean core with optional server/LSP features; preview gains a selectable bullet style for nested lines; fix image rendering in patto-preview-tui; Markdown import emits valid horizontal lines; dependency updates
- **v0.5.3** - Refine preview history navigation and roll up recent task/reliability improvements
- **v0.5.2** - Fix blockquote preview rendering (remove per-line border duplication); warn on done tasks with stale started_at field; fix elapsed time calculation for bare dates; dependency updates
- **v0.5.1** - Conceal URLs with titles in task labels; apply patto syntax highlighting and custom buffer names to Neovim Trouble buffers; dependency updates
- **v0.5.0** - patto-preview-tui reaches Neovim plugin parity with active task overlay, Tasks Review panel, due-date chips, status/time tracking, and total elapsed time; add paused task status and LSP folding ranges with opt-in Vim/Neovim integration; task syntax concealment improvements; dependency updates
- **v0.4.7** - task time tracking with transition detection; structured task info (due date, time_spent); fix preserve inline code in markdown table cells
- **v0.4.6** - org-mode-style task management: `experimental/mark_task_done` and `experimental/tasks_review` LSP commands, `scheduled`/`completed_at` task properties with auto-tracking, recency grouping and timeframe presets for tasks_review; dependency updates
- **v0.4.5** - allow comma in URL; fix self-links in repository graph causing RwLock deadlock; dependency updates
- **v0.4.4** - patto-preview-tui: flatten transparent image backgrounds before rendering, fix self-link anchor jumping; fix font size scaling (only `[** xxx]` and above enlarge)
- **v0.4.3** - patto-preview-tui: syntect syntax highlighting for code blocks, vim-like `/`/`?` search with cmdline editing, task view panel (`T`) grouped by deadline category
- **v0.4.2** - patto-preview-tui enhancements: math rendering via typst, text wrapping, TCP LSP backend, extensible editor integration with Neovim viewport sync, tui-widget-list for backlinks panel; PDF embedding via `@embed ./file.pdf`; `@embed`/`@img` syntax unification with LSP diagnostics (local paths now require `./` prefix)
- **v0.4.1** - Add TUI previewer
- **v0.4.0** - Rewrite the previewer, improving its latency and stability of real-time previewing
- **v0.3.1** - Add markdown import support, nested quotes, anchor renaming, and fix tab indentation handling
- **v0.3.0** - Complete Markdown export overhaul with 72 new tests
- **v0.2.10** - Bump nextjs
- **v0.2.9** - Minor fix
- **v0.2.8** - Zotero integration
- **v0.2.7** - Real-time preview without saving
- **v0.2.6** - Enhanced diagnostic messages, Improved neovim integration
- **v0.2.5** - Comprehensive tests for lsp server added
- **v0.2.4** - Lsp Renaming Support
- **v0.2.3** - Minor fix of vscode extension
- **v0.2.2** - VS Code extension, semantic highlighting
- **v0.2.0** - Repository system, LSP scanning, trouble.nvim integration

[Remaining Todos](./todo.md)
