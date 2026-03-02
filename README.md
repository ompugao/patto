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
    Anchored text  #anchor

[* bold]  [/ italic]  [` code `]
```

### Links & Tasks
```txt
[other note]                     Link to note
[note#anchor]                    Link to the anchored line in note
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

### Content embeddings
```txt
[@img http://example.com/img "image alt"]
[@embed https://www.youtube.com/watch?v=dQw4w9WgXcQ Youtube Alt]
[@embed https://twitter.com/... Tweet]
[@embed https://speakerdeck.com/... Slide]
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

See **[docs/advanced-usage.md](./docs/advanced-usage.md)** for detailed documentation on:
- Task management
- Markdown import / export
- Zotero integration
- Terminal preview (`patto-preview-tui`) — keybindings, editor integration, image protocols
- Google Calendar sync

## Recent Updates

**v0.4.1** - Add TUI previewer
**v0.4.0** - Rewrite the previewer, improving its latency and stability of real-time previewing
**v0.3.1** - Add markdown import support, nested quotes, anchor renaming, and fix tab indentation handling
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
