#!/usr/bin/env bash
# Builds the Rust core for Android and generates the Kotlin bindings.
set -euo pipefail
cd "$(dirname "$0")/core"
export ANDROID_NDK_HOME="${ANDROID_NDK_HOME:-$HOME/Library/Android/sdk/ndk/30.0.16248370}"
cargo ndk -t arm64-v8a -t armeabi-v7a -t x86_64 -o ../android/app/src/main/jniLibs build --release
# Bindings are generated from the host build of the same crate (library mode).
cargo build --release --lib
cargo run --release --bin uniffi-bindgen -- generate \
  --library target/release/libspike_core.dylib \
  --language kotlin --no-format --out-dir ../android/app/src/main/java
