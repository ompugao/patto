# Patto Note 🪽
A simple plain-text format for quick note-taking, outlining, and task management, powered by language server.

<img width="400" height="400" alt="patto_logo" src="https://github.com/user-attachments/assets/4dd09466-97af-46e9-badf-2fd793096de0" />

<!-- [![Ask DeepWiki](https://deepwiki.com/badge.svg)](https://deepwiki.com/ompugao/patto) -->

## Description
Patto Note is a text format inspired by [Cosense (formerly Scrapbox)](https://scrapbox.io), designed for quick note-taking, task management, and outlining.
It works with your favorite editor, powered by the [Language Server Protocol](https://microsoft.github.io/language-server-protocol/).
Unlike Markdown, every newline (\n) creates a new line, and a leading hard tab (\t) itemizes the line.
This simple, line-oriented structure makes it easy to outline ideas, organize tasks, and brainstorm effectively.

## Demo
![demo.gif](https://github.com/user-attachments/assets/8e1772dc-e4a7-4b22-bf83-c434e73726fb)


## Features
* Primary [Zettelkasten](https://zettelkasten.de/introduction/) support
* Real-time Preview support
* Task management with `line property` (please refer to the syntax section below)
* Integrated vim plugin
* Primary Language Server Protocol support
    * asynchronous workspace scanning with progress notifications
    * diagnostics
    * jumping between notes by go-to definition
    * backlinks by find-references with precise location tracking
    * 2-hop links
    * note/anchor completion
* Advanced task aggregation
    * deadline-based sorting (Overdue, Today, This Week, etc.)
    * [trouble.nvim](https://github.com/folke/trouble.nvim) integration for enhanced task viewing

## Syntax
```txt
Hello world.
	itemize lines with a leading hard tab `\t'
		that can be nested
	the second element  #sampleanchor
	the third element
	[@quote]
		quoted text must be indented with `\t'
	[@table caption="sample table"]
		header	column1	column2	column3	column4
		row1	item1	item2	item3	item4
		row2	item5	item6	item7	item8

Task Management
	a task {@task status=todo}
	another task with deadline {@task status=todo due=2030-12-31T23:59:00}
		abbreviated version of task !2030-12-31
	a completed task {@task status=done}

Decoration:
	[* bold text]
	[/ italic text]
	[*/ bold italic text]

Links:
	[other note]
		link to other note in a workspace
	[other note#anchor]
		direct link to an anchored line
	[#sampleanchor]
		self note link to the anchored line (i.e., this line) #sampleanchor
	url link:
		[https://google.com url title]
	title and url can be flipped:
		[url title https://google.com]
	link to an image
		[@img https://placehold.co/100.png "alt string"]

Code highlight with highlight.js
	[@code python]
		import numpy as np
		print(np.sum(10))
	[` inline code `]

Math with MathJax
	inline math: [$ O(n log(n)) $]
	[@math]
		O(n^2)\\
		sum_{i=0}^{10}{i} = 55
```

which is rendered as follows:  
<img width="906" height="2042" alt="screencapture-localhost-3031-2025-10-24-09_37_40" src="https://github.com/user-attachments/assets/60e18d5f-f92d-4a50-9a3e-f0c60fc0ba2b" />

### Line property
A text in the form of `{@XXX YYY=ZZZ}` is named as `line property` and adds an property to the line (not the whole text).
Currently, `anchor` and `task` properties are implemented:
* `{@anchor name}`: adds an anchor to the line. abbrev: `#name`
* `{@task status=todo due=2024-12-31}`: marks the line as a todo.  
  The due date only supports the ISO 8601 UTC formats (YYYY-MM-DD or YYYY-MM-DDThh:mm).  
  abbrev (symbols might be changed some time):
    * todo: `!2024-12-31`
    * doing: `*2024-12-31`
    * done: `-2024-12-31`

## Usage with (neo)vim
* First, open a file in a workspace with suffix `.pn`, or `:new` and `:set syntax=patto`
* Then, write your memos.
* Once you type `[` and `@`, lsp client will complete links and snippets respectively
	* snippets will only be completed with lsp-oriented snippet plugins such as [vim-vsnip](https://github.com/hrsh7th/vim-vsnip).
* You will have `:LspPattoTasks` command; that will gather tasks from the notes in your workspace and show them in a location window.

![demo_tasks aggregation](https://github.com/user-attachments/assets/e9945524-b430-496e-ae56-6a68bfd7c390)

* You will see 2-hop links of the current buffer with `:LspPattoTwoHopLinks` command (only in neovim, currently).

## Installation
### Install lsp server
Please download binaries from [GitHub release](https://github.com/ompugao/patto/releases)

If you use [jdx/mise](https://github.com/jdx/mise):
```sh
mise use -g github:ompugao/patto
mise use -g cargo:patto
```

or, use cargo:
```sh
cargo install patto
```

This will install the following utilities:
* `patto-lsp`: a lsp server
* `patto-preview`: a preview server for your patto notes
* `patto-markdown-renderer`: a format converter from patto note to markdown
* `patto-html-renderer`: a format converter from patto note to html

### Setup vim with vim-lsp (using vim-plug)
```vim
call plug#begin()
Plug 'prabirshrestha/asyncomplete.vim'
Plug 'prabirshrestha/asyncomplete-lsp.vim'
Plug 'prabirshrestha/vim-lsp'
Plug 'ompugao/patto', {'for': 'patto'}
call plug#end()
```
### Setup neovim with nvim-lspconfig (using vim-plug)
```vim
call plug#begin()
Plug 'neovim/nvim-lspconfig'
Plug 'hrsh7th/cmp-nvim-lsp'
Plug 'hrsh7th/nvim-cmp'
Plug 'ompugao/patto'
call plug#end()

lua << EOF
  require('patto')
  vim.lsp.config('patto_lsp', {}) -- for note management
  vim.lsp.config('patto_preview', {}) -- for preview server
  vim.lsp.enable({'patto_lsp', 'patto_preview'})
EOF
```

Note: we recommend neovim@nightly for non-ascii notes since PositionEncoding UTF-16 support has a bug in the current neovim stable v0.10.3. see https://github.com/neovim/neovim/issues/32105.

### Customization

* `g:patto_enable_open_browser`: Set to `1` to enable automatic browser opening for the preview server (default: disabled)

#### Zotero integration (experimental)
`patto-lsp` can suggest entries from your Zotero library while completing `[` links when it is built with the `zotero` cargo feature (e.g. `cargo install patto --features zotero`).

1. Create a config file at `$XDG_CONFIG_HOME/patto/patto-lsp.toml` (defaults to `~/.config/patto/patto-lsp.toml`).
2. Provide your credentials:

```toml
[zotero]
user_id = "1234567"
api_key = "zotero_api_key"
endpoint = "http://127.0.0.1:23119/api" # optional: talk to local Zotero desktop
```

(Uppercase keys such as `ZOTERO_USER_ID`/`ZOTERO_API_KEY`/`ZOTERO_ENDPOINT` are also accepted.)

When configured, patto-lsp will log the Zotero connection status on startup and include matching papers as completion candidates in the form `paper title zotero://select/library/items/<ITEM_ID>`. Use the `endpoint` setting to target the Zotero desktop application's local API (default remains the public `https://api.zotero.org`).

To keep completions fast even when the Zotero API is slow, patto-lsp periodically refreshes and stores the full paper list at `$XDG_CACHE_HOME/patto/paper-catalog.json` (or the platform-specific equivalent) and serves completion items from that cache.

#### Optional: Integration with trouble.nvim
For enhanced task viewing with deadline sorting and categorization:
```vim
Plug 'folke/trouble.nvim'
```

After installing trouble.nvim, you can use:
```vim
:Trouble patto_tasks
```
This will display tasks organized by deadline categories (Overdue, Today, This Week, etc.) with automatic sorting.

### Setup vscode extension
Released from v0.2.2. You can install from [HERE](https://marketplace.visualstudio.com/items?itemName=ompugao.patto-language-server), supporting content preview and task management.

<img width="752" height="524" alt="image" src="https://github.com/user-attachments/assets/320d8f00-dd03-45e9-b58b-c5a900c25a3a" />

## Recent Updates
### v0.2.10
- Bump nextjs
- Minor Update

### v0.2.9
- Minor update

### v0.2.8
- **Zotero Integration**: Experimental Zotero integration is supported.

### v0.2.7
- **Improved Realtime Preview**: No need to save file for preview by sending content from editor to previewer via lsp.

### v0.2.6
- **Enhanced diagnostic messages**: Human-readable error messages including examples and helpful hints for common parsing
    errors
- **Improved Neovim integration**: Enhanced trouble.nvim support with better task view formatting
- Bugfix: the parser hangs when insufficiently indented lines exist after table command

### v0.2.5
* **Test Lsp features**: Add comprehensive tests for lsp server.

### v0.2.4
* **Lsp Renaming**: Add support for renaming notes
* Add tests for LSP server

### v0.2.3
* Minor fix of vscode extension

### v0.2.2
* **Semantic Tokens**: patto-lsp now offers semantic highlighting.

### v0.2.0
* **Repository System**: New centralized repository management for improved performance and scalability
* **Link Location Tracking**: Backlinks now include precise line and column locations
* **Workspace Scanning**: Asynchronous scanning with progress notifications via LSP
* **Task Management**: Advanced deadline-based sorting with trouble.nvim integration
* **Table Support**: New `[@table]` block element for structured data
* **Preview Enhancements**: WebSocket-based updates for backlinks and 2-hop links, file search in sidebar
* **Rendering Improvements**: Better image and PDF support in preview, configurable hard line breaks in Markdown export

## Upcoming features:
please refer to [todo](./todo.md)

## FAQ
### Why not Markdown?
The differences in behavior between Markdown parsers led me to create the Patto format.
For example, in GitHub Markdown, code fences can be contained within a list item, whereas in Obsidian, they cannot (ref [Code Block Indentation](https://forum.obsidian.md/t/code-block-indentation/43966)):

- item
- ```python
  print('hello')
  ```
- item3

### Custom template?
Please use your favorite template/snippet engine of your editor. I personally use [LuaSnip](https://github.com/L3MON4D3/LuaSnip) in neovim.  
Other candidate vim plugins:
- https://github.com/mattn/vim-sonictemplate
- https://github.com/hrsh7th/vim-vsnip
- https://github.com/echasnovski/mini.snippets

## Misc
## unix command utilities
### sort tasks with grep and sort
```sh
rg --vimgrep '.*@task.*todo' . | awk '{match($0, /due=([0-9:\-T]+)/, m); if (RLENGTH>0) print m[1], $0; else print "9999-99-99", $0}' |sort |cut -d' ' -f2-
# or, in vim
cgetexpr system('rg --vimgrep ".*@task.*todo" . | awk "{match(\$0, /due=([0-9T:\-]+)/, m); if (RLENGTH>0) print m[1], \$0; else print \"9999-99-99\", \$0}" |sort|cut -d" " -f2-')|copen

```
