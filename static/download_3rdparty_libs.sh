#!/bin/bash
cd $(git rev-parse --show-toplevel)
echo -n 'Downloading idiomorph... '
wget -q https://raw.githubusercontent.com/bigskysoftware/idiomorph/refs/heads/main/dist/idiomorph.min.js -O ./static/js/idiomorph.min.js
echo 'done'
echo -n 'Downloading mermaid.js... '
wget -q https://unpkg.com/mermaid/dist/mermaid.min.js -O ./static/js/mermaid.min.js
echo 'done'
