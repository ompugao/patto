# 🐙 Patto Note
Yet another plain text format for quick note taking and task management.

## Description
Patto Note is a [Cosense (formerly Scrapbox)](https://scrapbox.io)-inspired text format.
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

Upcoming features:
### parser
* [ ] link to localfile

### lsp
* [ ] semantic tokens
* [ ] document backlinks using find references
### renderer
* [ ] markdown export
* [ ] math expression rendering

### Using cargo
```sh
cargo install patto
```

## Syntax
TBD

## Misc
## unix command utilities
### sort tasks with grep and sort
```sh
rg --vimgrep '.*@task.*todo' . | awk '{match($0, /due=([0-9:\-T]+)/, m); if (RLENGTH>0) print m[1], $0; else print "9999-99-99", $0}' |sort |cut -d' ' -f2-
# or, in vim
cgetexpr system('rg --vimgrep ".*@task.*todo" . | awk "{match(\$0, /due=([0-9T:\-]+)/, m); if (RLENGTH>0) print m[1], \$0; else print \"9999-99-99\", \$0}" |sort|cut -d" " -f2-')|copen

```
