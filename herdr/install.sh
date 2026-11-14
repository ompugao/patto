#!/bin/sh
# Install the patto mod into the Herdr app.
# Files are copied, not linked: Herdr does not reload a mod whose folder is a symlink.
set -eu

here=$(cd "$(dirname "$0")" && pwd)
app_home=${HERDR_APP_HOME:-$HOME/.config/herdr-app}
target="$app_home/mods/patto"

if ! command -v patto-query >/dev/null 2>&1 && [ ! -x "$HOME/.cargo/bin/patto-query" ]; then
  echo "Installing patto-query with cargo..."
  cargo install --locked --path "$here/.." --bin patto-query --bin patto-markdown-renderer
fi

if [ -L "$target" ]; then
  rm "$target"
fi
mkdir -p "$target"
for file in "$here"/mods/patto/*.js "$here"/mods/patto/mod.json; do
  cp "$file" "$target/"
done

if command -v herdr-app >/dev/null 2>&1; then
  herdr-app check "$target"
fi

# The panel's block id is made per layout when Herdr docks the mod, so the
# toggle key is printed from the layouts as they are now.
toggle_keys() {
  python3 - "$app_home/layouts" <<'PY'
import json, pathlib, sys
def blocks(node):
    if isinstance(node, dict):
        content = node.get("content")
        if node.get("type") == "block" and isinstance(content, dict) and content.get("mod") == "patto":
            yield node["id"]
        for value in node.values():
            yield from blocks(value)
    elif isinstance(node, list):
        for value in node:
            yield from blocks(value)
for path in sorted(pathlib.Path(sys.argv[1]).glob("*.json")):
    try:
        ids = list(blocks(json.loads(path.read_text())))
    except ValueError:
        continue
    for block_id in ids:
        print(f'  "layout:{path.stem}": {{ "ctrl-alt-p": "layout.toggle:{block_id}" }},')
PY
}

cat <<'KEYS'
Installed. Suggested keys for ~/.config/herdr-app/keys.json:
  "global": {
    "ctrl-alt-t": "patto.tasks",
    "ctrl-alt-l": "patto.links",
    "ctrl-alt-f": "patto.folders",
    "ctrl-alt-c": "patto.capture",
    "ctrl-alt-m": "patto.menu",
    "ctrl-alt-v": "patto.preview"
  },
Show or hide the panel (one line per layout that has it):
KEYS
keys=$(toggle_keys 2>/dev/null || true)
if [ -n "$keys" ]; then
  printf '%s\n' "$keys"
else
  echo "  (Herdr has not docked the panel yet; run this script again once it shows.)"
fi
