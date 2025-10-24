# 🐙 Patto Note
A simple plain-text format for quick note-taking, outlining, and task management, powered by language server.

## Description
Patto Note is a text format inspired by [Cosense (formerly Scrapbox)](https://scrapbox.io), designed for quick note-taking, task management, and outlining.
It works with your favorite editor, powered by the [Language Server Protocol](https://microsoft.github.io/language-server-protocol/).
Unlike Markdown, every newline (\n) creates a new line, and a leading hard tab (\t) itemizes the line.
This simple, line-oriented structure makes it easy to outline ideas, organize tasks, and brainstorm effectively.

## Demo
![demo.gif](https://github.com/user-attachments/assets/a1f1dcb4-e1b2-4fff-91de-e587009f2dae)

## Features
* Primary [Zettelkasten](https://zettelkasten.de/introduction/) support
* Real-time Preview support
* Task management with `line property` (please refer to the syntax section below)
* Integrated vim plugin
* Primary Language Server Protocol support
    * asynchronous workspace scanning
    * diagnostics
    * jumping between notes by go-to definition
    * backlinks by find-references
    * 2-hop links
    * note/anchor completion

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

![task aggregation demo](https://github.com/user-attachments/assets/7d05ffdd-0ccd-4fb7-9f5c-d90491a7cb88)

* You will see 2-hop links of the current buffer with `:LspPattoTwoHopLinks` command (only in neovim, currently).

## Installation
### Install lsp server
Please download binaries from [GitHub release](https://github.com/ompugao/patto/releases)

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
  require('lspconfig.configs').patto_lsp.setup({})
EOF
```
Note: we recommend neovim@nightly for non-ascii notes since PositionEncoding UTF-16 support has a bug in the current neovim stable v0.10.3. see https://github.com/neovim/neovim/issues/32105.

### Setup vscode extension
To be released.

## Upcoming features:
### parser
* [x] link to local files

### lsp
* [x] document backlinks using find references
    * [ ] file renaming keeping note connections
    * [ ] anchor renaming
* [x] 2-hop links
* [ ] semantic tokens

### renderer
* [x] markdown export
* [x] math expression rendering
* [x] replace highlight.js with syntect

### other todos
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
