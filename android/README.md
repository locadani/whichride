# whichride for Android

Gradle project with three modules: `:designsystem` (theme generated from `design/tokens.json`),
`:core` (the Rust core through UniFFI, built by Gradle) and `:app`.

## Build

Needs JDK 21, the Android SDK with NDK 30.0.16248370, Rust (the workspace toolchain) with
`rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android`, and
`cargo install cargo-ndk --version 4.1.2`.

```sh
export JAVA_HOME=/opt/homebrew/opt/openjdk@21
./gradlew :app:assembleDebug
adb install -r app/build/outputs/apk/debug/app-debug.apk
./gradlew :core:testDebugUnitTest :app:testDebugUnitTest   # JVM tests run the real Rust (host build)
```

## Server

Without a server the app compares with the prices built into the APK and asks for km and minutes.
To use a server, set in `~/.gradle/gradle.properties` (not in the repository):

```properties
whichride.serverUrl=https://…            # or http://localhost:8080 with `adb reverse tcp:8080 tcp:8080` (debug builds)
whichride.cloudProjectNumber=123456789   # Play Integrity; registration needs it
```

Play Integrity verdicts only pass for builds installed from Google Play (milestone 6); a sideloaded
debug build can't register with a production server, so routing falls back to manual entry.
