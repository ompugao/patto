# Patto Notes for Android

A mobile client for [patto](../README.md) notes. Notes live in a git repository
that the app clones onto the device, so everything works offline and syncs when
you ask it to.

- Several workspaces, each its own repository and its own folder on the device,
  switched from the note list or from settings
- Note list with fuzzy title search and sorting by recency, backlinks or name
- Note view rendering the full syntax: nesting, links, anchors, decorations,
  code with highlighting, tables, math, images, embeds and task markers
- Every `[@embed ...]` the web preview supports: YouTube and Google Photos
  thumbnails, PDFs in an in-app viewer, tweets with their text, Speaker Deck
  and SlideShare decks with their first slide, and a link card for any other
  URL. Tweets, decks and other pages open in an in-app web view; videos and
  Google Photos shares open in the app that owns them
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
    app.dart           theme, first-run gate, bottom navigation
    core/              settings, workspaces, Riverpod providers
    features/          notes, tasks, editor, sync, settings, workspaces
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
flutter test                       # settings and workspace storage
./rust/build-android.sh            # cross-compile check for both ABIs
```

## Continuous integration

`.github/workflows/android.yml` builds the release APKs on every push and pull
request that touches the app or the core crate, and uploads them as a build
artifact named `patto-notes-apk`. These builds are signed with the runner's
debug key and carry the version from `pubspec.yaml`. It also fails if the
committed bridge bindings differ from a fresh `flutter_rust_bridge_codegen
generate`.

The workflow pins the Flutter version, the NDK and the codegen version; they
have to stay in step with `pubspec.yaml`, `android/app/build.gradle.kts` and
`rust/Cargo.toml`.

## Releases

Pushing a `vX.Y.Z` tag runs the `Release` workflow, which creates the GitHub
Release. When that workflow finishes successfully, the Android workflow runs
once more (as a `workflow_run`, because a release created with `GITHUB_TOKEN`
fires no `release` event), checks out the tag, and attaches
`patto-notes-X.Y.Z-arm64-v8a.apk` and `patto-notes-X.Y.Z-x86_64.apk` to the
release. To attach them to a release again, run the Android workflow by hand
with the tag as its input; it builds that tag.

A release build takes its version from the tag: the version name is the tag
without the `v`, and the version code is `major * 1000000 + minor * 1000 +
patch`, so each release installs over the one before. A pre-release such as
`v0.7.0-rc.1` shares its version code with `v0.7.0`, which Android still
installs over it. Flutter adds `1000 * ABI` for the split APKs, as it does for
the regular builds.

Release APKs are signed with an upload keystore kept in four repository
secrets:

| Secret | Content |
|---|---|
| `ANDROID_KEYSTORE_BASE64` | the `.jks` file, base64-encoded |
| `ANDROID_KEYSTORE_PASSWORD` | its store password |
| `ANDROID_KEY_ALIAS` | the key alias (`upload` below) |
| `ANDROID_KEY_PASSWORD` | the key password |

Generate the keystore once and keep it somewhere safe; the key must stay the
same for the lifetime of the app, because Android refuses to update an
installed app with one signed by another key.

```sh
keytool -genkeypair -v -keystore upload-keystore.jks -keyalg RSA -keysize 2048 \
  -validity 10000 -alias upload
base64 -w0 upload-keystore.jks | gh secret set ANDROID_KEYSTORE_BASE64
gh secret set ANDROID_KEYSTORE_PASSWORD
gh secret set ANDROID_KEY_ALIAS --body upload
gh secret set ANDROID_KEY_PASSWORD
```

Only a release build writes the keystore and `android/key.properties` from
these secrets, and deletes them after the build; pushes, pull requests and
manual runs without a tag never see the key. `build.gradle.kts` signs with the
keystore when the file exists and with the debug key otherwise. Without the
secrets a release still builds, but the job carries a warning and the APKs are
debug-signed. Every GitHub runner has its own debug key, so one debug-signed
release cannot update another, and neither can the first properly signed
release update a debug-signed install: the app has to be uninstalled first.

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

**Workspaces.** A workspace is a repository plus the folder it was cloned into,
named after the workspace id. An upgrade from the single-workspace version keeps
the folder that one used (`notes`), so the clone already on the device is not
thrown away. The link index is keyed by root, so switching back to a workspace
does not rebuild it.

**Merge conflicts** are resolved in favour of the copy on the phone, which
cannot present a merge. The sync report lists the files that were auto-resolved.

**The editor is pinned to a fork of re_editor**
([ompugao/re-editor](https://github.com/ompugao/re-editor), branch
`fix/mobile-cursor-left-at-line-start`), which is 0.10.0 plus one fix. On
Android the editor sends only the current line to the keyboard, prefixed with a
zero-width space so a backspace at the line start can be recognised; a caret
arriving at offset 0 was always taken for that backspace, so moving the cursor
left at the start of a line deleted the line break. Point `pubspec.yaml` back at
the published package once the fix is released upstream.
