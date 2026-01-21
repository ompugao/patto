# Patto Note 🪽

A simple plain-text format for note-taking, outlining, and task management. Inspired by [Cosense/Scrapbox](https://scrapbox.io), powered by LSP.

<img width="200" height="200" alt="patto_logo" src="https://github.com/user-attachments/assets/4dd09466-97af-46e9-badf-2fd793096de0" />

![demo.gif](https://github.com/user-attachments/assets/8e1772dc-e4a7-4b22-bf83-c434e73726fb)

## What is Patto?

A **line-oriented** text format where newlines create lines and tabs create nesting. Perfect for:
- 📝 Quick outlining and note-taking
- ✅ Task management with deadlines
- 🔗 [Zettelkasten](https://zettelkasten.de/introduction/)-style linked notes

## Features

- Wiki-style links `[note name]` with backlinks and 2-hop visualization
- Tasks with deadlines: `!2024-12-31` or `{@task due=2024-12-31}` (sorted by Overdue, Today, This Week)
- Real-time preview and LSP-powered autocomplete
- Works with Vim, Neovim, VS Code

## Syntax

### Basic
```txt
Plain text
	Tab to nest
		Tab twice for deeper nesting

[* bold]  [/ italic]  [` code `]
```

### Links & Tasks
```txt
[other note]                     Link to note
[note#anchor]                    Link to section
[https://example.com Title]     External link

!2024-12-31    Todo with deadline
*2024-12-31    In progress
-2024-12-31    Done
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
  vim.lsp.config('patto_preview', {})
  vim.lsp.enable({'patto_lsp', 'patto_preview'})
EOF
```
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
3. Use `:LspPattoTasks` to view all tasks (Vim/Neovim)

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

## Recent Updates

**v0.3.0** - Complete Markdown export overhaul with 72 new tests  
**v0.2.10** - Bump nextjs
**v0.2.9** - Minor fix
**v0.2.8** - Zotero integration  
**v0.2.7** - Real-time preview without saving  
**v0.2.6** - Enhanced diagnostic messages, Improved neovim integration

**v0.2.5** - Comprehensive tests for lsp server added

**v0.2.4** - Lsp Renaming Support
**v0.2.3** - Minor fix of vscode extension
**v0.2.2** - VS Code extension, semantic highlighting  
**v0.2.0** - Repository system, LSP scanning, trouble.nvim integration

[Remaining Todos](./todo.md)
