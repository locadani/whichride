# Registration key: running without Google Play for now

**Status:** Draft, awaiting owner approval · **Date:** 2026-10-21 · **Decision:** 77 (proposed)

## 1. Why

Location, address search and price reports need a registered install (decision 59), and the
server registers an install only with a Play Integrity verdict of `PLAY_RECOGNIZED` (decision 33).
That verdict exists only for a copy installed from Google Play, which needs a paid developer
account. The owner has decided not to publish on Play for now: for the time being the owner is
the only user, with a sideloaded build.

So the server needs a second way to decide who may register, one that:

- lets the owner's sideloaded build use every feature against the live server;
- keeps strangers out, because the server is public and the OpenRouteService quota and the
  shared prices must be protected;
- can be swapped back to Play Integrity **by configuration only**, with no code change, the day
  the app goes to Play.

## 2. What changes for the owner

| | Today | With the registration key | After moving to Play |
|---|---|---|---|
| Server `PLAY_INTEGRITY` | `google` | `key` | `google` |
| Server secret | none | `REGISTRATION_KEY` | (left unused, can be deleted) |
| App build | Play Integrity | `whichride.registrationKey` in `~/.gradle/gradle.properties` | property removed |
| Current location, address search, price reports on the phone | refused | work | work (installed from Play) |

## 3. Design

### 3.1 Server: a third integrity mode

`IntegrityMode` (`crates/server/src/config.rs`) and `Integrity` (`crates/server/src/integrity.rs`)
gain a variant:

```rust
pub enum Integrity {
    Google(GoogleIntegrity),
    /// Owner-only use before Play: the token must equal the registration key.
    Key(RegistrationKey),
    Off,
}
```

- **Configuration:** `PLAY_INTEGRITY=key` requires `REGISTRATION_KEY`; startup fails without
  it, or if it is shorter than 32 bytes. `off` stays refused on Cloud Run (unchanged).
- **Check:** `Integrity::verify(token, expected_hash)` in `Key` mode compares the token with
  the key in **constant time** (`subtle::ConstantTimeEq`) and ignores the request hash. A
  missing token is `IntegrityError::Missing`; a wrong one is `Refused("key")`. Both reach the
  app as the same `integrity_failed` it already handles.
- **Nothing else moves.** Registration (`Attested`) and writes (`Signed`) already call
  `verify` for every request, so both are protected by the key; the routes, the extractors, the
  rate limits (`limits.rs`) and the reputation rules are untouched.
- **`RegistrationKey`** wraps the value so that `Debug` prints `RegistrationKey(…)` and the key
  never reaches a log line.

The key travels in the existing `X-Integrity-Token` header. A separate header was considered and
rejected: it would mean a second code path on both sides for the same job, "prove this
request may write", which is exactly what the header already means.

### 3.2 App: attestation picked at build time

`IntegrityTokens` (`android/app/.../server/IntegrityTokens.kt`) is already the seam:

```kotlin
fun interface IntegrityTokens { suspend fun token(requestHash: String): String? }
```

- New `RegistrationKeyTokens(key)`: returns the key for every request hash.
- `AppContainer` chooses, in this order:
  1. `BuildConfig.REGISTRATION_KEY` not empty → `RegistrationKeyTokens`;
  2. `CLOUD_PROJECT_NUMBER == 0` → no tokens (as today);
  3. otherwise → `PlayIntegrityTokens` (as today, unchanged).
- `build.gradle.kts` reads `whichride.registrationKey` from Gradle properties (never from the
  repo) into `BuildConfig.REGISTRATION_KEY`, default empty.
- **Guard:** the release workflow (`.github/workflows/release.yml`) fails if
  `whichride.registrationKey` is set, so a bundle meant for Play can never contain the key. The
  Gradle build fails too when `whichride.requireUploadKey=true` and a registration key is set.

`Installs`, `ServerApi` and every screen stay as they are.

### 3.3 Infrastructure

The `cloud-run-app` module attaches all secrets with one switch, `attach_secret_env`. A
secret with no version stops Cloud Run from starting, so a new secret cannot be added and
attached in one apply. The module changes the switch to a set:

```hcl
secret_env       = { ORS_API_KEY = "ors-api-key", REGISTRATION_KEY = "registration-key" }
attached_secrets = ["ORS_API_KEY"]                       # step 1
attached_secrets = ["ORS_API_KEY", "REGISTRATION_KEY"]   # step 3
```

The key is created by the owner in Cloud Shell, never by Terraform, so it is not in the
Terraform state (same rule as `DATABASE_URL` and the ORS key).

## 4. Security

| Threat | Outcome |
|---|---|
| A stranger calls the API | Cannot register without the key: no routing quota used, no prices written. Public reads (`/v1/catalogue`) are public today too. |
| Someone copies the owner's APK | Can extract the key and register. Accepted: the APK is not distributed. Mitigation: rotate the key (§6) and delete the installs. |
| The key leaks from a log | `RegistrationKey` hides itself in `Debug`; the app never logs tokens. |
| Timing attack on the comparison | Constant-time comparison. |
| A Play build ships the key | Release workflow and Gradle refuse it (§3.2). |
| Key mode left on after moving to Play | The Play switch (§6) sets `PLAY_INTEGRITY=google`; installs registered with the key keep their bearer token, so §6 also says to delete them. |

What the key does **not** give: Play Integrity's per-request binding (`requestHash`) and device
verdicts. Acceptable with one user; it is the reason this mode is temporary.

## 5. Rollout

1. **PR 1, code:** server mode + tests, app `RegistrationKeyTokens` + selection + tests, Gradle
   property, release guard, the owner command `delete-installs` (§6), the `subtle` crate for the comparison, module change with `attached_secrets = ["ORS_API_KEY"]`, docs
   (this file, decision 77, `infra/README.md`, `docs/play/README.md`). After merge, the deploy
   ships the new image with `PLAY_INTEGRITY=google`: nothing changes yet. The infra apply
   creates the empty `registration-key` secret.
2. **Owner, Cloud Shell:** create the key and store it:
   ```bash
   openssl rand -base64 48 | tr -d '\n' | gcloud secrets versions add registration-key --data-file=-
   ```
   Copy the same value into `~/.gradle/gradle.properties` as `whichride.registrationKey=…`.
3. **PR 2, infra:** `attached_secrets` adds `REGISTRATION_KEY`, `PLAY_INTEGRITY = "key"`. Apply.
4. **Phone:** build and install; registration succeeds; location, search and reports work.

## 6. Switching to Google Play later

1. Remove `whichride.registrationKey` from `~/.gradle/gradle.properties`.
2. Terraform: `PLAY_INTEGRITY = "google"`, drop `REGISTRATION_KEY` from `attached_secrets`.
3. Delete the installs registered with the key (owner command `delete-installs --before <date>`, added in PR 1, run like `lock-offer`), so only Play-verified installs remain.
4. Optionally delete the `registration-key` secret.

**Rotating the key** (if the APK leaks): add a new secret version, redeploy (Cloud Run reads
`latest` on start), update `gradle.properties`, reinstall, and delete the old installs.

## 7. Testing

- **Server (Rust):** configuration accepts `key` only with a long enough key, refuses an unknown
  mode; `verify` accepts the key, refuses missing / wrong / prefix-of-key tokens; `Debug` hides
  the key; an end-to-end test registers and writes in key mode, and is refused without the key.
- **App (JVM):** container selection order (key, none, Play); `RegistrationKeyTokens` returns
  the key; Gradle refuses a key together with `requireUploadKey`.
- **Phone:** the owner checks current location, address search and a price report.

## 8. Alternatives considered

| Alternative | Why not |
|---|---|
| `PLAY_INTEGRITY=off` on Cloud Run | Anyone could register and write; the routing quota is open to the internet. |
| Pay for Play now | The owner's choice for now; this design keeps that path one config change away. |
| Firebase App Check / other attestation | A new dependency for a temporary need; still needs Play or a debug provider. |
| A separate `X-Registration-Key` header | Two code paths for one job (§3.1). |
| Allow-list of install IDs | Needs registration to work first; the key is simpler and covers it. |
