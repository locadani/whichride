# Registration Key Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let the owner's sideloaded build register with the live server through a secret registration key, switchable back to Play Integrity by configuration only.

**Architecture:** The server gains a third integrity mode, `PLAY_INTEGRITY=key`, checked inside the existing `Integrity::verify`, so registration and every signed write are covered without touching routes. The app gains a second `IntegrityTokens` implementation chosen at build time from `android/local.properties`, read through a small `LocalSettings` helper that also prints a build summary and checks the committed template. Terraform attaches secrets by name so the new secret can be created before it is attached.

**Tech Stack:** Rust (Axum, sqlx, `subtle`), Kotlin + Gradle (AGP 9.4, buildSrc), Terraform, GitHub Actions.

**Spec:** `docs/specs/2026-10-21-registration-key-design.md` (approved 2026-10-21).

## Global Constraints

- Server mode names: `google` (default), `key`, `off`; `off` stays refused on Cloud Run.
- `PLAY_INTEGRITY=key` requires `REGISTRATION_KEY`, at least 32 bytes after trimming.
- The key travels in the existing `X-Integrity-Token` header (`TOKEN_HEADER = "x-integrity-token"`).
- Comparison in constant time (`subtle::ConstantTimeEq`); `Debug` never prints the key.
- App setting name: `whichride.registrationKey`; key characters `[A-Za-z0-9+/=_-]`, length ≥ 32.
- Local settings order: `android/local.properties` → Gradle property (`-P`, `~/.gradle/gradle.properties`) → environment variable `WHICHRIDE_<NAME>` (dots → `_`, upper case).
- A registration key together with `whichride.requireUploadKey=true` is a build error.
- The key is never created by Terraform and never committed.
- Commit messages end with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- Run Android Gradle with `JAVA_HOME=/opt/homebrew/opt/openjdk@21`; Rust DB tests need `TEST_POSTGRES_URL=postgres://postgres@localhost:55432/postgres` (smoke-pg container).

## Review Focus

1. A key with surrounding whitespace or a trailing newline (copied from `openssl … | gcloud`) — server trims; Gradle trims; both sides must agree. Pinned in Task 1 (`RegistrationKey::new` trims) and Task 4 (`LocalSettings.get` trims).
2. A token that is a prefix or extension of the key must be refused, not accepted by a length-short comparison. Pinned in Task 1.
3. A key containing `"` or `\` would break the generated `BuildConfig` Java string. Pinned in Task 4 (character class check).
4. `local.properties` missing entirely (fresh clone, CI) must still build, with the "no server" summary. Pinned in Task 4.
5. After `revoke-installs`, the old bearer token must be refused while reports and votes keep their rows. Pinned in Task 3.

---

### Task 1: Server — `RegistrationKey` and `Integrity::Key`

**Files:**
- Modify: `crates/server/Cargo.toml` (`[dependencies]`), `Cargo.toml` (workspace dependencies)
- Modify: `crates/server/src/integrity.rs` (enum `Integrity`, `impl Integrity::verify`, tests module)

**Interfaces:**
- Produces: `pub struct RegistrationKey` with `pub const MIN_BYTES: usize = 32`, `pub fn new(key: &str) -> Result<RegistrationKey, &'static str>`; `Integrity::Key(RegistrationKey)`.

- [ ] **Step 1: Add the dependency.** Workspace `Cargo.toml` `[workspace.dependencies]`: `subtle = "2.6"`. `crates/server/Cargo.toml` `[dependencies]`: `subtle.workspace = true` (alphabetical, after `sqlx`).

- [ ] **Step 2: Write the failing tests** at the end of `mod tests` in `integrity.rs`:

```rust
    const KEY: &str = "owner-test-key-owner-test-key-owner-test-key";

    fn key() -> Integrity {
        Integrity::Key(RegistrationKey::new(KEY).unwrap())
    }

    #[test]
    fn a_registration_key_must_be_long_enough() {
        assert!(RegistrationKey::new("short").is_err());
        assert!(RegistrationKey::new(&"k".repeat(31)).is_err());
        assert!(RegistrationKey::new(&"k".repeat(32)).is_ok());
    }

    #[test]
    fn a_registration_key_is_trimmed() {
        let key = RegistrationKey::new(&format!("  {KEY}\n")).unwrap();
        assert_eq!(key, RegistrationKey::new(KEY).unwrap());
    }

    #[test]
    fn debug_hides_the_registration_key() {
        let shown = format!("{:?}", RegistrationKey::new(KEY).unwrap());
        assert!(!shown.contains("owner"), "{shown}");
    }

    #[tokio::test]
    async fn key_mode_accepts_the_key_for_any_request() {
        assert_eq!(key().verify(Some(KEY), "h1").await, Ok(()));
        assert_eq!(key().verify(Some(KEY), "h2").await, Ok(()));
    }

    #[tokio::test]
    async fn key_mode_refuses_anything_else() {
        assert_eq!(key().verify(None, "h").await, Err(IntegrityError::Missing));
        assert_eq!(key().verify(Some(""), "h").await, Err(IntegrityError::Missing));
        for wrong in [&KEY[..KEY.len() - 1], &format!("{KEY}x"), "owner-test-key-owner-test-key-other-key"] {
            assert_eq!(key().verify(Some(wrong), "h").await, Err(IntegrityError::Refused("key")), "{wrong}");
        }
    }
```

- [ ] **Step 3: Run to see them fail.** `cargo test -p whichride-server --lib integrity` → compile error: `RegistrationKey` / `Integrity::Key` not found.

- [ ] **Step 4: Implement.** In `integrity.rs`, add `use std::fmt;` and `use subtle::ConstantTimeEq;`, then:

```rust
/// The owner's registration key (`PLAY_INTEGRITY=key`, spec 2026-10-21): every install that
/// presents it may register and write. `Debug` never shows it.
#[derive(Clone, PartialEq, Eq)]
pub struct RegistrationKey(String);

impl RegistrationKey {
    pub const MIN_BYTES: usize = 32;

    pub fn new(key: &str) -> Result<RegistrationKey, &'static str> {
        let key = key.trim();
        if key.len() < Self::MIN_BYTES {
            return Err("must be at least 32 bytes");
        }
        Ok(RegistrationKey(key.to_owned()))
    }

    /// Constant time for equal lengths; a different length is refused at once.
    fn matches(&self, token: &str) -> bool {
        self.0.as_bytes().ct_eq(token.as_bytes()).into()
    }
}

impl fmt::Debug for RegistrationKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("RegistrationKey(<hidden>)")
    }
}
```

Add the variant between `Google` and `Off`:

```rust
    /// Owner-only use before Play: the token must be the registration key.
    Key(RegistrationKey),
```

In `verify`, replace `let Integrity::Google(google) = self else { return Ok(()); };` with:

```rust
        let google = match self {
            Integrity::Google(google) => google,
            Integrity::Key(key) => {
                let token = token.filter(|token| !token.is_empty()).ok_or(IntegrityError::Missing)?;
                return if key.matches(token) { Ok(()) } else { Err(IntegrityError::Refused("key")) };
            }
            Integrity::Off => return Ok(()),
        };
```

- [ ] **Step 5: Run.** `cargo test -p whichride-server --lib integrity` → PASS. `cargo clippy -p whichride-server --all-targets -- -D warnings` → clean.

- [ ] **Step 6: Commit** `feat(server): registration key integrity mode`.

### Task 2: Server — `PLAY_INTEGRITY=key` configuration and end-to-end

**Files:**
- Modify: `crates/server/src/config.rs` (`IntegrityMode`, `from_lookup`, tests)
- Modify: `crates/server/src/commands.rs:28-36` (`serve`)
- Modify: `.env.example`
- Test: `crates/server/tests/installs.rs`

**Interfaces:**
- Consumes: `RegistrationKey::new`, `Integrity::Key` (Task 1).
- Produces: `IntegrityMode::Key(RegistrationKey)`.

- [ ] **Step 1: Failing config tests** in `config.rs` `mod tests`:

```rust
    const KEY: &str = "owner-test-key-owner-test-key-owner-test-key";

    #[test]
    fn key_mode_reads_the_registration_key() {
        let config = ServeConfig::from_lookup(lookup(&[
            ("ORS_API_KEY", "k"),
            ("PLAY_INTEGRITY", "key"),
            ("REGISTRATION_KEY", KEY),
            ("K_SERVICE", "whichride"),
        ]))
        .unwrap();
        assert_eq!(config.integrity, IntegrityMode::Key(RegistrationKey::new(KEY).unwrap()));
    }

    #[test]
    fn key_mode_needs_a_long_enough_key() {
        let missing = ServeConfig::from_lookup(lookup(&[("ORS_API_KEY", "k"), ("PLAY_INTEGRITY", "key")]))
            .unwrap_err();
        assert_eq!(missing, ConfigError::Missing("REGISTRATION_KEY"));
        let short = ServeConfig::from_lookup(lookup(&[
            ("ORS_API_KEY", "k"),
            ("PLAY_INTEGRITY", "key"),
            ("REGISTRATION_KEY", "short"),
        ]))
        .unwrap_err();
        assert!(matches!(short, ConfigError::Invalid { name: "REGISTRATION_KEY", .. }));
    }

    #[test]
    fn debug_output_hides_the_registration_key() {
        let config = ServeConfig::from_lookup(lookup(&[
            ("ORS_API_KEY", "k"),
            ("PLAY_INTEGRITY", "key"),
            ("REGISTRATION_KEY", KEY),
        ]))
        .unwrap();
        assert!(!format!("{config:?}").contains("owner"));
    }
```

- [ ] **Step 2: Run** `cargo test -p whichride-server --lib config` → FAIL (no `IntegrityMode::Key`).

- [ ] **Step 3: Implement.** `use crate::integrity::RegistrationKey;`. Add to `IntegrityMode`:

```rust
    /// Only builds holding the owner's registration key may register and write.
    Key(RegistrationKey),
```

In `from_lookup`, before `Some("off") if …`:

```rust
            Some("key") => {
                let key = get("REGISTRATION_KEY").ok_or(ConfigError::Missing("REGISTRATION_KEY"))?;
                IntegrityMode::Key(RegistrationKey::new(&key).map_err(|reason| invalid("REGISTRATION_KEY", reason))?)
            }
```

Change the unknown-value message to `format!("expected google, key or off, got {other:?}")`. Check that `ServeConfig`'s `Debug` prints `integrity` with `{:?}` (hidden by Task 1); if it skips the field, leave it.

In `commands.rs` `serve`, add the arm:

```rust
        IntegrityMode::Key(key) => {
            info!("PLAY_INTEGRITY=key: only builds with the registration key can register");
            Integrity::Key(key.clone())
        }
```

(`use tracing::info;` beside `warn`.)

`.env.example`: replace the `PLAY_INTEGRITY` comment and add the key:

```
# google (default), key or off; off is refused on Cloud Run
PLAY_INTEGRITY=off
# Needed when PLAY_INTEGRITY=key: at least 32 bytes, e.g. openssl rand -base64 48
REGISTRATION_KEY=
```

- [ ] **Step 4: Failing end-to-end tests** in `tests/installs.rs` (imports: `whichride_server::integrity::{Integrity, RegistrationKey}`):

```rust
const KEY: &str = "owner-test-key-owner-test-key-owner-test-key";

fn key_mode() -> Integrity {
    Integrity::Key(RegistrationKey::new(KEY).unwrap())
}

#[tokio::test]
async fn key_mode_registers_only_with_the_key() {
    let Some(app) = TestApp::start_with_integrity(key_mode()).await else {
        return;
    };
    let (status, body) = app.post("/v1/installs", &registration()).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
    let (status, _) = app
        .post_with("/v1/installs", &registration(), &[("x-integrity-token", "wrong")])
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, body) = app
        .post_with("/v1/installs", &registration(), &[("x-integrity-token", KEY)])
        .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
}

#[tokio::test]
async fn key_mode_writes_need_the_key_too() {
    let Some(app) = TestApp::start_with_integrity(key_mode()).await else {
        return;
    };
    app.insert_install("bearer-token-of-the-owner-phone").await;
    let report = json!({"offer": "miles.s", "cards": [{
        "label": "Standard", "kind": "standard", "base_fee": 100, "per_km": 30,
        "per_minute": 0, "minimum": 0,
        "km_rounding": "started_unit", "minute_rounding": "started_unit"
    }]});
    let bearer = "Bearer bearer-token-of-the-owner-phone";
    let (status, _) = app.post_with("/v1/reports", &report, &[("authorization", bearer)]).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, body) = app
        .post_with("/v1/reports", &report, &[("authorization", bearer), ("x-integrity-token", KEY)])
        .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
}
```

- [ ] **Step 5: Run** `TEST_POSTGRES_URL=postgres://postgres@localhost:55432/postgres cargo test -p whichride-server` → all PASS (start the `smoke-pg` container first if it is stopped: `docker start smoke-pg`).

- [ ] **Step 6: Commit** `feat(server): PLAY_INTEGRITY=key with REGISTRATION_KEY`.

### Task 3: Server — `revoke-installs` owner command

**Files:**
- Modify: `crates/server/src/owner.rs`, `crates/server/src/cli.rs`, `crates/server/src/main.rs`, `crates/server/src/commands.rs`
- Test: `crates/server/tests/owner.rs`, `cli.rs` tests

**Interfaces:**
- Produces: `owner::revoke_installs(pool: &PgPool) -> anyhow::Result<u64>`; `Command::RevokeInstalls` (`revoke-installs`).

- [ ] **Step 1: Failing tests.** `tests/owner.rs`:

```rust
#[tokio::test]
async fn revoking_installs_refuses_old_tokens_and_keeps_their_history() {
    let Some(app) = TestApp::start().await else {
        return;
    };
    let token = app.register().await;
    let (status, _) = app.post_as(&token, "/v1/reports", &report(30)).await;
    assert_eq!(status, axum::http::StatusCode::CREATED);

    assert_eq!(owner::revoke_installs(&app.pool).await.unwrap(), 1);

    assert!(whichride_server::auth::find_install(&app.pool, &token).await.unwrap().is_none());
    let installs: i64 = sqlx::query_scalar("SELECT count(*) FROM installs").fetch_one(&app.pool).await.unwrap();
    let reports: i64 = sqlx::query_scalar("SELECT count(*) FROM reports").fetch_one(&app.pool).await.unwrap();
    assert_eq!((installs, reports), (1, 1));
    let (status, _) = app.post_as(&token, "/v1/reports", &report(31)).await;
    assert_eq!(status, axum::http::StatusCode::UNAUTHORIZED);
}
```

In `cli.rs` `owner_commands_parse`: `assert_eq!(parse(&["revoke-installs"]).unwrap(), Command::RevokeInstalls);`

- [ ] **Step 2: Run** → FAIL (no `revoke_installs`, no variant).

- [ ] **Step 3: Implement.** `owner.rs`:

```rust
/// Gives every install a token nobody knows (spec 2026-10-21 §6). Rows stay, because reports,
/// votes and price history refer to them; each app registers again on its next 401.
pub async fn revoke_installs(pool: &PgPool) -> anyhow::Result<u64> {
    let revoked = sqlx::query(
        "UPDATE installs SET token_hash = sha256(convert_to(gen_random_uuid()::text || id::text, 'UTF8'))",
    )
    .execute(pool)
    .await?
    .rows_affected();
    Ok(revoked)
}
```

`cli.rs` (after `RevertOffer`):

```rust
    /// Give every install a token nobody knows, so each must register again (switching to Play,
    /// or after rotating REGISTRATION_KEY).
    RevokeInstalls,
```

`commands.rs`:

```rust
pub async fn revoke_installs() -> anyhow::Result<()> {
    let pool = owner_pool().await?;
    let revoked = owner::revoke_installs(&pool).await?;
    println!("{revoked} installs revoked; each registers again on its next request");
    pool.close().await;
    Ok(())
}
```

`main.rs`: `Command::RevokeInstalls => whichride_server::commands::revoke_installs().await,`

- [ ] **Step 4: Run** the server tests (as Task 2 Step 5) → PASS.

- [ ] **Step 5: Commit** `feat(server): revoke-installs owner command`.

### Task 4: Android build — `LocalSettings`, template, summary, registration key setting

**Files:**
- Create: `android/buildSrc/src/main/kotlin/LocalSettings.kt`
- Create: `android/local.properties.example`
- Modify: `android/app/build.gradle.kts` (top settings block, `defaultConfig`, new `checkLocalSettings` task)

**Interfaces:**
- Produces: `class LocalSettings(providers: ProviderFactory, localFile: File)` with `fun get(name: String): String?` and `val names: Set<String>`; `abstract class CheckLocalSettings : DefaultTask` with `names: SetProperty<String>`, `template: RegularFileProperty`; Gradle task `:app:checkLocalSettings`; `BuildConfig.REGISTRATION_KEY: String`.

- [ ] **Step 1: Create `LocalSettings.kt`:**

```kotlin
import java.io.File
import java.util.Properties
import org.gradle.api.DefaultTask
import org.gradle.api.GradleException
import org.gradle.api.file.RegularFileProperty
import org.gradle.api.provider.ProviderFactory
import org.gradle.api.provider.SetProperty
import org.gradle.api.tasks.Input
import org.gradle.api.tasks.InputFile
import org.gradle.api.tasks.TaskAction

/**
 * whichride's per-machine settings (spec 2026-10-21 §3.4): android/local.properties first, then
 * a Gradle property (-P, ~/.gradle/gradle.properties), then WHICHRIDE_<NAME>. Blank means unset.
 */
class LocalSettings(private val providers: ProviderFactory, localFile: File) {
    private val local = Properties().apply { if (localFile.isFile) localFile.inputStream().use(::load) }
    private val asked = sortedSetOf<String>()

    /** Every setting the build read, for [CheckLocalSettings]. */
    val names: Set<String> get() = asked

    fun get(name: String): String? {
        require(name.startsWith("whichride.")) { "whichride settings start with whichride.: $name" }
        asked += name
        val value = local.getProperty(name)
            ?: providers.gradleProperty(name).orNull
            ?: providers.environmentVariable(envName(name)).orNull
        return value?.trim()?.takeIf { it.isNotEmpty() }
    }

    companion object {
        /** whichride.upload.storeFile → WHICHRIDE_UPLOAD_STOREFILE (the release workflow's names). */
        fun envName(name: String) = "WHICHRIDE_" + name.removePrefix("whichride.").replace('.', '_').uppercase()
    }
}

/** Fails when the build reads a setting the template doesn't list, or the template lists one it never reads. */
abstract class CheckLocalSettings : DefaultTask() {
    @get:Input abstract val names: SetProperty<String>
    @get:InputFile abstract val template: RegularFileProperty

    @TaskAction
    fun check() {
        val listed = Properties().apply { template.get().asFile.inputStream().use(::load) }.stringPropertyNames()
        val used = names.get()
        val unlisted = used - listed
        val unused = listed - used
        if (unlisted.isNotEmpty() || unused.isNotEmpty()) {
            throw GradleException(
                "android/local.properties.example is out of date. Add: $unlisted. Remove: $unused.",
            )
        }
    }
}
```

- [ ] **Step 2: Create `android/local.properties.example`:**

```properties
# Copy to android/local.properties (git-ignored) and fill in. Blank = not set.
# Each value can also come from -P / ~/.gradle/gradle.properties or WHICHRIDE_<NAME>.
# Android Studio adds sdk.dir to local.properties by itself.

# The server, e.g. the Cloud Run URL (terraform output service_url). Blank: built-in prices only.
whichride.serverUrl=
# Google Cloud project number (infra/README.md). Blank: no Play Integrity.
whichride.cloudProjectNumber=
# Owner builds before Play (spec 2026-10-21): same value as the registration-key secret.
# Never set it for a Play release.
whichride.registrationKey=

# Play upload key (docs/play/README.md §2). Blank: release builds use the debug key.
whichride.upload.storeFile=
whichride.upload.storePassword=
whichride.upload.keyAlias=
whichride.upload.keyPassword=
# Set to true only by the release workflow.
whichride.requireUploadKey=
```

- [ ] **Step 3: Rewrite the settings block at the top of `android/app/build.gradle.kts`** (everything from the `/** Not secrets…` comment through the `require(uploadStoreFile != null || !requireUploadKey)` block) as:

```kotlin
/** Per-machine settings from android/local.properties (spec 2026-10-21 §3.4); see local.properties.example. */
val settings = LocalSettings(providers, rootProject.file("local.properties"))
val serverUrl = settings.get("whichride.serverUrl") ?: ""
val cloudProjectNumber = settings.get("whichride.cloudProjectNumber") ?: "0"
val registrationKey = settings.get("whichride.registrationKey") ?: ""
val uploadStoreFile = settings.get("whichride.upload.storeFile")
val uploadStorePassword = settings.get("whichride.upload.storePassword")
val uploadKeyAlias = settings.get("whichride.upload.keyAlias")
val uploadKeyPassword = settings.get("whichride.upload.keyPassword")
val requireUploadKey = settings.get("whichride.requireUploadKey").toBoolean()

require(serverUrl.isEmpty() || serverUrl.matches(Regex("""https://[^\s"]+|http://(localhost|10\.0\.2\.2)(:\d+)?"""))) {
    "whichride.serverUrl must be https:// (or http://localhost / http://10.0.2.2 for a local server)"
}
require(cloudProjectNumber.matches(Regex("""\d{1,20}"""))) { "whichride.cloudProjectNumber must be digits" }
require(registrationKey.isEmpty() || registrationKey.matches(Regex("""[A-Za-z0-9+/=_-]{32,}"""))) {
    "whichride.registrationKey must be at least 32 characters of A-Z a-z 0-9 + / = _ -"
}
require(!(requireUploadKey && registrationKey.isNotEmpty())) {
    "A Play release must not contain whichride.registrationKey (spec 2026-10-21 §3.2)"
}
require(uploadStoreFile != null || !requireUploadKey) {
    "No upload key: set whichride.upload.storeFile, storePassword, keyAlias and keyPassword"
}

logger.lifecycle(
    "whichride: " + listOf(
        if (serverUrl.isEmpty()) "no server (built-in prices only)" else "server $serverUrl",
        "attestation: " + when {
            registrationKey.isNotEmpty() -> "registration key"
            cloudProjectNumber == "0" -> "none"
            else -> "Play Integrity"
        },
        "signing: " + if (uploadStoreFile != null) "upload key" else "debug key",
    ).joinToString(" · ") + if (serverUrl.isEmpty()) " · see android/local.properties.example" else "",
)
```

In `defaultConfig`, add after `CLOUD_PROJECT_NUMBER`:

```kotlin
        buildConfigField("String", "REGISTRATION_KEY", "\"$registrationKey\"")
```

In `signingConfigs`, replace the four `requireNotNull(upload(...))` calls with `requireNotNull(uploadStorePassword) { … }`, `requireNotNull(uploadKeyAlias) { … }`, `requireNotNull(uploadKeyPassword) { … }` (messages unchanged). Delete the old `fun upload(name: String)`.

After the `bundledFiles` task, register:

```kotlin
tasks.register<CheckLocalSettings>("checkLocalSettings") {
    names.set(settings.names)
    template.set(rootProject.layout.projectDirectory.file("local.properties.example"))
}
```

- [ ] **Step 4: Verify the template check fails when out of date.** Temporarily delete the `whichride.requireUploadKey=` line from the example, run `./gradlew -q :app:checkLocalSettings` in `android/` → FAIL "Add: [whichride.requireUploadKey]". Restore the line, run again → PASS.

- [ ] **Step 5: Verify the guards and the summary** (in `android/`):
  - `./gradlew -q :app:help` with no `whichride.*` anywhere → prints `whichride: no server (built-in prices only) · attestation: none · signing: debug key · see android/local.properties.example`.
  - `./gradlew -q :app:help -Pwhichride.registrationKey=short` → FAIL "at least 32 characters".
  - `./gradlew -q :app:help -Pwhichride.registrationKey=$(printf 'k%.0s' {1..40}) -Pwhichride.requireUploadKey=true` → FAIL "must not contain whichride.registrationKey".
  - `./gradlew -q :app:help '-Pwhichride.registrationKey=owner-test-key-owner-test-key-owner"x'` → FAIL (character class).

- [ ] **Step 6: Run** `./gradlew --no-daemon :app:testDebugUnitTest :app:assembleRelease` → PASS.

- [ ] **Step 7: Commit** `feat(android): local.properties settings, template check, build summary, registration key setting`.

### Task 5: Android app — `RegistrationKeyTokens` and selection

**Files:**
- Modify: `android/app/src/main/kotlin/io/github/locadani/whichride/server/IntegrityTokens.kt`
- Modify: `android/app/src/main/kotlin/io/github/locadani/whichride/AppContainer.kt:33-35`
- Test: `android/app/src/test/kotlin/io/github/locadani/whichride/server/IntegrityTokensTest.kt`

**Interfaces:**
- Consumes: `BuildConfig.REGISTRATION_KEY` (Task 4).
- Produces: `class RegistrationKeyTokens(key: String) : IntegrityTokens`; `fun chooseIntegrityTokens(registrationKey: String, cloudProjectNumber: Long, play: () -> IntegrityTokens): IntegrityTokens`.

- [ ] **Step 1: Failing test** `IntegrityTokensTest.kt`:

```kotlin
package io.github.locadani.whichride.server

import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class IntegrityTokensTest {
    private val play = IntegrityTokens { "play-token" }
    private val key = "owner-test-key-owner-test-key-owner-test-key"

    @Test fun theKeyWinsOverPlay() = runTest {
        val tokens = chooseIntegrityTokens(key, cloudProjectNumber = 42, play = { play })
        assertTrue(tokens is RegistrationKeyTokens)
        assertEquals(key, tokens.token("hash-1"))
        assertEquals(key, tokens.token("hash-2"))
    }

    @Test fun withoutKeyOrProjectThereIsNoToken() = runTest {
        assertNull(chooseIntegrityTokens("", cloudProjectNumber = 0, play = { play }).token("h"))
    }

    @Test fun withAProjectPlayIsUsed() = runTest {
        assertEquals("play-token", chooseIntegrityTokens("", cloudProjectNumber = 42, play = { play }).token("h"))
    }
}
```

- [ ] **Step 2: Run** `./gradlew -q :app:testDebugUnitTest --tests '*IntegrityTokensTest'` → FAIL (unresolved references).

- [ ] **Step 3: Implement** in `IntegrityTokens.kt`:

```kotlin
/** The owner's registration key for every request, before the app is on Play (spec 2026-10-21 §3.2). */
class RegistrationKeyTokens(private val key: String) : IntegrityTokens {
    override suspend fun token(requestHash: String): String = key
}

/** Build-time choice: the registration key if this build has one, else Play when a Cloud project is set. */
fun chooseIntegrityTokens(registrationKey: String, cloudProjectNumber: Long, play: () -> IntegrityTokens): IntegrityTokens =
    when {
        registrationKey.isNotEmpty() -> RegistrationKeyTokens(registrationKey)
        cloudProjectNumber == 0L -> IntegrityTokens { null }
        else -> play()
    }
```

`AppContainer.kt`:

```kotlin
    private val integrity: IntegrityTokens =
        chooseIntegrityTokens(BuildConfig.REGISTRATION_KEY, BuildConfig.CLOUD_PROJECT_NUMBER) {
            PlayIntegrityTokens(app, BuildConfig.CLOUD_PROJECT_NUMBER)
        }
```

(Import `chooseIntegrityTokens`.)

- [ ] **Step 4: Run** `./gradlew --no-daemon :app:testDebugUnitTest :app:lintDebug` → PASS.

- [ ] **Step 5: Commit** `feat(android): registration key attestation, chosen at build time`.

### Task 6: CI and release guards

**Files:**
- Modify: `.github/workflows/ci.yml` (android job)
- Modify: `.github/workflows/release.yml`

- [ ] **Step 1: CI.** In the android job, before `setup-gradle`, add:

```yaml
      - name: local.properties is never committed
        run: |
          if git ls-files --error-unmatch local.properties > /dev/null 2>&1; then
            echo "::error::android/local.properties is tracked by git; remove it with git rm --cached"
            exit 1
          fi
```

Add `:app:checkLocalSettings` to the Gradle command after `:app:lintDebug`.

- [ ] **Step 2: Release.** In `release.yml`, extend the "Server settings" step's `run` with:

```bash
          if [[ -n "${WHICHRIDE_REGISTRATIONKEY:-}" ]]; then
            echo "::error::A Play release must not contain a registration key"
            exit 1
          fi
```

(Gradle already refuses it with `requireUploadKey`; this fails before the build starts.)

- [ ] **Step 3: Check** with `actionlint` if installed (`actionlint .github/workflows/ci.yml .github/workflows/release.yml`), else review the YAML indentation by eye.

- [ ] **Step 4: Commit** `ci: local.properties never tracked, template check, no key in releases`.

### Task 7: Terraform — attach secrets by name, create `registration-key`

**Files:**
- Modify: `infra/modules/cloud-run-app/variables.tf`, `infra/modules/cloud-run-app/service.tf:50-52`
- Modify: `infra/modules/cloud-run-app/tests/cloud_run_app.tftest.hcl` (two runs at lines 341-371)
- Modify: `infra/prod/main.tf`

**Interfaces:**
- Produces: module variable `attached_secrets: set(string)` replacing `attach_secret_env: bool`.

- [ ] **Step 1: Update the tests** (they fail first): rename the second run and add a partial-attach run:

```hcl
run "attached_secret_env_reaches_the_container" {
  command = plan

  variables {
    secret_env       = { ORS_API_KEY = "ors-api-key" }
    attached_secrets = ["ORS_API_KEY"]
  }

  assert {
    condition     = length([for e in google_cloud_run_v2_service.app.template[0].containers[0].env : e if e.name == "ORS_API_KEY"]) == 1
    error_message = "ORS_API_KEY comes from its secret"
  }
}

run "only_named_secrets_are_attached" {
  command = plan

  variables {
    secret_env       = { ORS_API_KEY = "ors-api-key", REGISTRATION_KEY = "registration-key" }
    attached_secrets = ["ORS_API_KEY"]
  }

  assert {
    condition     = google_secret_manager_secret.extra["REGISTRATION_KEY"].secret_id == "${var.name}-registration-key"
    error_message = "every secret is created"
  }

  assert {
    condition     = length([for e in google_cloud_run_v2_service.app.template[0].containers[0].env : e if e.name == "REGISTRATION_KEY"]) == 0
    error_message = "a secret not in attached_secrets is not attached"
  }
}

run "an_attached_secret_must_exist" {
  command = plan

  variables {
    secret_env       = { ORS_API_KEY = "ors-api-key" }
    attached_secrets = ["MISSING"]
  }

  expect_failures = [var.attached_secrets]
}
```

In `secret_env_creates_secrets_without_attaching_them`, change the error message to `"secrets are attached only when named in attached_secrets"`.

- [ ] **Step 2: Run** `terraform -chdir=infra/modules/cloud-run-app init -backend=false && terraform -chdir=infra/modules/cloud-run-app test` → FAIL (unknown variable `attached_secrets`). Terraform is not installed on the owner's laptop: install it (`brew install hashicorp/tap/terraform`, 1.16.x as pinned in `infra/prod/versions.tf`) or rely on the infra workflow's "Format, validate, test, scan" job on the PR. Cross-variable `validation` needs Terraform ≥ 1.9; 1.16 is pinned.

- [ ] **Step 3: Implement.** `variables.tf`, replace `attach_secret_env`:

```hcl
variable "attached_secrets" {
  description = "Names from secret_env to give the service. Add a name only after its secret has a version: Cloud Run won't start an instance whose secret can't be read."
  type        = set(string)
  default     = []

  validation {
    condition     = alltrue([for name in var.attached_secrets : contains(keys(var.secret_env), name)])
    error_message = "Every attached secret must be listed in secret_env."
  }
}
```

`service.tf`: `for_each = { for name, secret in var.secret_env : name => secret if contains(var.attached_secrets, name) }`.

`infra/prod/main.tf`:

```hcl
  secret_env = {
    ORS_API_KEY      = "ors-api-key"
    REGISTRATION_KEY = "registration-key"
  }

  # Attach a secret only once it has a version (infra/README.md steps 8 and 11).
  attach_database_url = true
  attached_secrets    = ["ORS_API_KEY"]
```

- [ ] **Step 4: Run** the module tests → PASS. `terraform -chdir=infra/prod init -backend=false && terraform -chdir=infra/prod validate` → valid. `terraform fmt -check -recursive infra` → clean.

- [ ] **Step 5: Commit** `feat(infra): attach secrets by name; registration-key secret (not attached yet)`.

### Task 8: Docs

**Files:**
- Modify: `docs/DECISIONS.md` (row 77), `docs/specs/2026-10-21-registration-key-design.md` (status → Approved), `infra/README.md` (step 11), `docs/play/README.md` (§2 paths, §5 switch), `android/README.md` (settings section, if it mentions `~/.gradle/gradle.properties`), `docs/plans/ROADMAP.md` (row 6a)

- [ ] **Step 1: Decision 77** in `docs/DECISIONS.md`:

```markdown
| 77 | 2026-10-21 | Integrity | Before Play, the server runs `PLAY_INTEGRITY=key`: a secret registration key (Secret Manager, ≥ 32 bytes) in `X-Integrity-Token` admits registration and writes; the app sends it when built with `whichride.registrationKey`; local settings live in `android/local.properties` with a committed template. Back to Play by configuration (spec 2026-10-21). | `PLAY_INTEGRITY=off` on Cloud Run; paying for Play now; Firebase App Check; Secrets Gradle Plugin | The owner is the only user for now; the API stays closed; the switch to Play needs no code |
```

- [ ] **Step 2: `infra/README.md` step 11** becomes:

```markdown
11. **Integrity.** Until the app is on Play the server runs `PLAY_INTEGRITY=key`
    (`docs/specs/2026-10-21-registration-key-design.md`):
    1. After the PR that creates the `whichride-registration-key` secret is applied, store the key:
       `openssl rand -base64 48 | tr -d '\n' | gcloud secrets versions add whichride-registration-key --data-file=-`
    2. Put the same value in `android/local.properties` as `whichride.registrationKey=…`
       (`gcloud secrets versions access latest --secret=whichride-registration-key`).
    3. Open the follow-up PR: `attached_secrets = ["ORS_API_KEY", "REGISTRATION_KEY"]` and
       `PLAY_INTEGRITY = "key"`. Apply.
    Moving to Play: `docs/play/README.md` §5.
```

(Check the secret ID: the module names it `${var.name}-${suffix}` = `whichride-registration-key`; the spec's `registration-key` is the suffix. Use the full ID in commands.)

- [ ] **Step 3: `docs/play/README.md`.** In §2 replace `~/.gradle/gradle.properties` with `android/local.properties` (copy `android/local.properties.example`). §5 becomes: link Play Integrity in Play Console, then remove `whichride.registrationKey`, set `PLAY_INTEGRITY = "google"` and drop `REGISTRATION_KEY` from `attached_secrets`, apply, and run `revoke-installs` from Cloud Shell like `lock-offer`.

- [ ] **Step 4:** Spec header `**Status:** Approved 2026-10-21`; the spec's Cloud Shell command uses `whichride-registration-key`. ROADMAP: add a row `| 6a | **Registration key** | Owner-only server access before Play (decision 77) | — | \`06a-registration-key.md\` |` after row 6.

- [ ] **Step 5: Commit** `docs: decision 77, registration key steps`.

### Task 9: Whole-branch check and push

- [ ] **Step 1:** From the repo root: `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && TEST_POSTGRES_URL=postgres://postgres@localhost:55432/postgres cargo test --workspace` → PASS.
- [ ] **Step 2:** `android/`: the CI Gradle command plus `:app:checkLocalSettings` → PASS. `design/`: `npm test` → PASS.
- [ ] **Step 3:** `git push -u origin feat/registration-key`; the owner opens PR 1.

## After PR 1 (owner, then PR 2)

1. Owner stores the key (infra/README.md step 11.1–11.2) and moves `whichride.serverUrl` and `whichride.cloudProjectNumber` from `~/.gradle/gradle.properties` to `android/local.properties`.
2. PR 2 (small, `infra/prod/main.tf` only): `PLAY_INTEGRITY = "key"`, `attached_secrets = ["ORS_API_KEY", "REGISTRATION_KEY"]`.
3. Phone: `./gradlew :app:installDebug`; check current location, address search and a price report.
