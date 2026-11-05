//! Shared upload fixture drives HTTP and AWS SDK interfaces with isolated durable storage.

use super::{Client, Fixture, Reply, login, setup, storage_settings, wait_job};
use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
    response::Response,
};
use base64::{Engine, engine::general_purpose::STANDARD};
use mori::{
    app::AppState,
    features::videos::{self, Video},
    http,
    infra::storage::Storage,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use tokio::task::JoinHandle;
use tower::ServiceExt;

/// A complete upload environment; tests arrange only the behavior they need.
pub struct Media {
    /// Real application resources and durable job records.
    pub state: AppState,
    /// Real HTTP entry points under test.
    pub router: Router,
    /// Authentication obtained from the login endpoint.
    cookie: String,
    /// Mutation token paired with this session.
    csrf: String,
    /// Worker whose restart can be tested independently of object storage.
    worker: JoinHandle<()>,
    /// Protocol state never shared between tests.
    fixture: Fixture,
    /// Metadata and vault key removed after the test.
    _directory: tempfile::TempDir,
}
impl Media {
    /// Suspend the next object write to exercise concurrent upload transitions.
    pub fn pause_put(&self) -> super::s3::PausedPut {
        self.fixture.pause_put()
    }
    /// Configure either S3-only or optional CDN delivery using local SDK endpoints.
    pub async fn new(cdn: bool) -> Self {
        let (mut state, _, directory) = setup().await;
        let fixture = Fixture::start().await;
        let mut config = (*state.config).clone();
        config.s3_endpoint = Some(fixture.endpoint.clone());
        config.cloudfront_endpoint = Some(fixture.endpoint.clone());
        state.config = Arc::new(config);
        let mut settings = storage_settings();
        if !cdn {
            settings.distribution_id = None;
        }
        state.storage.persist(&state.db, &settings).await.unwrap();
        state
            .storage
            .install(Storage::new(&settings, &state.config).unwrap());
        let router = http::router(state.clone());
        let (cookie, csrf) = login(&router).await;
        let worker = tokio::spawn(mori::jobs::run(state.clone()));
        Self {
            state,
            router,
            cookie,
            csrf,
            worker,
            fixture,
            _directory: directory,
        }
    }
    /// Send one authenticated JSON operation through the public HTTP interface.
    pub async fn request(&self, method: &str, path: &str, body: Value) -> Reply {
        Client {
            router: &self.router,
            cookie: Some(self.cookie.clone()),
            csrf: Some(self.csrf.clone()),
        }
        .request(method, path, body)
        .await
    }
    /// Create a resumable upload with browser-supplied presentation metadata.
    pub async fn begin(&self, visibility: &str, size: usize, extension: &str) -> Value {
        let reply=self.request("POST","/v1/admin/uploads",json!({"size":size,"extension":extension,"visibility":visibility,"width":32,"height":24,"duration":1.0})).await;
        assert_eq!(reply.status, StatusCode::CREATED);
        reply.body
    }
    /// Sign and send one checksummed fixture part with the real AWS SDK.
    pub async fn part(&self, upload: &Value, bytes: &[u8]) {
        let id = upload["id"].as_str().unwrap();
        let checksum = STANDARD.encode(Sha256::digest(bytes));
        let signed = self
            .request(
                "POST",
                &format!("/v1/admin/uploads/{id}/parts/1"),
                json!({"checksum":checksum}),
            )
            .await;
        assert_eq!(signed.status, StatusCode::OK);
        assert!(
            signed.body["url"]
                .as_str()
                .unwrap()
                .contains("X-Amz-Signature")
        );
        let upload_id: String = sqlx::query_scalar("SELECT upload_id FROM uploads WHERE id=?")
            .bind(id)
            .fetch_one(&self.state.db)
            .await
            .unwrap();
        let storage = self.state.storage.get().unwrap();
        let video = videos::get(&self.state, upload["video_id"].as_str().unwrap())
            .await
            .unwrap();
        storage
            .s3
            .upload_part()
            .bucket(&storage.bucket)
            .key(video.private_source())
            .upload_id(upload_id)
            .part_number(1)
            .checksum_sha256(checksum)
            .body(aws_sdk_s3::primitives::ByteStream::from(bytes.to_vec()))
            .send()
            .await
            .unwrap();
    }
    /// Store a minimal JPEG-shaped fixture for publication and preview concurrency checks.
    pub async fn poster(&self, id: &str) {
        let response = self
            .router
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(format!("/v1/admin/videos/{id}/poster"))
                    .header("cookie", &self.cookie)
                    .header("origin", "http://localhost:3000")
                    .header("x-csrf-token", &self.csrf)
                    .header("content-type", "image/jpeg")
                    .body(Body::from(vec![255, 216, 255, 217]))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
    }
    /// Complete browser transfer and return the persisted verification job.
    pub async fn complete(&self, upload: &Value) -> String {
        let reply = self
            .request(
                "POST",
                &format!(
                    "/v1/admin/uploads/{}/complete",
                    upload["id"].as_str().unwrap()
                ),
                Value::Null,
            )
            .await;
        assert_eq!(reply.status, StatusCode::ACCEPTED);
        reply.body["job_id"].as_str().unwrap().into()
    }
    /// Arrange a verified private video through the entire upload interface.
    pub async fn ready(&self) -> Video {
        let bytes = include_bytes!("../fixtures/short.webm");
        let upload = self.begin("private", bytes.len(), "webm").await;
        self.part(&upload, bytes).await;
        self.poster(upload["video_id"].as_str().unwrap()).await;
        let job = self.complete(&upload).await;
        assert_eq!(wait_job(&self.state, &job).await, "complete");
        videos::get(&self.state, upload["video_id"].as_str().unwrap())
            .await
            .unwrap()
    }
    /// Change visibility and wait for this operation, including CDN withdrawal when enabled.
    pub async fn visibility(&self, video: &Video, visibility: &str) -> Video {
        let reply = self
            .request(
                "PATCH",
                &format!("/v1/admin/videos/{}", video.id),
                json!({"name":video.name,"album_id":video.album_id,"visibility":visibility}),
            )
            .await;
        assert_eq!(reply.status, StatusCode::ACCEPTED);
        if let Some(job) = reply.body["job_id"].as_str() {
            assert_eq!(wait_job(&self.state, job).await, "complete");
        }
        videos::get(&self.state, &video.id).await.unwrap()
    }
    /// Inspect a fixture object through the actual storage SDK.
    pub async fn exists(&self, key: String) -> bool {
        let storage = self.state.storage.get().unwrap();
        storage
            .s3
            .head_object()
            .bucket(&storage.bucket)
            .key(key)
            .send()
            .await
            .is_ok()
    }
    /// Request authenticated streaming without collecting or releasing its body early.
    pub async fn content(&self, path: &str, range: Option<&str>) -> Response {
        let mut request = Request::builder().uri(path).header("cookie", &self.cookie);
        if let Some(range) = range {
            request = request.header("range", range);
        }
        self.router
            .clone()
            .oneshot(request.body(Body::empty()).unwrap())
            .await
            .unwrap()
    }
    /// Resume persisted tasks using a new worker, without replacing the database or fixture.
    pub fn restart_worker(&mut self) {
        self.worker.abort();
        self.worker = tokio::spawn(mori::jobs::run(self.state.clone()));
    }
}
impl Drop for Media {
    /// Stop job processing before removing its test database.
    fn drop(&mut self) {
        self.worker.abort();
    }
}
