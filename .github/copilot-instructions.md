# Patto Note - Copilot Instructions

## Project Overview

Patto is a plain-text note format (`.pn` files) with LSP support, task management, and wiki-style links. It ships as:
- A Rust library + multiple CLI binaries (`patto-lsp`, `patto-preview`, `patto-markdown-renderer`, `patto-markdown-importer`, `patto-html-renderer`)
- A VS Code extension (TypeScript, Webpack)
- Vim/Neovim plugin (Lua)
- A React/Vite preview UI embedded into the `patto-preview` binary via `rust-embed`

## Build & Test

### Rust
```sh
cargo build                     # Also triggers patto-preview-ui frontend build via build.rs
cargo test                      # Run all tests
cargo test <test_name>          # Run a single test by name (substring match)
cargo test --test lsp_rename    # Run a specific test file
cargo fmt --all                 # Format
cargo clippy --all-targets --all-features  # Lint
```

### VS Code Extension
```sh
pnpm install    # From repo root (installs root + client/)
npm run build   # Webpack bundle → dist/extension.js
npm run lint    # ESLint on client/
```

### Preview UI (React/Vite, embedded in Rust binary)
```sh
cd patto-preview-ui
npm install
npm run build   # Output goes to patto-preview-ui/dist/, picked up by rust-embed at cargo build time
npm run dev     # Dev server (standalone, not embedded)
```

> **Important:** `cargo build` runs `build.rs` which automatically runs `npm install && npm run build` in `patto-preview-ui/` whenever `patto-preview-ui/src/` changes. The `patto-preview-ui/dist/` directory is embedded into the `patto-preview` binary at compile time via `rust-embed`.

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
