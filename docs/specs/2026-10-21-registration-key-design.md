# Registration key: running without Google Play for now

**Status:** Approved 2026-10-21 · **Date:** 2026-10-21 · **Decision:** 77

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
| App build | Play Integrity | `whichride.registrationKey` in `android/local.properties` | property removed |
| Current location, address search, price reports on the phone | refused | work | work (installed from Play) |

### In plain words

**The question the server asks.** The server is public on the internet. Before it lets a
request use the routing service or write a price, it asks: *is this really whichride on a real
phone?* Without that check, anyone could write a script that pretends to be the app, uses up the
routing quota and posts fake prices. `PLAY_INTEGRITY` chooses how the server answers:

| Setting | How the server decides | Where it is used |
|---|---|---|
| `google` | Asks Google: "was this app installed from Google Play, unmodified?" Only Play installs pass. | Once the app is on Play. Today's setting, which is why a sideloaded phone is refused. |
| `off` | Does not check: every request is accepted. | Only on the laptop, when the app talks to a server running on that laptop. The server refuses to start with `off` on Cloud Run. Already exists. |
| `key` (new) | Checks for a password, the **registration key**. | The real server, while the owner is the only user and the app is not on Play. |

**The registration key** is a long random password (about 64 characters) that only the owner
knows. It is kept in two places: Google Cloud's secret store, where the server reads it, and
the owner's `android/local.properties`, from where it is built into the owner's copy of the
app. The app sends it with every request that needs it; the server compares it with its copy.
The server refuses to start with a key shorter than 32 bytes, so a short, guessable key such as
`ciao` cannot be set by mistake.

**"Attestation picked at build time."** Attestation is the proof the app attaches to a request:
either a Play Integrity token (Google vouching that the app came from Play) or the registration
key. The choice is made when the app is built on the laptop, not while it runs. `./gradlew` looks
in `android/local.properties`:

| What the build finds | What the app does |
|---|---|
| `whichride.registrationKey=…` | Sends the key. The owner's phone build now. |
| No key, but a Cloud project number | Asks Google for a Play Integrity token. The Play build later. |
| Neither | Sends nothing. A test build without a server. |

So the owner adds one line once and builds as usual; for Play, deletes that line and builds
again. No code changes. The release workflow, which builds the file uploaded to Play, refuses to
run when a key is set, so the password can never end up in the app on the store.

In the code, the app already asks one component, "give me the proof for this request", without
knowing how the proof is made. Today that component only knows Play; this change adds a second
version that returns the key, and the build decides which one goes into the app.

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
- `build.gradle.kts` reads `whichride.registrationKey` from `android/local.properties` into
  `BuildConfig.REGISTRATION_KEY`, default empty (§3.4).
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

### 3.4 Where the owner's local values live

All of whichride's local, per-machine values move from `~/.gradle/gradle.properties` to
`android/local.properties`: `whichride.serverUrl`, `whichride.cloudProjectNumber`,
`whichride.registrationKey` and `whichride.upload.*`. The owner wants them kept with the app,
and `local.properties` is the usual Android place for per-project values that are not committed
(it already holds `sdk.dir`).

- **Never committed:** `.gitignore` already lists `/android/local.properties`; gitleaks scans
  every push; a new CI step fails if the file is ever tracked by git.
- **Reading:** Gradle does not load `local.properties` by itself, so the build reads it once and
  looks values up there first, then in Gradle properties (`-P…`, `~/.gradle/gradle.properties`)
  and environment variables, so CI and the release workflow keep working unchanged.
- **Migration:** the owner moves the existing lines over once; the build also still accepts them
  from `~/.gradle/gradle.properties`.

**Knowing what the file needs.** Following the common "committed template" practice:

- **Template:** `android/local.properties.example` is committed. It lists every `whichride.*`
  setting with a comment (what it is for, when it is needed, where the value comes from) and no
  secret values. If `local.properties` is lost, copy the template and fill it in; the key itself
  is recoverable from Secret Manager.
- **Drift check:** a test fails when the build reads a `whichride.*` setting that the template
  does not list, so the template cannot go stale.
- **Build summary:** every build prints one line saying what it will do, for example
  `whichride: server https://… · attestation: registration key · signing: debug key`, or
  `whichride: no server (built-in prices only) · see android/local.properties.example`.
- **Errors:** a malformed value (as today), a registration key shorter than 32 characters, or a
  key in a Play release stops the build. A missing value does not: a build without a server is
  valid (CI, offline work), and the summary line makes it visible. The release workflow already
  fails without the upload key or the server settings.
- **Not used:** Google's Secrets Gradle Plugin. Its last release is from February 2022, it is not
  confirmed to work with Android Gradle Plugin 9, it copies every value in the file into the app
  (the upload-key passwords would need excluding one by one), and it does not validate.

## 4. Security

| Threat | Outcome |
|---|---|
| A stranger calls the API | Cannot register without the key: no routing quota used, no prices written. Public reads (`/v1/catalogue`) are public today too. |
| Someone copies the owner's APK | Can extract the key and register. Accepted: the APK is not distributed. Mitigation: rotate the key and run `revoke-installs` (§6). |
| The key leaks from a log | `RegistrationKey` hides itself in `Debug`; the app never logs tokens. |
| Timing attack on the comparison | Constant-time comparison. |
| A Play build ships the key | Release workflow and Gradle refuse it (§3.2). |
| Key mode left on after moving to Play | The Play switch (§6) sets `PLAY_INTEGRITY=google`; installs registered with the key keep their bearer token, so §6 also says to revoke them. |

What the key does **not** give: Play Integrity's per-request binding (`requestHash`) and device
verdicts. Acceptable with one user; it is the reason this mode is temporary.

## 5. Rollout

1. **PR 1, code:** server mode + tests, app `RegistrationKeyTokens` + selection + tests, Gradle
   property, release guard, the owner command `revoke-installs` (§6), the `subtle` crate for the comparison, module change with `attached_secrets = ["ORS_API_KEY"]`, docs
   (this file, decision 77, `infra/README.md`, `docs/play/README.md`). After merge, the deploy
   ships the new image with `PLAY_INTEGRITY=google`: nothing changes yet. The infra apply
   creates the empty `registration-key` secret.
2. **Owner, Cloud Shell:** create the key and store it:
   ```bash
   openssl rand -base64 48 | tr -d '\n' | gcloud secrets versions add whichride-registration-key --data-file=-
   ```
   Copy the same value into `android/local.properties` as `whichride.registrationKey=…`.
3. **PR 2, infra:** `attached_secrets` adds `REGISTRATION_KEY`, `PLAY_INTEGRITY = "key"`. Apply.
4. **Phone:** build and install; registration succeeds; location, search and reports work.

## 6. Switching to Google Play later

1. Remove `whichride.registrationKey` from `android/local.properties`.
2. Terraform: `PLAY_INTEGRITY = "google"`, drop `REGISTRATION_KEY` from `attached_secrets`.
3. Revoke the installs registered with the key: `revoke-installs` (added in PR 1, run like
   `lock-offer`) gives every install a token nobody knows. Installs are not deleted, because
   reports, votes and price history refer to them; the app registers again by itself the next
   time the server answers 401, now through Play Integrity.
4. Optionally delete the `registration-key` secret.

**Rotating the key** (if the APK leaks): add a new secret version, redeploy (Cloud Run reads
`latest` on start), update `android/local.properties`, reinstall, and run `revoke-installs`.

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
