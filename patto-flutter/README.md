# Patto Flutter Mobile App

A Flutter-based mobile app for Patto Notes - a plain-text note-taking system with git sync.

## Features

- **Git Sync**: Notes are stored and synchronized via git
- **Page List**: View all notes with sorting (recent, most-linked, title)
- **Page View**: Read-only rendering of Patto notes
- **Title Search**: Quick search by note title
- **Dark Mode**: System-aware theme with manual override

## Architecture

This app uses:
- **Flutter** for cross-platform UI
- **flutter_rust_bridge** to integrate the Rust parser for accurate .pn file parsing
- **Riverpod** for state management
- **go_router** for navigation
- **git2-rs** (via Rust) for native git operations

## Project Structure

```
patto-flutter/
├── lib/
│   ├── main.dart                 # App entry point
│   └── src/
│       ├── app.dart              # Main app widget
│       ├── core/
│       │   ├── router/           # Navigation
│       │   ├── theme/            # Light/dark themes
│       │   └── utils/            # Secure storage
│       ├── features/
│       │   ├── git/              # Git sync functionality
│       │   ├── notes/            # Note list and view
│       │   ├── parser/           # Parser service
│       │   └── settings/         # App settings
│       └── rust_bridge/          # Generated Rust bindings
├── rust/
│   ├── Cargo.toml
│   └── src/
│       └── api/
│           ├── parser_api.rs     # Parser bindings
│           └── git_api.rs        # Git operations
├── pubspec.yaml
└── test/
```

## Setup

### Prerequisites

- Flutter SDK >= 3.2.0
- Rust toolchain
- Cargo (for building Rust code)

### Install Dependencies

```bash
# Flutter dependencies
flutter pub get

# Generate Rust bridge code
flutter_rust_bridge_codegen generate
```

### Build

```bash
# Debug build
flutter run

# Release build
flutter build apk --release    # Android
flutter build ios --release    # iOS
```

## Development

### Rust Bridge

The Rust bridge wraps the main patto crate's parser for accurate parsing:

```rust
// Parse a document
let result = parse_document(content);

// Get wiki links
let links = get_links(content);

// Get anchors
let anchors = get_anchors(content);
```

### State Management

Uses Riverpod providers:

- `gitProvider` - Git configuration and operations
- `notesProvider` - Note list and content
- `themeModeProvider` - Light/dark mode

## TODO

- [ ] Integrate flutter_rust_bridge code generation
- [ ] Full Rust parser integration
- [ ] Git operations via Rust bridge
- [ ] Note editing support
- [ ] Full-text search
- [ ] Offline support with SQLite cache

## License

MIT
