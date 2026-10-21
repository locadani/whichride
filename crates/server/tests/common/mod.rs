//! Shared by the HTTP tests: a fresh database per test on `TEST_POSTGRES_URL`, the app wired
//! to it with a fixed clock, and request helpers.
#![allow(dead_code)] // each test file uses a different subset

use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicI32, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use carsharing_core::{Catalogue, Day};
use serde_json::Value;
use sqlx::{AssertSqlSafe, Connection, PgConnection, PgPool};
use tower::ServiceExt;
use uuid::Uuid;
use whichride_server::app::router;
use whichride_server::db;
use whichride_server::expand::Expander;
use whichride_server::integrity::{GoogleIntegrity, Integrity};
use whichride_server::limits::{Kind, RateLimits};
use whichride_server::ors::Ors;
use whichride_server::state::AppState;
use wiremock::matchers::{body_json, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// 2026-10-19.
pub const TODAY: i32 = 20_745;

static NEXT: AtomicU32 = AtomicU32::new(0);

/// A database that lives as long as this value; dropping it drops the database.
pub struct TestDatabase {
    pub url: String,
    admin_url: String,
    name: String,
}

impl Drop for TestDatabase {
    /// Runs on its own thread and runtime: the test's runtime may be the one dropping this
    /// value. A failure is reported, never a panic inside `drop`.
    fn drop(&mut self) {
        let admin_url = self.admin_url.clone();
        let sql = format!("DROP DATABASE IF EXISTS {} WITH (FORCE)", self.name);
        let dropped = std::thread::spawn(move || -> Result<(), String> {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| e.to_string())?;
            runtime.block_on(async {
                let mut admin = PgConnection::connect(&admin_url)
                    .await
                    .map_err(|e| e.to_string())?;
                sqlx::raw_sql(AssertSqlSafe(sql))
                    .execute(&mut admin)
                    .await
                    .map_err(|e| e.to_string())?;
                admin.close().await.map_err(|e| e.to_string())
            })
        })
        .join()
        .unwrap_or_else(|_| Err("the thread dropping it panicked".to_owned()));
        if let Err(error) = dropped {
            eprintln!("could not drop test database {}: {error}", self.name);
        }
    }
}

/// A new, empty database, or `None` outside CI when `TEST_POSTGRES_URL` is not set.
/// The URL must end with a database name (e.g. `/postgres`) and have no query string.
pub async fn fresh_database() -> Option<TestDatabase> {
    let Ok(admin_url) = std::env::var("TEST_POSTGRES_URL") else {
        assert!(
            std::env::var_os("CI").is_none(),
            "TEST_POSTGRES_URL must be set in CI"
        );
        eprintln!("skipped: set TEST_POSTGRES_URL to run the Postgres tests");
        return None;
    };
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after 1970")
        .as_micros();
    let name = format!(
        "whichride_test_{}_{stamp}_{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    );
    let mut admin = PgConnection::connect(&admin_url)
        .await
        .expect("connect to TEST_POSTGRES_URL");
    sqlx::raw_sql(AssertSqlSafe(format!("CREATE DATABASE {name}")))
        .execute(&mut admin)
        .await
        .expect("create a test database");
    admin.close().await.expect("close the admin connection");
    let (server, _) = admin_url
        .rsplit_once('/')
        .expect("TEST_POSTGRES_URL ends with /<database>");
    Some(TestDatabase {
        url: format!("{server}/{name}"),
        admin_url,
        name,
    })
}

pub fn fixture_catalogue() -> Catalogue {
    Catalogue::from_json(include_str!("catalogue.json")).expect("valid fixture")
}

/// The smallest built site: two language pages, the 404 page and a hashed asset with its gzip.
pub fn fake_site() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("whichride-site-{}", Uuid::new_v4()));
    for (path, body) in [
        ("en/index.html", "<h1>en</h1>"),
        ("it/index.html", "<h1>it</h1>"),
        ("404.html", "<h1>missing</h1>"),
        ("_astro/app.1a2b3c.js", "console.log(1)"),
        ("_astro/app.1a2b3c.js.gz", "gzipped"),
    ] {
        let file = dir.join(path);
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(file, body).unwrap();
    }
    dir
}

pub struct Options {
    pub integrity: Integrity,
    /// One hourly limit for every kind.
    pub per_hour: Option<u32>,
    /// Hourly limits of single kinds.
    pub limits: Vec<(Kind, u32)>,
    /// A built website to serve next to the API (see [`fake_site`]).
    pub site: Option<PathBuf>,
}

impl Default for Options {
    fn default() -> Options {
        Options {
            integrity: Integrity::Off,
            per_hour: None,
            limits: Vec::new(),
            site: None,
        }
    }
}

pub struct TestApp {
    pub router: Router,
    pub pool: PgPool,
    /// The fake OpenRouteService.
    pub upstream: MockServer,
    today: Arc<AtomicI32>,
    // Last, so the pool closes before the database is dropped.
    _db: TestDatabase,
}

impl TestApp {
    /// The app on a fresh, migrated and seeded database, with today = [`TODAY`].
    pub async fn start() -> Option<TestApp> {
        TestApp::start_with_integrity(Integrity::Off).await
    }

    /// As [`TestApp::start`], with the given integrity verification.
    pub async fn start_with_integrity(integrity: Integrity) -> Option<TestApp> {
        TestApp::start_with(Options {
            integrity,
            ..Options::default()
        })
        .await
    }

    pub async fn start_with(options: Options) -> Option<TestApp> {
        let database = fresh_database().await?;
        let pool = db::connect(&database.url).await.expect("connect");
        let catalogue = fixture_catalogue();
        db::prepare(&pool, &catalogue, Day(TODAY))
            .await
            .expect("prepare");
        let today = Arc::new(AtomicI32::new(TODAY));
        let clock = Arc::clone(&today);
        let upstream = MockServer::start().await;
        let ors_base = url::Url::parse(&format!("{}/", upstream.uri())).unwrap();
        let state = AppState {
            today: Arc::new(move || Day(clock.load(Ordering::Relaxed))),
            db: pool.clone(),
            providers: Arc::new(catalogue),
            integrity: Arc::new(options.integrity),
            ors: Arc::new(
                Ors::new(ors_base, "test-key".into()).with_timeout(Duration::from_millis(500)),
            ),
            limits: Arc::new(
                options.limits.iter().fold(
                    options
                        .per_hour
                        .map_or_else(RateLimits::new, RateLimits::with_per_hour),
                    |limits, &(kind, per_hour)| limits.with_limit(kind, per_hour),
                ),
            ),
            expander: Arc::new(Expander::with_origin(
                url::Url::parse(&upstream.uri()).unwrap(),
            )),
        };
        Some(TestApp {
            router: router(
                state,
                options.site.as_deref().map(whichride_server::site::router),
            ),
            pool,
            upstream,
            today,
            _db: database,
        })
    }

    pub fn set_today(&self, day: i32) {
        self.today.store(day, Ordering::Relaxed);
    }

    pub async fn get(&self, path: &str) -> (StatusCode, Value) {
        self.send(Request::get(path).body(Body::empty()).unwrap())
            .await
    }

    pub async fn post(&self, path: &str, body: &Value) -> (StatusCode, Value) {
        self.post_with(path, body, &[]).await
    }

    pub async fn post_with(
        &self,
        path: &str,
        body: &Value,
        headers: &[(&str, &str)],
    ) -> (StatusCode, Value) {
        let mut request = Request::post(path).header("content-type", "application/json");
        for (name, value) in headers {
            request = request.header(*name, *value);
        }
        self.send(request.body(Body::from(body.to_string())).unwrap())
            .await
    }

    /// Registers a new install and returns its bearer token.
    pub async fn register(&self) -> String {
        let (status, body) = self.post("/v1/installs", &registration()).await;
        assert_eq!(status, StatusCode::CREATED, "{body}");
        body["token"].as_str().unwrap().to_owned()
    }

    /// Adds an install straight to the database (for apps that check integrity with Google),
    /// with this bearer token; returns its ID.
    pub async fn insert_install(&self, token: &str) -> Uuid {
        let id = Uuid::new_v4();
        sqlx::query("INSERT INTO installs (id, token_hash, first_seen) VALUES ($1, $2, $3)")
            .bind(id)
            .bind(whichride_server::auth::hash_token(token).as_slice())
            .bind(TODAY)
            .execute(&self.pool)
            .await
            .unwrap();
        id
    }

    pub async fn post_as(&self, token: &str, path: &str, body: &Value) -> (StatusCode, Value) {
        let bearer = format!("Bearer {token}");
        self.post_with(path, body, &[("authorization", bearer.as_str())])
            .await
    }

    /// Gives an install a history, so its votes weigh more (or less) than a new install's.
    pub async fn make_trusted(&self, token: &str, decided: i32, matching: i32, age_days: i32) {
        sqlx::query(
            "UPDATE installs SET decided_votes = $2, matching_votes = $3, first_seen = $4
             WHERE token_hash = $1",
        )
        .bind(whichride_server::auth::hash_token(token).as_slice())
        .bind(decided)
        .bind(matching)
        .bind(TODAY - age_days)
        .execute(&self.pool)
        .await
        .unwrap();
    }

    pub async fn send(&self, request: Request<Body>) -> (StatusCode, Value) {
        let response = self.router.clone().oneshot(request).await.unwrap();
        let status = response.status();
        let bytes = to_bytes(response.into_body(), 1 << 20).await.unwrap();
        let json = if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes)
                .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&bytes).into_owned()))
        };
        (status, json)
    }
}

/// A registration body with a fresh nonce.
pub fn registration() -> Value {
    serde_json::json!({"nonce": Uuid::new_v4().simple().to_string()})
}

/// Something shaped like a Play Integrity token (JWE compact: five base64url parts).
pub fn integrity_token(name: &str) -> String {
    let name = URL_SAFE_NO_PAD.encode(name);
    format!("eyJhbGciOiJBMjU2S1cifQ.{name}.aXY.Y2lwaGVydGV4dA.dGFn")
}

/// Google's integrity service on `server`: the metadata server answers, and no token decodes
/// until [`mock_verdict`] says what it means.
pub async fn google_integrity(server: &MockServer) -> Integrity {
    Mock::given(method("GET"))
        .and(path(
            "/computeMetadata/v1/instance/service-accounts/default/token",
        ))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(
                serde_json::json!({"access_token": "ya29.test", "expires_in": 3599}),
            ),
        )
        .mount(server)
        .await;
    let base = url::Url::parse(&format!("{}/", server.uri())).unwrap();
    Integrity::Google(GoogleIntegrity::with_endpoints(
        "app.whichride".into(),
        base.clone(),
        base,
    ))
}

/// Google decodes `token` to a good verdict for the request with this hash.
pub async fn mock_verdict(server: &MockServer, token: &str, request_hash: &str) {
    Mock::given(method("POST"))
        .and(path("/v1/app.whichride:decodeIntegrityToken"))
        .and(body_json(serde_json::json!({"integrityToken": token})))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "tokenPayloadExternal": {
                "requestDetails": {
                    "requestPackageName": "app.whichride",
                    "requestHash": request_hash,
                    "timestampMillis": whichride_server::integrity::now_millis().to_string()
                },
                "appIntegrity": {"appRecognitionVerdict": "PLAY_RECOGNIZED"},
                "deviceIntegrity": {"deviceRecognitionVerdict": ["MEETS_DEVICE_INTEGRITY"]}
            }
        })))
        .mount(server)
        .await;
}

thread_local! {
    static CAPTURE: std::cell::RefCell<Option<Arc<Mutex<Vec<u8>>>>> =
        const { std::cell::RefCell::new(None) };
}

/// Stops capturing this thread's logs when dropped.
pub struct CaptureGuard;

impl Drop for CaptureGuard {
    fn drop(&mut self) {
        CAPTURE.with(|capture| capture.borrow_mut().take());
    }
}

/// Writes to the capture buffer of the current thread, if any.
struct ThreadSink;

impl Write for ThreadSink {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        CAPTURE.with(|capture| {
            if let Some(buffer) = capture.borrow().as_ref() {
                buffer.lock().unwrap().extend_from_slice(bytes);
            }
        });
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Everything logged on this thread while the guard lives, at `trace` for the server's own
/// targets and tower-http. One global subscriber serves every test: per-thread subscribers race
/// with tracing's global callsite cache and sometimes lose the request span.
pub fn capture_logs() -> (Arc<Mutex<Vec<u8>>>, CaptureGuard) {
    static INIT: std::sync::Once = std::sync::Once::new();
    INIT.call_once(|| {
        let subscriber = tracing_subscriber::fmt()
            .with_env_filter("whichride_server=trace,tower_http=trace")
            // A timestamp such as 15:07:13.413 would match the coordinate fragment "13.41".
            .without_time()
            .with_ansi(false)
            .with_writer(|| ThreadSink)
            .finish();
        tracing::subscriber::set_global_default(subscriber).expect("no other global subscriber");
    });
    let buffer = Arc::new(Mutex::new(Vec::new()));
    CAPTURE.with(|capture| *capture.borrow_mut() = Some(Arc::clone(&buffer)));
    (buffer, CaptureGuard)
}
