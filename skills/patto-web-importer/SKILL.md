---
name: patto-web-importer
description: Fetches content from a URL and converts it into the Patto Note format. Use this skill when the user asks to import, fetch, or copy content from the web into a Patto buffer or document.
argument-hint: "<URL> [OUTFILE]"
allowed-tools: WebFetch, Bash(patto-syntax-checker *)
---

# Patto Web Importer

This skill fetches content from the web and converts it into the Patto Note format. It ensures the output is syntactically valid by using a dedicated checker tool.

## Workflow

1. **Fetch Content**: Use the `web_fetch` tool to retrieve the content from the provided URL.
2. **Convert to Patto**:
   - Read `references/patto-syntax.md` for the syntax rules.
   - Convert the fetched content (HTML/Text/Markdown) into Patto Note format.
   - **Important Syntax Note**: When using task symbols (`!`, `*`, `-` with dates), ensure they are placed at the **end** of the line if there is a description, or use the `{@task ...}` property.
   - If an output file name is provided and the content contains images that are relevant to the main content, fetch and save them to `assets` directory. Use `[@img ./assets/{filename}]` to embed the image.
3. **Validate Syntax**:
   - Save the converted content to a temporary file (e.g., `tmp.pn`).
   - Run the syntax checker: `patto-syntax-checker tmp.pn`.
   - If the checker reports errors, analyze them and fix the Patto output. Repeat until valid.
4. **Output**: Return the validated Patto Note content. If an output file name is provided, write the content to the specified file as well.

## References
- See [patto-syntax.md](references/patto-syntax.md) for the full syntax specification.
