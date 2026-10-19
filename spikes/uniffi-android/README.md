# Spike: Rust core on Android via UniFFI

**Result (2026-10-19): works.** A Compose app on a Redmi Note 10 5G (arm64) calls a Rust function
that takes a record (`RateCard`) and returns a price; values match the Rust unit tests
(€9.69 and €4.73 for 10.2 km / 25 min).

## Versions

| Tool | Version |
|---|---|
| Rust target / cargo-ndk | aarch64, armv7, x86_64 / cargo-ndk 4.1.2 |
| UniFFI | 0.32.2 (proc macros, library mode bindgen) |
| NDK | 30.0.16248370 |
| Gradle / AGP / Kotlin | 9.8.1 / 9.4.1 (built-in Kotlin) / 2.4.21 |
| Compose BOM | 2026.09.00 |
| JNA | 5.19.1 (aar) |
| JDK | 21 (Homebrew openjdk@21) |

## Run

```sh
./build-rust.sh                      # .so files for 3 ABIs + Kotlin bindings
cd android && ./gradlew assembleDebug
adb install -r app/build/outputs/apk/debug/app-debug.apk
```

Needs `JAVA_HOME=/opt/homebrew/opt/openjdk@21` and `ANDROID_HOME=~/Library/Android/sdk`.

## Findings for the real project

- Release `.so` size with `opt-level="z"`, LTO and strip: ~380 KB (arm64).
- Current Compose needs **compileSdk 37**; minSdk can stay low (26 here).
- Set `ndkVersion` in Gradle, otherwise AGP can't strip the Rust library.
- JNA ships libraries for obsolete ABIs (mips, armeabi): add `abiFilters` in the real app.
- Bindings are generated from the host build (`.dylib`); move this into Gradle (Gobley or a
  task calling cargo-ndk) so a single `./gradlew` builds everything, CI included.
- uniffi-bindgen tries to run ktlint; `--no-format` avoids the warning.
