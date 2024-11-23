# 🐙 Patto Note
Yet another plain text format for quick note taking and task management.

## Description
Patto Note is a [Cosense (formerly Scrapbox)](https://scrapbox.io)-inspired text format.
This enables quick note taking and task management in your favorite editor powered by [Language Server Protocol](https://microsoft.github.io/language-server-protocol/).
Unlike markdown format, newline "\n" literally create a new line, and a leading (hard) tab "\t" itemize the line.

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
### Setup vim with vim-lsp
```vim
Plug 'ompugao/patto', {'for': 'patto'}
```
### Setup vscode extension
To be released

## Syntax
```txt
Hello world.
	itemize lines with a leading hard tab `\t'
		that can be nested
	the second element
	the third element
	[@quote]
		quoted text must be indented with '\t'

Decoration:
	[* bold text]
	[/ italic text]
	[*/ bold italic text]

Links:
	[wikilink]
	[https://google.com url title]
	[url title https://google.com]
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


## Upcoming features:
### parser
* [ ] link to localfile

### lsp
* [ ] semantic tokens
* [ ] document backlinks using find references
### renderer
* [ ] markdown export
* [ ] math expression rendering

## Misc
## unix command utilities
### sort tasks with grep and sort
```sh
rg --vimgrep '.*@task.*todo' . | awk '{match($0, /due=([0-9:\-T]+)/, m); if (RLENGTH>0) print m[1], $0; else print "9999-99-99", $0}' |sort |cut -d' ' -f2-
# or, in vim
cgetexpr system('rg --vimgrep ".*@task.*todo" . | awk "{match(\$0, /due=([0-9T:\-]+)/, m); if (RLENGTH>0) print m[1], \$0; else print \"9999-99-99\", \$0}" |sort|cut -d" " -f2-')|copen

```
