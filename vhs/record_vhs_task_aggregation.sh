#!/bin/bash
cat > project1.pn <<EOF
Project1
You can record tasks whereever you want.
	re-write design doc    {@task status=todo due=2025-04-01}
	task2                  {@task status=todo due=2025-04-02}

Due with time:
	re-organize code       {@task status=todo due=2025-05-15T11:59}
	done                   {@task status=done due=2025-06-15T10:00}
EOF
cat > project2.pn <<EOF
Here is the note for project2.

Some tasks:
	abbrev todo  !2025-04-20
	wip          *2025-05-01T12:00
	done         -2025-02-01
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
Type "cat project1.pn"
Sleep 0.5s
Enter
Sleep 5s
Type "clear"
Sleep 1s
Enter
Type "cat project2.pn"
Sleep 0.5s
Enter
Sleep 5s
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
Type "f!"
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
Type ";qa"
Sleep 1s
Enter
EOF
vhs demo.tape
rm *.pn demo.tape

