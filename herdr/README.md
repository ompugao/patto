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

## Note folders

Set the folders that hold your notes with **⚙** in the panel (Add folder…, ✕ to remove), or by hand in
`~/.config/patto/herdr.json`:

```json
{ "roots": ["~/notes", "~/work/notes"], "inbox": "inbox.pn" }
```

- **Tasks** come from every listed folder, in every space, merged into one list. With more than one folder, each task
  shows which folder it is from.
- **Links** and the **preview** use the listed folder that holds the note open in nvim.
- **Capture** writes to `inbox` in a listed folder; with more than one, the dialog lets you choose. `inbox` is
  relative to the folder unless it starts with `/`.
- With no folders listed, the panel uses the current tab's folder (herdr-nvim's folder for the tab, else the git root
  of the focused terminal). Spaces on other machines have no such fallback.

Other optional keys: `query_bin`, `renderer_bin` (paths to `patto-query` and `patto-markdown-renderer`).

## Keys

With the panel focused: `j`/`k` move, `Enter` opens, `Space` moves the status on, `x` toggles done, `Tab` switches
Tasks/Links, `d` changes the done range, `c` captures, `p` opens the preview, `r` refreshes.

Global keys go in `~/.config/herdr-app/keys.json`, for example:

```json
"global": {
  "ctrl-alt-t": "patto.tasks",
  "ctrl-alt-l": "patto.links",
  "ctrl-alt-f": "patto.folders",
  "ctrl-alt-c": "patto.capture",
  "ctrl-alt-v": "patto.preview"
}
```

These reach the panel only while it is shown.

To show or hide the panel, bind `layout.toggle` to the panel's block. Herdr gives the block its own id in each
layout, so `install.sh` prints the exact lines, such as:

```json
"layout:main": { "ctrl-alt-p": "layout.toggle:b07404ab699265142911df87f10b74a13" }
```

## patto-query

The mod reads the notes through `patto-query`, which prints JSON and exits, because Herdr mods can only run short
commands, not hold a language server session:

```sh
patto-query tasks --root ~/notes --done this_week
patto-query links --root ~/notes ~/notes/project.pn
patto-query set-status --file ~/notes/project.pn --line 12 done
patto-query capture --file ~/notes/inbox.pn --task --due 2026-11-20 "write the report"
```
