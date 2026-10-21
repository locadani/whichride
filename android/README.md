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
To use a server, copy `local.properties.example` to `local.properties` (git-ignored) and fill it
in; values can also come from `-P…`, `~/.gradle/gradle.properties` or `WHICHRIDE_<NAME>`:

```properties
whichride.serverUrl=https://…            # or http://localhost:8080 with `adb reverse tcp:8080 tcp:8080` (debug builds)
whichride.cloudProjectNumber=123456789   # Play Integrity
whichride.registrationKey=…              # the owner's build before Play (decision 77)
```

Every build prints one `whichride:` line with the server, attestation and signing it uses.
`./gradlew :app:checkLocalSettings` fails if the template and the build disagree.

Play Integrity verdicts only pass for builds installed from Google Play; until then the
production server runs `PLAY_INTEGRITY=key` and only a build with `whichride.registrationKey`
can register.

## Personal

Discounts, passes, refuel credits, the user's own prices and the app chosen for each provider
live in one file, `files/personal.json`, on the phone only (backups are off for the whole app).
Rules are in `personal/` with JVM tests; nothing personal is sent anywhere (decision 67).

## Crowd prices

Price reports, the confirmation question and new-car proposals are signed writes: a Play
Integrity token bound to the install ID, path and body (decision 44). Play Integrity is linked to
the Cloud project only in milestone 6, so until then every write fails on a real phone and a
correction stays the user's own price. Tests use fakes and MockWebServer.

## Languages

English, Italian, German and Turkish (`res/values*/strings.xml`). Add every new string to all
four files; `TranslationsTest` fails on a missing string, plural or placeholder. On Android 13+
the language can be set per app (Settings → App language).
