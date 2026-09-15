# Patto Notes for Android

A mobile client for [patto](../README.md) notes. Notes live in a git repository
that the app clones onto the device, so everything works offline and syncs when
you ask it to.

- Note list with fuzzy title search and sorting by recency, backlinks or name
- Note view rendering the full syntax: nesting, links, anchors, decorations,
  code with highlighting, tables, math, images, embeds and task markers
- Backlinks and 2-hop links under every note
- Tasks grouped by deadline, a completed-task review by timeframe, and status
  changes that write `completed_at` and time tracking the way the language
  server does
- Plain-text editor with tab nesting and wiki-link completion
- Sync: commit, fetch, fast-forward or merge, push
- Appearance: light, dark or system theme, and a note text size from 80% to
  180% that applies to the note view and the editor

## Layout

```
patto-flutter/
  lib/
    main.dart          entry point; loads the CA bundle and starts the app
    app.dart           theme, onboarding gate, bottom navigation
    core/              settings, workspace, Riverpod providers
    features/          notes, tasks, editor, sync, settings
    src/rust/          GENERATED Dart bindings
  rust/                the Rust core (see rust/src/api)
  rust_builder/        Cargokit, builds the Rust library during a Flutter build
  assets/cacert.pem    trust roots for libgit2
```

The Rust side holds all the logic: parsing and flattening notes for display,
the link index, task aggregation and edits, and git. `rust/src/api` is plain
Rust that builds and tests on the host; `rust/src/frb_api.rs` is the thin
surface exposed to Dart.

## Building

Prerequisites: Flutter (stable), the Rust toolchain, and the Android SDK with
NDK 27.2.12479018.

```sh
rustup target add aarch64-linux-android x86_64-linux-android
cargo install flutter_rust_bridge_codegen --version 2.13.0
flutter pub get
flutter run

# Release APKs, one per ABI. Name the platforms explicitly: Flutter otherwise
# also emits an armeabi-v7a slice, for which no Rust library is built.
flutter build apk --release --split-per-abi \
  --target-platform android-arm64,android-x64
```

Cargokit looks for the SDK command-line tools at `$ANDROID_HOME/cmdline-tools/latest`,
so install that package (not just a versioned one).

After changing anything in `rust/src/frb_api.rs` or the types it exposes:

```sh
flutter_rust_bridge_codegen generate
```

The generated Dart in `lib/src/rust/` and `rust/src/frb_generated.rs` are
committed, so a plain `flutter run` works without the codegen installed.

## Testing

```sh
cd rust && cargo test              # 71 tests: rendering, index, tasks, git
cd rust && cargo test -- --ignored # also clones over HTTPS, needs network
flutter analyze
./rust/build-android.sh            # cross-compile check for both ABIs
```

## Continuous integration

`.github/workflows/android.yml` builds the release APKs on every push and pull
request that touches the app or the core crate, and uploads them as a build
artifact named `patto-notes-apk`. It also fails if the committed bridge
bindings differ from a fresh `flutter_rust_bridge_codegen generate`.

The workflow pins the Flutter version, the NDK and the codegen version; they
have to stay in step with `pubspec.yaml`, `android/app/build.gradle.kts` and
`rust/Cargo.toml`.

## Notes for maintainers

**Certificates.** `openssl-src` configures Android builds with `no-stdio`, so
OpenSSL there cannot open a PEM file: pointing libgit2 at a path fails, and so
does `SSL_CERT_FILE`. The bundled roots are parsed in Dart-visible startup code
and handed to libgit2 one certificate at a time. See `rust/src/api/git.rs`.

**Streaming results.** flutter_rust_bridge turns a function taking a
`StreamSink` into a Dart `Stream` and discards that function's own `Result`, so
an error would surface on a future nobody awaits. Clone, sync and index build
therefore report their outcome as a terminal event on the stream; see
`rust/src/api/events.rs`.

**Note timestamps** come from git, not the filesystem. A clone stamps every file
with the time of the clone, so the note list would otherwise show one date for
everything and "Recent" would mean nothing. The history is walked once per index
to find the last commit touching each note. A note whose working copy differs
from HEAD keeps its file time, because it really was edited here.

**Merge conflicts** are resolved in favour of the copy on the phone, which
cannot present a merge. The sync report lists the files that were auto-resolved.
