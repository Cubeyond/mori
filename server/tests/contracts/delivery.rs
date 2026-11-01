//! Exercise S3-only delivery without permitting a CloudFront network dependency.

use crate::support::{Client, call, login, media::Media, s3_fixture, setup, wait_job};
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use mori::{
    http,
    infra::{storage::Storage, storage_settings::config::StorageSettings},
};
use serde_json::{Value, json};
use std::sync::Arc;
use tower::ServiceExt;

#[tokio::test]
/// Publishing moves the source instead of retaining a second full-size private object.
async fn publication_releases_private_objects_and_restores_them_on_withdrawal() {
    let media = Media::new(false).await;
    let private = media.ready().await;
    let published = media.visibility(&private, "public").await;
    assert!(
        media
            .exists(Storage::public(&published.token, &published.extension))
            .await
    );
    assert!(!media.exists(Storage::source(&published.id)).await);
    assert!(!media.exists(Storage::poster(&published.id)).await);
    assert!(!media.exists(published.private_source()).await);
    assert!(!media.exists(published.private_poster()).await);
    let private = media.visibility(&published, "private").await;
    assert_eq!(
        private.private_source().strip_prefix("private/"),
        Storage::public(&private.token, &private.extension).strip_prefix("public/")
    );
    assert!(media.exists(private.private_source()).await);
    assert!(
        !media
            .exists(Storage::public(&private.token, &private.extension))
            .await
    );
    assert_eq!(
        media
            .content(
                &format!("/v1/admin/videos/{}/content", private.id),
                Some("bytes=0-15")
            )
            .await
            .status(),
        StatusCode::PARTIAL_CONTENT
    );
    let published = media.visibility(&private, "public").await;
    assert!(!media.exists(Storage::source(&published.id)).await);
    assert!(!media.exists(published.private_source()).await);
}

/// Use the fixture through the real storage constructor, leaving `CloudFront` unconfigured.
async fn without_cdn() -> (
    mori::app::AppState,
    crate::support::Fixture,
    tempfile::TempDir,
) {
    let (mut state, _, directory) = setup().await;
    let (fixture, endpoint) = s3_fixture().await;
    let mut config = (*state.config).clone();
    config.s3_endpoint = Some(endpoint);
    state.config = Arc::new(config);
    let settings = StorageSettings {
        bucket: "test-bucket".into(),
        region: "us-east-1".into(),
        distribution_id: None,
        access_key_id: "testing".into(),
        secret_access_key: "testing".into(),
        session_token: None,
    };
    state.storage.persist(&state.db, &settings).await.unwrap();
    state
        .storage
        .install(Storage::new(&settings, &state.config).unwrap());
    assert!(state.storage.get().unwrap().cf.is_none());
    (state, fixture, directory)
}

/// Create actual private objects and metadata; publication must still use its durable job.
async fn seed(state: &mori::app::AppState) {
    sqlx::query("INSERT INTO albums(id,name,slug,visibility,created_at) VALUES('collection','Nature','nature','public',1)")
        .execute(&state.db).await.unwrap();
    for (id, token) in [("sample", "a"), ("hidden", "b")] {
        sqlx::query("INSERT INTO videos(id,token,name,visibility,state,size,poster,created_at,extension) VALUES(?,?,'Sample','private','ready',?,1,1,'webm')")
            .bind(id).bind(token.repeat(32)).bind(i64::try_from(include_bytes!("../fixtures/short.webm").len()).unwrap())
            .execute(&state.db).await.unwrap();
        let storage = state.storage.get().unwrap();
        for (key, bytes) in [
            (
                Storage::source(id),
                include_bytes!("../fixtures/short.webm").as_slice(),
            ),
            (Storage::poster(id), [255, 216, 255, 217].as_slice()),
        ] {
            storage
                .s3
                .put_object()
                .bucket(&storage.bucket)
                .key(key)
                .body(aws_sdk_s3::primitives::ByteStream::from(bytes.to_vec()))
                .send()
                .await
                .unwrap();
        }
    }
}

/// Assert a method-specific signed redirect stays on the fixture's published prefix.
async fn signed(router: &axum::Router, method: &str, path: &str) -> url::Url {
    let (status, headers, _) = call(router, method, path, None, None, Value::Null).await;
    assert_eq!(status, StatusCode::TEMPORARY_REDIRECT);
    assert_eq!(headers["cache-control"], "no-store");
    assert!(
        headers["content-security-policy"]
            .to_str()
            .unwrap()
            .contains("frame-ancestors *")
    );
    assert!(
        headers["permissions-policy"]
            .to_str()
            .unwrap()
            .contains("fullscreen=*")
    );
    let url = url::Url::parse(headers["location"].to_str().unwrap()).unwrap();
    assert!(url.path().starts_with("/test-bucket/public/files/"));
    let query: std::collections::BTreeMap<_, _> = url.query_pairs().collect();
    assert_eq!(query.get("X-Amz-Expires").unwrap(), "3600");
    assert_eq!(query.get("X-Amz-SignedHeaders").unwrap(), "host");
    assert!(query.contains_key("X-Amz-Signature"));
    if method == "GET" {
        assert_eq!(query.get("response-cache-control").unwrap(), "no-store");
        assert_eq!(query.get("response-content-disposition").unwrap(), "inline");
    }
    url
}

/// Publish an album member using its real source and durable copy job.
async fn published_album() -> (Media, mori::features::videos::Video) {
    let media = Media::new(false).await;
    seed(&media.state).await;
    let reply = media
        .request(
            "PATCH",
            "/v1/admin/videos/sample",
            json!({"name":"Sample","album_id":"collection","visibility":"public"}),
        )
        .await;
    assert_eq!(reply.status, StatusCode::ACCEPTED);
    assert_eq!(
        wait_job(&media.state, reply.body["job_id"].as_str().unwrap()).await,
        "complete"
    );
    let video = mori::features::videos::get(&media.state, "sample")
        .await
        .unwrap();
    (media, video)
}

#[tokio::test]
/// Album links and posters receive method-specific signatures scoped to published copies.
async fn public_s3_links_and_posters_are_scoped() {
    let (media, video) = published_album().await;
    let path = format!("/nature/{}.webm", video.token);
    let get = signed(&media.router, "GET", &path).await;
    let head = signed(&media.router, "HEAD", &path).await;
    assert_ne!(get.query(), head.query());
    let poster = format!("/files/{}.jpg", video.token);
    signed(&media.router, "GET", &poster).await;
    signed(&media.router, "HEAD", &poster).await;
    let visitor = Client::visitor(&media.router);
    for path in [
        format!("/files/{}.jpg", "b".repeat(32)),
        format!("/file/{}.webm", video.token),
        format!("/nature/{}.mp4", video.token),
        format!("/nature/{}.webm", "b".repeat(32)),
    ] {
        assert_eq!(
            visitor.request("GET", &path, Value::Null).await.status,
            StatusCode::NOT_FOUND
        );
    }
    let album = visitor
        .request("GET", "/v1/albums/nature", Value::Null)
        .await;
    assert_eq!(album.body["videos"].as_array().unwrap().len(), 1);
    assert_eq!(
        album.body["videos"][0]["media_url"],
        format!("http://localhost:3000{path}")
    );
}

#[tokio::test]
/// Public ranges remain unsigned and browser policy admits only the configured delivery origin.
async fn s3_seeking_and_browser_policy() {
    let (media, video) = published_album().await;
    let path = format!("/nature/{}.webm", video.token);
    let range = media
        .router
        .clone()
        .oneshot(
            Request::builder()
                .uri(&path)
                .header("range", "bytes=3-17")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(range.status(), StatusCode::TEMPORARY_REDIRECT);
    let url = url::Url::parse(range.headers()["location"].to_str().unwrap()).unwrap();
    assert!(
        url.query_pairs()
            .any(|(key, value)| key == "X-Amz-SignedHeaders" && value == "host")
    );
    let storage = media.state.storage.get().unwrap();
    let object = storage
        .s3
        .get_object()
        .bucket(&storage.bucket)
        .key(Storage::public(&video.token, "webm"))
        .range("bytes=3-17")
        .send()
        .await
        .unwrap();
    assert_eq!(
        object.body.collect().await.unwrap().into_bytes(),
        include_bytes!("../fixtures/short.webm")[3..18]
    );
    let home = Client::visitor(&media.router)
        .request("GET", "/", Value::Null)
        .await;
    let policy = home.headers["content-security-policy"].to_str().unwrap();
    let origin = media.state.config.s3_endpoint.as_deref().unwrap();
    assert!(policy.contains(&format!("media-src 'self' blob: {origin}")));
    assert!(policy.contains(&format!("img-src 'self' blob: data: {origin}")));
}

#[tokio::test]
/// S3-only withdrawal removes public copies, retains sources, and needs no invalidation.
async fn s3_revocation_and_republication() {
    let (media, video) = published_album().await;
    let video = media.visibility(&video, "private").await;
    let visitor = Client::visitor(&media.router);
    for path in [
        format!("/nature/{}.webm", video.token),
        format!("/files/{}.jpg", video.token),
    ] {
        assert_eq!(
            visitor.request("GET", &path, Value::Null).await.status,
            StatusCode::NOT_FOUND
        );
    }
    assert!(!media.exists(Storage::public(&video.token, "webm")).await);
    assert!(media.exists(video.private_source()).await);
    assert_eq!(media.visibility(&video, "public").await.state, "published");
    let invalidations: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM jobs WHERE invalidation_id IS NOT NULL")
            .fetch_one(&media.state.db)
            .await
            .unwrap();
    assert_eq!(invalidations, 0);
}

#[tokio::test]
/// Deleting an S3-only public video removes its source and published copy through normal jobs.
async fn s3_public_video_deletion() {
    let (media, video) = published_album().await;
    let deletion = media
        .request(
            "DELETE",
            &format!("/v1/admin/videos/{}", video.id),
            Value::Null,
        )
        .await;
    assert_eq!(deletion.status, StatusCode::ACCEPTED);
    assert_eq!(
        wait_job(&media.state, deletion.body["job_id"].as_str().unwrap()).await,
        "complete"
    );
    assert!(!media.exists(Storage::source(&video.id)).await);
    assert!(!media.exists(Storage::public(&video.token, "webm")).await);
}

#[tokio::test]
/// Uploading a public standalone video does not require an album or CDN configuration.
async fn public_upload_needs_only_s3() {
    let (state, _fixture, _directory) = without_cdn().await;
    let router = http::router(state.clone());
    let (cookie, csrf) = login(&router).await;
    let upload = call(
        &router,
        "POST",
        "/v1/admin/uploads",
        Some(&cookie),
        Some(&csrf),
        json!({"size":100,"extension":"mp4","visibility":"public"}),
    )
    .await;
    assert_eq!(upload.0, StatusCode::CREATED);
    let visibility: String = sqlx::query_scalar("SELECT visibility FROM videos WHERE id=?")
        .bind(upload.2["video_id"].as_str().unwrap())
        .fetch_one(&state.db)
        .await
        .unwrap();
    assert_eq!(visibility, "public");
}

#[tokio::test]
/// Allow optional CDN changes for private libraries, preserving pending revocation barriers.
async fn delivery_changes_wait_for_public_revocation() {
    let (state, _fixture, _directory) = without_cdn().await;
    seed(&state).await;
    let router = http::router(state.clone());
    let (cookie, csrf) = login(&router).await;
    let enabled = json!({"bucket":"test-bucket","region":"us-east-1","distribution_id":"TESTDIST"});
    let disabled = json!({"bucket":"test-bucket","region":"us-east-1","distribution_id":null});
    let saved = call(
        &router,
        "PUT",
        "/v1/admin/storage",
        Some(&cookie),
        Some(&csrf),
        enabled.clone(),
    )
    .await;
    assert_eq!(saved.0, StatusCode::OK);
    assert_eq!(saved.2["location_locked"], true);
    assert_eq!(saved.2["delivery_change"], "available");
    for delivery in ["publishing", "published", "revoking", "deleting"] {
        sqlx::query("UPDATE videos SET state=? WHERE id='sample'")
            .bind(delivery)
            .execute(&state.db)
            .await
            .unwrap();
        let saved = call(
            &router,
            "PUT",
            "/v1/admin/storage",
            Some(&cookie),
            Some(&csrf),
            disabled.clone(),
        )
        .await;
        assert_eq!(saved.0, StatusCode::CONFLICT);
        assert!(state.storage.get().unwrap().distribution_id.is_some());
    }
    sqlx::query("UPDATE videos SET state='ready' WHERE id='sample'")
        .execute(&state.db)
        .await
        .unwrap();
    sqlx::query("INSERT INTO jobs(id,video_id,kind,revision,created_at,invalidation_id) VALUES('pending','sample','revoke',1,1,'in-progress')").execute(&state.db).await.unwrap();
    assert_eq!(
        call(
            &router,
            "PUT",
            "/v1/admin/storage",
            Some(&cookie),
            Some(&csrf),
            disabled.clone()
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    sqlx::query("UPDATE jobs SET state='complete' WHERE id='pending'")
        .execute(&state.db)
        .await
        .unwrap();
    let saved = call(
        &router,
        "PUT",
        "/v1/admin/storage",
        Some(&cookie),
        Some(&csrf),
        disabled,
    )
    .await;
    assert_eq!(saved.0, StatusCode::OK);
    assert!(state.storage.get().unwrap().cf.is_none());
    assert_eq!(saved.2["delivery_change"], "available");
}
