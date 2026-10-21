# Milestone 6: Release

Spec §12. Goal: everything in the repo that a Play release needs, so the owner's remaining work
is in Play Console and GitHub settings only (`docs/play/README.md`).

## Scope (agreed with the owner, 2026-10-21)

1. **Release signing** (`android/app/build.gradle.kts`): an upload key from
   `whichride.upload.*` Gradle properties or `WHICHRIDE_UPLOAD_*` variables; without it a
   release build is signed with the debug key so R8 builds can be tried on a phone. The
   property `whichride.requireUploadKey` makes a missing key an error. Decision 74.
2. **Release workflow** (`.github/workflows/release.yml`): a `v*` tag that matches
   `versionName` builds the signed bundle with no caches, in the `release` environment, and
   attaches it to a GitHub release. Upload to Play stays manual. Decision 75.
3. **Play material** (`docs/play/`): store listing in EN/IT/DE/TR, Data safety and other App
   content answers taken from the code, and the owner checklist (account, upload key, secrets,
   Play Integrity link, closed test, production). Decision 76.

## Left out

- **Privacy policy page**: the owner set it aside for now (2026-10-21). Play needs its URL
  before the closed test; it will be written then.
- Automatic upload to Play (needs the account and a service account first).
- Owner steps: developer account, testers, Play Console forms, the Play Integrity link.

## Checks still to run

- Release build on the owner's phone (R8 + JNA): the 5a deferred item.
- First tag through the `Release` workflow once the secrets exist.
