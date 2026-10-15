#!/bin/sh
# Compile-check iOS with its installed Rust target and Xcode 27 compatibility.
# No signing, installation, target download, SDK edits, or global PATH changes.
set -eu
workspace_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
task_compiler=$(rustup which --toolchain stable rustc)
task_rustdoc=$(rustup which --toolchain stable rustdoc)
task_sysroot=$("$task_compiler" --print sysroot)
if [ ! -d "$task_sysroot/lib/rustlib/aarch64-apple-ios/lib" ]; then
  printf '%s\n' 'The stable Rust toolchain needs the aarch64-apple-ios target.' >&2
  exit 1
fi
cd "$workspace_root"
PATH="$workspace_root/scripts/ios-swift-compat:$PATH" \
RUSTC="$task_compiler" RUSTDOC="$task_rustdoc" \
rustup run stable cargo check -p universal-film-camera \
  --target aarch64-apple-ios --target-dir target/ios-validation --offline -j 2 "$@"
