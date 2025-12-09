#!/bin/bash
cwd=$(pwd)
tmpdir=$(mktemp -d --suffix patto_demo)
cd $tmpdir
echo $tmpdir
git init
cat > project1.pn <<EOF
Project1
You can record tasks whereever you want.
	task 1 of project 1        {@task status=todo due=2025-12-08}
	task 2 of project 1        {@task status=todo due=2025-12-12}

Due with time:
	task 3 of project 1        {@task status=todo due=2025-12-20T11:59}
	finished task of project 1 {@task status=done due=2025-12-01T10:00}
EOF
cat > project2.pn <<EOF
Here is the note for project2.

Some tasks:
	abbrev task in project 2    !2025-12-09
	wip task in project 2       *2025-12-11T12:00
	finished task in project 2  -2025-12-09
EOF
rm -f ~/.nvimswap/project*.pn.*
cat > demo.tape << EOF
Set FontSize 26
Set Width 1400
Set Height 800
Type "# patto-lsp can aggregate tasks in a workspace."
Enter
Sleep 1s
Type "ls *.pn"
Enter
Sleep 2s
Type "clear"
Sleep 1s
Enter
Type "nvim project1.pn project2.pn"
Sleep 0.5s
Enter
Sleep 5s
Type ";bnext"
Sleep 1s
Enter
Sleep 0.5s
Enter
Sleep 5s
Type ";qall"
Enter
Sleep 1s
Type "nvim note.pn"
Sleep 1s
Enter
Sleep 1s
Type ";LspPattoTasks"
Sleep 1s
Enter
Sleep 1s
Enter
Type "V"
Sleep 2s
Escape
Escape
Type ";You can jump to tasks from location window"
Sleep 2s
Escape
Escape
Type ";lopen"
Sleep 1s
Enter
Sleep 1s
Type "jj"
Sleep 1s
Enter
Sleep 2s
Type ";Another jump to task"
Sleep 3s
Escape
Escape
Type "f*"
Sleep 2s
Type "r-"
Type ";w"
Enter
Sleep 3s
Type ";LspPattoTasks"
Enter
Sleep 1s
Type ";Completed tasks are not shown"
Sleep 2s
Escape
Escape
Type ";q"
Enter
Type ";Trouble patto_tasks"
Sleep 0.5s
Enter
Type ";Integration with trouble.nvim is included as well"
Sleep 5s
Escape
Escape
Type ";qa"
Enter
EOF
vhs demo.tape -o out_tasks.gif
cp out_tasks.gif $cwd/
cd $cwd
rm -rf $tmpdir

