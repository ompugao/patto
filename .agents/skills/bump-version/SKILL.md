---
name: bump-version
description: Bump the patto version (patch/minor/major), open the release PR, merge it when CI is green, tag the merge commit and check the Release workflow. Use when the user asks to bump the version or cut a release.
---

# Bump the patto version

Previous examples: v0.7.1 (PR #178, commit `89be1a9`), v0.7.2 (PR #186).
Read `git show <previous bump commit>` first if anything below looks out of date.

## 1. Decide the new version

- Find the last bump: `git log --oneline -i --grep='chore(release)'`.
- List changes since then: `git log --oneline --no-merges <last release merge>..HEAD`
  and `git log --oneline --merges <last release merge>..HEAD` for PR numbers.
- No breaking changes → patch bump.

## 2. Edit these files on branch `chore/release-vX.Y.Z`

| File | What |
|---|---|
| `Cargo.toml` | `version = "X.Y.Z"` in `[package]` |
| `Cargo.lock` | the `version` line of the `name = "patto"` package only |
| `patto-flutter/rust/Cargo.lock` | the `version` line of the `name = "patto"` package only |
| `package.json` | `"version": "X.Y.Z"` |
| `client/src/binaries/versionMetadata.ts` | `BINARY_VERSION = 'X.Y.Z'` |
| `README.md` | new top line under "## Recent Updates" |

- Leave `patto-flutter/pubspec.yaml` alone.
- In the lockfiles, other crates (for example `serde_urlencoded`) may have the
  same old version string. Edit only the line right after `name = "patto"`.
- README line style: `- **vX.Y.Z** - <area>: <feature>; <feature>; fixes: ...; dependency updates`.
  Group by area (Neovim, Patto Notes mobile app, preview, Markdown import, CI).

Verify the lockfiles still match:

```sh
cargo metadata --locked --format-version 1 >/dev/null
(cd patto-flutter/rust && cargo metadata --locked --format-version 1 >/dev/null)
```

## 3. Commit and open the PR

Commit message:

```
chore(release): bump to vX.Y.Z

Since vA.B.C <summary of user-visible changes>; plus dependency updates.
No breaking changes, so a patch bump.
```

PR title is the same as the commit summary. The PR body lists the files changed,
then "Changes since vA.B.C" as bullets with PR numbers, then
"No breaking changes, so a patch bump."

## 4. Merge and tag

1. Wait for CI: `gh pr checks <n> --watch`. Merge only when all checks pass:
   `gh pr merge <n> --merge`.
2. Create an annotated tag on the merge commit and push it:

   ```sh
   git fetch origin && git checkout master && git pull --ff-only
   git tag -a vX.Y.Z -m "Release vX.Y.Z" <merge commit>
   git push origin vX.Y.Z
   ```

   The tag push starts `.github/workflows/release.yml` (cargo-dist).

## 5. Check the builds

- master CI: `gh run list --branch master --limit 5`, then `gh run watch <id>`.
  `android.yml` also runs on master when `Cargo.toml`/`Cargo.lock` change.
- Release: `gh run list --workflow release.yml --limit 3`, then
  `gh release view vX.Y.Z` and compare the asset count with the previous release
  (v0.7.1 had 22 assets).
- If the Release workflow fails because of a fix that lands later, move the tag
  to the commit with the fix (this happened for v0.7.0 with PR #168).
