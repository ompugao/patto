#!/usr/bin/env bash
# Cross-compile check for the Android ABIs the app ships.
#
# The Flutter build does this through Cargokit; this script is the standalone
# equivalent, so the Rust side can be verified without the Flutter toolchain.
#
# Usage: ./build-android.sh [aarch64|x86_64] ...   (default: both)
set -euo pipefail

NDK_HOME="${ANDROID_NDK_HOME:-$HOME/Android/Sdk/ndk/27.2.12479018}"
BIN="$NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin"
API=24

if [ ! -d "$BIN" ]; then
  echo "NDK toolchain not found at $BIN" >&2
  echo "set ANDROID_NDK_HOME to your NDK directory" >&2
  exit 1
fi

export ANDROID_NDK_ROOT="$NDK_HOME"
export ANDROID_NDK_HOME="$NDK_HOME"
export AR_aarch64_linux_android="$BIN/llvm-ar"
export AR_x86_64_linux_android="$BIN/llvm-ar"
export CC_aarch64_linux_android="$BIN/aarch64-linux-android$API-clang"
export CC_x86_64_linux_android="$BIN/x86_64-linux-android$API-clang"

# OpenSSL's generated Makefile calls the NDK-prefixed binutils (ranlib, ar, nm)
# by bare name, so the toolchain has to be on PATH.
export PATH="$BIN:$PATH"

archs=("$@")
if [ ${#archs[@]} -eq 0 ]; then
  archs=(aarch64 x86_64)
fi

read -r -a cargo_args <<<"${CARGO_ARGS:-}"

for arch in "${archs[@]}"; do
  echo "==> building $arch-linux-android"
  cargo build --target "$arch-linux-android" "${cargo_args[@]}"
done
