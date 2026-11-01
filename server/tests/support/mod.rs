//! Common isolated application, authenticated HTTP client, and durable-job fixtures.

/// Complete multipart, publication, and private-preview arrangements.
pub mod media;
/// Bounded in-process S3 and CloudFront protocols.
pub mod s3;
use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use mori::{
    app::{AppState, Config},
    http,
    infra::storage::Storage,
};
pub use s3::Fixture;
use serde_json::{Value, json};
use tower::ServiceExt;

/// Parsed HTTP result used by contract assertions.
pub struct Reply {
    /// Actual application status.
    pub status: StatusCode,
    /// Security, redirect, and session headers.
    pub headers: axum::http::HeaderMap,
    /// JSON document, or null for bodyless and media responses.
    pub body: Value,
}
/// Session-aware caller of the public HTTP interface, scoped to one test router.
pub struct Client<'a> {
    /// Application under test.
    router: &'a Router,
    /// Optional authenticated session token; never logged.
    cookie: Option<String>,
    /// Mutation protection for this session.
    csrf: Option<String>,
}
impl<'a> Client<'a> {
    /// Authenticate through the real endpoint, rather than inserting a session into SQLite.
    pub async fn admin(router: &'a Router) -> Self {
        let (cookie, csrf) = login(router).await;
        Self {
            router,
            cookie: Some(cookie),
            csrf: Some(csrf),
        }
    }
    /// Create a caller with no authentication cookie or mutation credentials.
    pub const fn visitor(router: &'a Router) -> Self {
        Self {
            router,
            cookie: None,
            csrf: None,
        }
    }
    /// Send a JSON operation and retain its observable HTTP result.
    pub async fn request(&self, method: &str, path: &str, body: Value) -> Reply {
        let (status, headers, body) = call(
            self.router,
            method,
            path,
            self.cookie.as_deref(),
            self.csrf.as_deref(),
            body,
        )
        .await;
        Reply {
            status,
            headers,
            body,
        }
    }
}

/// Consistent fake credentials used only against loopback protocol fixtures.
pub fn storage_settings() -> mori::infra::storage_settings::config::StorageSettings {
    mori::infra::storage_settings::config::StorageSettings {
        bucket: "test-bucket".into(),
        region: "us-east-1".into(),
        distribution_id: Some("test".into()),
        access_key_id: "testing".into(),
        secret_access_key: "testing".into(),
        session_token: None,
    }
}
/// Create a real initialized application with a disposable database and static document.
pub async fn setup() -> (AppState, Router, tempfile::TempDir) {
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(
        directory.path().join("index.html"),
        "<!doctype html><html><head><title>Mori</title></head><body></body></html>",
    )
    .unwrap();
    let config = Config {
        listen: "127.0.0.1:0".into(),
        database_url: format!("sqlite://{}", directory.path().join("mori.db").display()),
        public_url: "http://localhost:3000".into(),
        s3_endpoint: Some("http://127.0.0.1:9".into()),
        cloudfront_endpoint: None,
        web_dir: directory.path().display().to_string(),
        max_upload_bytes: 1_000_000_000,
        admin_username: "root".into(),
        admin_password: Some("a-long-test-password".into()),
        video_extensions: vec!["mp4".into(), "webm".into(), "mov".into()],
        image_extensions: vec![
            "jpg".into(),
            "jpeg".into(),
            "png".into(),
            "gif".into(),
            "webp".into(),
            "svg".into(),
            "heic".into(),
            "dng".into(),
        ],
        origin_secret_file: None,
    };
    let state = AppState::new(config).await.unwrap();
    state
        .storage
        .install(Storage::new(&storage_settings(), &state.config).unwrap());
    let router = http::router(state.clone());
    (state, router, directory)
}
/// Start a fixture with an already-bound listener and independent state.
pub async fn s3_fixture() -> (Fixture, String) {
    let fixture = Fixture::start().await;
    let endpoint = fixture.endpoint.clone();
    (fixture, endpoint)
}

/// Exercise an actual application route with explicit session and CSRF inputs.
pub async fn call(
    router: &Router,
    method: &str,
    path: &str,
    cookie: Option<&str>,
    csrf: Option<&str>,
    body: Value,
) -> (StatusCode, axum::http::HeaderMap, Value) {
    let mut request = Request::builder()
        .method(method)
        .uri(path)
        .header("origin", "http://localhost:3000")
        .header("content-type", "application/json");
    if let Some(cookie) = cookie {
        request = request.header("cookie", cookie);
    }
    if let Some(csrf) = csrf {
        request = request.header("x-csrf-token", csrf);
    }
    let response = router
        .clone()
        .oneshot(request.body(Body::from(body.to_string())).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        headers,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

/// Verify the administrator password and establish a revocable session.
pub async fn login(router: &Router) -> (String, String) {
    let (status, headers, body) = call(
        router,
        "POST",
        "/v1/auth/login",
        None,
        None,
        json!({"password":"a-long-test-password"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    (
        headers["set-cookie"]
            .to_str()
            .unwrap()
            .split(';')
            .next()
            .unwrap()
            .into(),
        body["csrf"].as_str().unwrap().into(),
    )
}

/// Wait for a durable task while accelerating only its test retry timestamps.
pub async fn wait_job(s: &AppState, id: &str) -> String {
    for _ in 0..150 {
        let state: String = sqlx::query_scalar("SELECT state FROM jobs WHERE id=?")
            .bind(id)
            .fetch_one(&s.db)
            .await
            .unwrap();
        if state != "pending" {
            return state;
        }
        sqlx::query("UPDATE jobs SET next_at=0 WHERE id=?")
            .bind(id)
            .execute(&s.db)
            .await
            .unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }
    let error: Option<String> = sqlx::query_scalar("SELECT error FROM jobs WHERE id=?")
        .bind(id)
        .fetch_one(&s.db)
        .await
        .unwrap();
    panic!("job timed out: {error:?}")
}

/// Seed a collection with both public and private videos without a storage side effect.
pub async fn lifecycle_collection(state: &AppState) {
    sqlx::query("INSERT INTO albums(id,name,slug,visibility,created_at) VALUES('collection','Nature','nature','public',1),('other','Other','other','private',2)")
        .execute(&state.db).await.unwrap();
    for (id, token, visibility, delivery) in [
        ("a", "a", "public", "published"),
        ("b", "b", "private", "ready"),
    ] {
        sqlx::query("INSERT INTO videos(id,token,album_id,name,visibility,state,size,created_at,media_prefix) VALUES(?,?,'collection',?,?,?,100,1,'nature')")
            .bind(id).bind(token.repeat(32)).bind(format!("{visibility} title")).bind(visibility).bind(delivery).execute(&state.db).await.unwrap();
        sqlx::query("INSERT INTO media_aliases(video_id,prefix) VALUES(?,'nature')")
            .bind(id)
            .execute(&state.db)
            .await
            .unwrap();
    }
}
