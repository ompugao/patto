# Patto Note - Copilot Instructions

## Project Overview

Patto is a plain-text note format (`.pn` files) with LSP support, task management, and wiki-style links. It ships as:
- A Rust library + multiple CLI binaries (`patto-lsp`, `patto-preview`, `patto-markdown-renderer`, `patto-markdown-importer`, `patto-html-renderer`)
- A VS Code extension (TypeScript, Webpack)
- Vim/Neovim plugin (Lua)
- A React/Vite preview UI embedded into the `patto-preview` binary via `rust-embed`

## Build, Test & Lint

### Rust (primary codebase)
```sh
# Build (automatically triggers patto-preview-ui build via build.rs)
cargo build

# Running tests
cargo test                              # All tests (runs on stable)
cargo test <test_name>                  # Single test by name (substring match)
cargo test <test_name> -- --nocapture   # Single test with output
cargo test --test lsp_completion        # Specific test file (e.g. tests/lsp_completion.rs)
cargo test --features preview-tui       # Tests with TUI feature enabled

# Code quality
cargo fmt --all                         # Format code
cargo clippy --all-targets --all-features  # Lint (as run in CI)

# Feature-gated builds
cargo build --features preview-tui              # TUI preview (no chafa image support)
cargo build --features preview-tui-chafa-dyn    # TUI with chafa dynamically linked
cargo build --features preview-tui-chafa-static # TUI with chafa statically linked (Linux only)
cargo build --no-default-features              # Without Zotero integration
```

**Note:** CI runs tests and builds with `--features preview-tui` on all platforms (ubuntu, macos, windows). When adding platform-specific code, verify it builds on all three.

### VS Code Extension
```sh
pnpm install              # From repo root (installs root + client/)
npm run compile           # TypeScript compile
npm run build             # Webpack bundle → dist/extension.js
npm run lint              # ESLint on client/
npm run watch             # Watch mode for development
```

### Preview UI (React/Vite, embedded in Rust binary)
```sh
cd patto-preview-ui
npm install
npm run build             # Output → patto-preview-ui/dist/ (picked up by rust-embed at cargo build time)
npm run dev               # Dev server (standalone, not embedded)
```

> **Important:** `cargo build` runs `build.rs` which automatically runs `npm install && npm run build` in `patto-preview-ui/` whenever `patto-preview-ui/src/` changes. The `patto-preview-ui/dist/` directory is embedded into the `patto-preview` binary at compile time via `rust-embed`. If frontend changes don't appear: check `build.rs` rerun conditions, or manually run `cd patto-preview-ui && npm run build` then `cargo build` again.

## Architecture

```
src/
  patto.pest          # PEG grammar (pest) — the source of truth for syntax
  parser.rs           # Parses .pn lines into AstNode trees using pest
  repository.rs       # Watches a directory of .pn files; builds a backlink/2-hop graph (gdsl)
  lsp/
    backend.rs        # tower-lsp Backend; implements LanguageServer trait
    lsp_config.rs     # LSP server initialization config
    paper.rs          # Zotero paper catalog integration
  renderer.rs         # Renders AstNode trees to HTML (used by preview server)
  markdown/           # Markdown export (MarkdownRenderer) with flavor support
  importer/           # Markdown → Patto importer
  diagnostic_translator.rs  # Translates pest errors to LSP diagnostics
  semantic_token.rs   # LSP semantic token provider
  line_tracker.rs     # Maps line numbers to rope positions (ropey)
  bin/
    patto-lsp.rs      # LSP server binary (stdin/stdout or TCP)
    patto-preview.rs  # Preview HTTP server (axum) + WebSocket + embedded Vite UI
    patto-markdown-renderer.rs
    patto-markdown-importer.rs
    patto-html-renderer.rs

tests/
  common/
    in_process_client.rs  # Directly instantiates Backend; avoids spawning a process
    workspace.rs          # Creates temp dirs with .pn files for tests
  lsp_*.rs              # Integration tests for each LSP feature
  markdown_*.rs         # Markdown export/import tests
```

### Key data flows

1. **LSP**: Editor ↔ `patto-lsp` binary ↔ `tower-lsp` ↔ `Backend` ↔ `Repository` (in-memory graph)
2. **Preview**: Browser ↔ WebSocket (`/ws`) ↔ `patto-preview` axum server ↔ `Repository` → HTML via `Renderer`; the frontend (React) is served from embedded `patto-preview-ui/dist/`
3. **Parser pipeline**: raw line → `PattoLineParser` (pest) → `AstNode` → consumed by LSP, renderer, or markdown exporter

### Repository & graph

`Repository` in `src/repository.rs` maintains:
- A `DashMap` of file URL → parsed AST + metadata
- A `gdsl` directed graph of wiki-link edges between documents (used for backlinks and 2-hop links)
- A `notify` file watcher that re-parses files on change and sends updates via a broadcast channel

## Key Conventions

- **Grammar first**: All syntax changes start in `src/patto.pest`. The pest grammar is the canonical definition of the format.
- **Async runtime**: The LSP backend and preview server both use `tokio`. Tests use `#[tokio::test]`.
- **UTF-16 column offsets**: LSP positions use UTF-16 column offsets (`str_indices::utf16` helpers). Use `utf16_from_byte_idx` / `utf16_to_byte_idx` when converting between byte indices and LSP positions.
- **`zotero` feature**: Enabled by default (`features = ["zotero"]` in Cargo.toml). Build without it via `cargo build --no-default-features`.
- **Test pattern**: Integration tests in `tests/` use `InProcessLspClient` (no subprocess), which wraps `Backend` directly. Use `TestWorkspace` to create temp directories with fixture `.pn` files.
- **Serde tags**: WebSocket messages use `#[serde(tag = "type", content = "data")]` — the frontend expects `{ type: "...", data: { ... } }`.
- **VS Code extension entry**: `client/src/extension.ts` — spawns `patto-lsp` and `patto-preview` as child processes.
- **`patto-preview-next/`**: A legacy Next.js preview (superseded by the Vite UI in `patto-preview-ui/`). Not embedded in the binary.
- **`patto-preview-tui`**: Terminal UI preview binary with three feature tiers: `preview-tui` (no chafa), `preview-tui-chafa-dyn` (chafa via dynamic linking, requires libchafa on system), `preview-tui-chafa-static` (chafa statically bundled, Linux only — used for release builds). The `chafa-dyn`/`chafa-static` features of `ratatui-image` are mutually exclusive; both are opted out by default via `default-features = false`. `preview-tui-chafa-static` activates static chafa via `patto-chafa-bridge`, a code-free crate that exists purely for Cargo feature unification.
- **LSP custom commands**: Backend exposes `experimental/aggregate_tasks`, `experimental/retrieve_two_hop_notes`, and `experimental/scan_workspace` via `workspace/executeCommand`. Editors call these to show task lists and 2-hop note graphs.
- **Markdown flavors**: `MarkdownFlavor` has three variants — `Standard`, `Obsidian`, and `GitHub`. Configured per-client via LSP `workspace/configuration` (`patto.markdown.defaultFlavor`). Implemented in `src/markdown/flavor.rs`.
- **LSP config file**: `patto-lsp.toml` (searched in XDG config dirs under namespace `patto`) configures Zotero credentials. Fields: `[zotero] user_id`, `api_key`, `endpoint` — also accepted as top-level keys with `ZOTERO_*` aliases.
- **`stable_id` on `AstNode`**: A `Mutex<Option<i64>>` assigned at parse time. Used by the WebSocket preview to identify lines across incremental updates (rendered as `data-line-id` attributes in HTML).

## Common Development Workflows

### Adding syntax features
1. Add grammar rules to `src/patto.pest`
2. Update `AstNode` variants in `src/parser.rs` to match new rules
3. Update renderer in `src/renderer.rs` (for HTML output)
4. Update markdown exporters in `src/markdown/` (for each flavor)
5. Add LSP support: semantic tokens in `src/semantic_token.rs`, completions in `src/lsp/backend.rs`
6. Add tests in `tests/` (use `TestWorkspace` + `InProcessLspClient`)

### Modifying LSP behavior
- Edit `src/lsp/backend.rs` (implements `LanguageServer` trait)
- Edit `src/lsp/lsp_config.rs` for server initialization config (capabilities, options)
- Test with integration tests in `tests/lsp_*.rs`

### Adding editor integrations
- **VS Code**: Modify `client/src/extension.ts` (extension lifecycle, command handlers)
- **Vim/Neovim**: Modify `lua/patto/init.lua` (startup, config)
- **Vim plugin files**: `plugin/patto.vim`, `ftdetect/patto.vim`, `after/ftplugin/patto.vim`

### Frontend/Preview changes
- React/Vite code: `patto-preview-ui/src/`
- Changes automatically picked up by `cargo build` (via `build.rs`)
- Tip: Use `cd patto-preview-ui && npm run dev` for standalone dev server to iterate quickly, then `cargo build` to verify embedded version works

### TUI Preview changes
- Edit `src/bin/patto-preview-tui.rs`
- Test with `cargo build --features preview-tui && cargo run --bin patto-preview-tui -- <.pn file>`
- For chafa (image support): `cargo build --features preview-tui-chafa-static` on Linux

### Repository graph changes
- Core logic in `src/repository.rs` (parsing, file watching, graph building)
- Graph data structure uses `gdsl` crate for directed graph operations
- File watching via `notify` crate (auto-detects `.pn` file changes)

## Debugging Tips

- **Parser errors**: Use `RUST_LOG=debug` to see detailed pest parser output when adding grammar rules
- **LSP diagnostics**: Test with `cargo test lsp` (runs all LSP integration tests)
- **WebSocket preview**: Browser DevTools → Network tab shows WebSocket `/ws` messages (check format matches `{ type: "...", data: { ... } }`)
- **File watcher issues**: Repository graph caches file state in `DashMap`; verify cache invalidation when files change
- **Decimal math**: When working with deadlines/timespans, note that the format uses ISO 8601 dates (`YYYY-MM-DD`)
