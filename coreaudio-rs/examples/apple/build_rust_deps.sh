#!/bin/bash

set -euo pipefail

PATH=$PATH:$HOME/.cargo/bin

RUSTUP_BIN="${RUSTUP:-$(command -v rustup)}"
# Avoid mixing Homebrew's compiler with rustup's target standard libraries.
RUSTC="$($RUSTUP_BIN which --toolchain stable rustc)"
export RUSTC
export RUSTUP_TOOLCHAIN=stable
CARGO_BIN="$($RUSTUP_BIN which --toolchain stable cargo)"

if [[ -n "${DEVELOPER_SDK_DIR:-}" ]]; then
  # Assume we're in Xcode, which means we're probably cross-compiling.
  # In this case, we need to add an extra library search path for build scripts and proc-macros,
  # which run on the host instead of the target.
  # (macOS Big Sur does not have linkable libraries in /usr/lib/.)
  export LIBRARY_PATH="${DEVELOPER_SDK_DIR}/MacOSX.sdk/usr/lib:${LIBRARY_PATH:-}"
fi

"$CARGO_BIN" build \
    --target aarch64-apple-tvos-sim \
    --target aarch64-apple-tvos \
    --target aarch64-apple-visionos \
    --target aarch64-apple-visionos-sim \
    --release

"$CARGO_BIN" build --target aarch64-apple-ios --target aarch64-apple-ios-sim --release
