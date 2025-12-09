#!/bin/bash
cwd=$(pwd)
tmpdir=$(mktemp -d --suffix patto_demo)
cd $tmpdir
echo $tmpdir
git init
cat > another\ note.pn <<EOF
Here is another note.
jump back from here: [note].

Here is the anchored line. #anchor
	a #a1
	b #a2
	c #a3
EOF
rm -f ~/.nvimswap/note.pn.*
rm -f note.pn
touch note2.pn note3.pn note4.pn
cat > demo.tape << EOF
Set FontSize 26
Set Width 1200
Set Height 600
Type "nvim"
Sleep 0.5s
Enter
Type ";w note.pn"
Sleep 0.5s
Enter
Sleep 0.1s
Type "iThis is a demo of Patto note."
Enter
Tab@500ms 1
Type "Patto note uses a hard tab (\t) to itemize lines,"
Enter
Type "like"
Enter
Type "this."
Enter
Tab@500ms 1
Type "and can be nested to create a hierarchy."
Enter
Sleep 0.5s
Ctrl+d
Ctrl+d
Enter
Type "Patto note has a primary Zettelkasten support."
Enter
Escape
Type ";w"
Enter
Type "i"
Tab@100ms 1
Type "link to [another note]"
Escape
Left 2
Type "gd"
Sleep 3s
Type "j$"
Left 2
Sleep 0.5s
Type "gd"
Escape

Type "G"
Enter
Sleep 1s
Type "o"
Ctrl+d
Type "and has lsp-powered completion."
Enter
Tab@100ms 1
Type "["
Sleep 1s
Type "ano"
Sleep 0.5s
Ctrl+n
Enter
Type "#"
Sleep 1s
Type "an"
Ctrl+n
Enter
Sleep 1s
Type "]"
Sleep 1s
Escape
Left 2
Type "gd"
Sleep 3s
Ctrl+o
Type "Go"
Ctrl+d
Enter
Type "Please refer to README.md for more information. Thanks!"
Enter
Escape
Type ";wqa"
Sleep 500ms
Ctrl+D
EOF
vhs demo.tape
cp out.gif $cwd/
cd $cwd
rm -rf $tmpdir
