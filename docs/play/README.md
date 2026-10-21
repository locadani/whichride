# Releasing on Google Play

Owner steps for milestone 6 (`docs/plans/06-release.md`). The code side is done: release signing,
the `Release` workflow and the texts in this folder.

| File | Use |
|---|---|
| `listing/*.md` | Store listing text per language (Play Console → *Main store listing*) |
| `data-safety.md` | Answers for *App content* (Data safety, content rating, audience) |

## 1. Developer account

Create one at play.google.com/console (one-off fee, identity check that can take days). A
personal account created after November 2023 must run a closed test with at least 12 testers for
14 days before it can publish to production (spec §12). Then create the app: name `whichride`,
app, free, package `io.github.locadani.whichride` (decision 41: never changes).

## 2. Upload key

Play App Signing keeps the real signing key; you keep only the upload key. Create it once and
store the file and passwords in your password manager, never in the repo:

```bash
keytool -genkeypair -v -keystore ~/whichride-upload.jks -alias upload \
  -keyalg RSA -keysize 4096 -validity 10000
```

Local release builds use it when `android/local.properties` (copy `android/local.properties.example`) has:

```properties
whichride.upload.storeFile=/Users/<you>/whichride-upload.jks
whichride.upload.storePassword=…
whichride.upload.keyAlias=upload
whichride.upload.keyPassword=…
```

Without these, `assembleRelease` is signed with the debug key, which is fine for trying an R8
build on the phone but cannot be uploaded.

## 3. GitHub settings for the Release workflow

Repository → *Settings* → *Environments* → `release` (created by the first run if missing):

| Kind | Name | Value |
|---|---|---|
| Secret | `WHICHRIDE_UPLOAD_KEYSTORE_BASE64` | `base64 -i ~/whichride-upload.jks \| pbcopy` |
| Secret | `WHICHRIDE_UPLOAD_STORE_PASSWORD` | store password |
| Secret | `WHICHRIDE_UPLOAD_KEY_ALIAS` | `upload` |
| Secret | `WHICHRIDE_UPLOAD_KEY_PASSWORD` | key password |
| Variable | `WHICHRIDE_SERVER_URL` | the Cloud Run URL |
| Variable | `WHICHRIDE_CLOUD_PROJECT_NUMBER` | `1034831276867` |

Optionally add yourself as a required reviewer of the `release` environment, so a tag waits for
your approval before the key is used.

## 4. Build a release

1. Bump `versionCode` (+1) and `versionName` in `android/app/build.gradle.kts`, merge.
2. Tag the merge commit and push: `git tag v0.1.0 && git push origin v0.1.0`.
3. The `Release` workflow checks the tag matches `versionName`, builds the signed bundle and
   attaches `whichride-v0.1.0.aab` to a GitHub release.
4. Upload that `.aab` in Play Console.

## 5. Play Integrity

Until the app is on Play the server runs `PLAY_INTEGRITY=key` and only the owner's build, which
holds the registration key, can register (decision 77, infra/README.md step 11). To switch:

1. Play Console → *Test and release* → *App integrity* → *Play Integrity API* → link the Cloud
   project `1034831276867`. From then on a copy installed from Play gets `PLAY_RECOGNIZED`.
2. Remove `whichride.registrationKey` from `android/local.properties`.
3. A PR to `infra/prod/main.tf`: `PLAY_INTEGRITY = "google"` and drop `REGISTRATION_KEY` from
   `attached_secrets`. Merge and apply.
4. From Cloud Shell, run `revoke-installs` like `lock-offer` (infra/README.md, owner commands):
   installs registered with the key must register again, now through Play Integrity.
5. Optionally delete the `whichride-registration-key` secret.

A Play copy's registration then turns on current location, address search and price reports. A
sideloaded build stays refused; that is intended.

## 6. Closed test, then production

1. *App content*: fill everything from `data-safety.md`. The privacy policy URL is still missing.
2. *Main store listing*: texts from `listing/`, the icon (`design/logo.svg`, 512 × 512), a
   feature graphic (1024 × 500) and at least two phone screenshots.
3. *Closed testing*: create a track, add at least 12 testers (an e-mail list or a Google
   group), upload the bundle, send them the opt-in link. They must stay opted in for 14 days.
4. Ask the German and Turkish testers to check their translations (decision 71).
5. After 14 days: *Apply for production* in the Dashboard, answer the questions about the test,
   then promote the release.
