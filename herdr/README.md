# patto for the Herdr app

A Herdr app mod that keeps patto tasks and links beside your terminals and nvim.

- **Tasks**: open tasks across the notes folder, grouped Overdue / Today / Tomorrow / This Week / This Month / Later / No Deadline,
  plus done tasks for today or this week. Click a task to open it in nvim. Click its status mark to move it on
  (todo → doing → done); `completed_at`, `started_at` and `time_spent` are recorded as the language server does.
- **Links**: back links (with the linking line) and two-hop links of the note open in the tab's nvim.
- **Capture** (`+`): add a task, with an optional due date, or a plain line to an inbox note.
- **Preview**: a pane beside the tab that renders the current note.

## Install

```sh
herdr/install.sh
```

It installs `patto-query` with cargo when it is missing, copies the mod to `~/.config/herdr-app/mods/patto`
and checks it. Run it again after pulling changes.

Following the note open in nvim needs the herdr-nvim mod, which keeps one nvim per tab.
Without it, tasks still work and links open in a new nvim terminal.

## Notes folder

The first match wins:

1. `root` in `~/.config/patto/herdr.json`
2. the folder herdr-nvim uses for the tab
3. the git root of the focused terminal's folder

`~/.config/patto/herdr.json` (all keys optional):

```json
{ "root": "~/notes", "inbox": "inbox.pn", "query_bin": "patto-query", "renderer_bin": "patto-markdown-renderer" }
```

`inbox` is relative to the notes folder unless it starts with `/`.

The panel shows only while that folder has `.pn` files (tracked or untracked in git, or within three levels
outside git). Elsewhere it shrinks to zero width and comes back at its last width.

## Keys

With the panel focused: `j`/`k` move, `Enter` opens, `Space` moves the status on, `x` toggles done, `Tab` switches
Tasks/Links, `d` changes the done range, `c` captures, `p` opens the preview, `r` refreshes.

Global keys go in `~/.config/herdr-app/keys.json`, for example:

```json
"ctrl-alt-t": "patto.tasks",
"ctrl-alt-l": "patto.links",
"ctrl-alt-c": "patto.capture",
"ctrl-alt-p": "patto.preview"
```

These reach the panel only while it is shown.

## patto-query

The mod reads the notes through `patto-query`, which prints JSON and exits, because Herdr mods can only run short
commands, not hold a language server session:

```sh
patto-query tasks --root ~/notes --done this_week
patto-query links --root ~/notes ~/notes/project.pn
patto-query set-status --file ~/notes/project.pn --line 12 done
patto-query capture --file ~/notes/inbox.pn --task --due 2026-11-20 "write the report"
```
