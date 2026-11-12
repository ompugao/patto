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

cat <<'KEYS'
Installed. Suggested keys for ~/.config/herdr-app/keys.json:
  "ctrl-alt-t": "patto.tasks",
  "ctrl-alt-l": "patto.links",
  "ctrl-alt-f": "patto.folders",
  "ctrl-alt-c": "patto.capture",
  "ctrl-alt-p": "patto.preview"
KEYS
