# 🐙 Patto Note
A simple, language server-powered plain-text format for quick note-taking, outlining, and task management.

## Description
Patto Note is a text format inspired by [Cosense (formerly Scrapbox)](https://scrapbox.io), designed for quick note-taking, task management, and outlining.
It works with your favorite editor, powered by the [Language Server Protocol](https://microsoft.github.io/language-server-protocol/).
Unlike Markdown, every newline (\n) creates a new line, and a leading hard tab (\t) itemizes the line.
This simple, line-oriented structure makes it easy to outline ideas, organize tasks, and brainstorm effectively.

## Features
* Primary [Zettelkasten](https://zettelkasten.de/introduction/) support
* Task management support by `line property` (please refer to the syntax section below)
* Integrated vim plugin
* Language server protocol
    * asynchronous workspace scanning
    * diagnostics
    * jumping between notes by go-to definition
    * note/anchor completion

## Installation
### Install lsp server using cargo
```sh
cargo install patto
```
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
### Setup vscode extension
To be released

## Syntax
```txt
Hello world.
	itemize lines with a leading hard tab `\t'
		that can be nested
	the second element  #sampleanchor
	the third element
	[@quote]
		quoted text must be indented with '\t'

Task Management
	a task {@task status=todo}
	another task with deadline {@task status=todo due=2030-12-31T23:59:00}
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
		self note link to the anchored line
	url link:
		[https://google.com url title]
	title and url can be flipped:
		[url title https://google.com]
	link to an image
		[@img https://via.placeholder.com/50 width=300 height=300]

Code highlight with highlight.js
	[@code python]
		import numpy as np
		print(np.sum(10))
	[` inline code `]

Math with katex
	[@math]
		O(n^2)\\
		sum_{i=0}^{10}{i} = 55
	inline math: [$ O(n log(n)) $]
```

### Line property
A text in the form of `{@XXX YYY=ZZZ}` is named as `line property` and adds an property to the line (not the whole text).
Currently, `anchor` and `task` properties are implemented:
* `{@anchor name}`: adds an anchor to the line. abbrev: `#name`
* `{@task status=todo due=2024-12-31}`: marks the line as a todo. The due date only supports the ISO 8601 UTC formats (YYYY-MM-DD or YYYY-MM-DDThh:mm). Its abbreviation is coming soon (TBD).

## Upcoming features:
### parser
* [ ] link to localfile

### lsp
* [ ] semantic tokens
* [ ] document backlinks using find references
### renderer
* [x] markdown export
* [ ] math expression rendering

## Misc
## unix command utilities
### sort tasks with grep and sort
```sh
rg --vimgrep '.*@task.*todo' . | awk '{match($0, /due=([0-9:\-T]+)/, m); if (RLENGTH>0) print m[1], $0; else print "9999-99-99", $0}' |sort |cut -d' ' -f2-
# or, in vim
cgetexpr system('rg --vimgrep ".*@task.*todo" . | awk "{match(\$0, /due=([0-9T:\-]+)/, m); if (RLENGTH>0) print m[1], \$0; else print \"9999-99-99\", \$0}" |sort|cut -d" " -f2-')|copen

```
