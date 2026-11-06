// SPDX-FileCopyrightText: 2026 Cubeyond
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Multipart integrity, private preview, and durable optional CDN contracts.
use crate::support::{Client, call, login, media::Media, setup, wait_job};
use axum::http::StatusCode;
use http_body_util::BodyExt;
use mori::infra::storage::Storage;
use serde_json::{Value, json};

#[tokio::test]
/// Upload limits come from server configuration and reject oversized or disabled formats early.
async fn configured_upload_policy_contract() {
    let (state, router, _directory) = setup().await;
    let mut config = (*state.config).clone();
    for limit in [16, 1_000_000_000, 5_000_000_000] {
        config.max_upload_bytes = limit;
        assert!(config.validate().is_ok());
    }
    for limit in [15, 5_000_000_001] {
        config.max_upload_bytes = limit;
        assert!(config.validate().is_err());
    }
    let (cookie, csrf) = login(&router).await;
    assert_eq!(
        call(
            &router,
            "GET",
            "/v1/admin/upload-policy",
            None,
            None,
            Value::Null
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    let policy = call(
        &router,
        "GET",
        "/v1/admin/upload-policy",
        Some(&cookie),
        None,
        Value::Null,
    )
    .await
    .2;
    assert_eq!(policy["max_upload_bytes"], 1_000_000_000);
    assert_eq!(policy["max_image_upload_bytes"], 104_857_600);
    assert!(
        policy["allowed_extensions"]
            .as_array()
            .unwrap()
            .contains(&json!("webm"))
    );
    for input in [
        json!({"album_id":null,"size":1_000_000_001i64,"visibility":"private","extension":"mp4"}),
        json!({"album_id":null,"size":104_857_601i64,"visibility":"private","extension":"jpg"}),
        json!({"album_id":null,"size":100,"visibility":"private","extension":"exe"}),
        json!({"album_id":null,"size":100,"visibility":"private","extension":"../mp4"}),
    ] {
        assert_eq!(
            call(
                &router,
                "POST",
                "/v1/admin/uploads",
                Some(&cookie),
                Some(&csrf),
                input
            )
            .await
            .0,
            StatusCode::BAD_REQUEST
        );
    }
}

#[tokio::test]
/// Image limits are independent of video limits and enforced before a multipart session exists.
async fn image_limit_is_independent_and_configurable() {
    let media = Media::new(false).await;
    let mut state = media.state.clone();
    let mut config = (*state.config).clone();
    config.max_image_upload_bytes = 512;
    config.max_upload_bytes = 256;
    config.validate().unwrap();
    state.config = std::sync::Arc::new(config);
    let router = mori::http::router(state);
    let admin = Client::admin(&router).await;
    for (extension, size, expected) in [
        ("jpg", 512, StatusCode::CREATED),
        ("jpg", 513, StatusCode::BAD_REQUEST),
        ("webm", 257, StatusCode::BAD_REQUEST),
        ("webm", 256, StatusCode::CREATED),
    ] {
        let response = admin
            .request(
                "POST",
                "/v1/admin/uploads",
                json!({"size":size,"extension":extension,"visibility":"private"}),
            )
            .await;
        assert_eq!(response.status, expected);
    }
}

#[tokio::test]
/// Upload names are generated independently of the user's local filename.
async fn upload_assigns_random_display_name() {
    let media = Media::new(false).await;
    let bytes = include_bytes!("../fixtures/short.webm");
    let rejected = media
        .request(
            "POST",
            "/v1/admin/uploads",
            json!({"name":"local-file.webm","size":bytes.len(),"visibility":"private"}),
        )
        .await;
    assert_eq!(rejected.status, StatusCode::UNPROCESSABLE_ENTITY);
    let upload = media.begin("private", bytes.len(), "webm").await;
    assert!(mori::app::valid_token(upload["name"].as_str().unwrap()));
    assert_ne!(upload["name"], upload["video_id"]);
    let library = media.request("GET", "/v1/admin/videos", Value::Null).await;
    assert_eq!(library.body["videos"], json!([]));
}

#[tokio::test]
/// Resuming observes real checksum-confirmed parts before server-controlled completion.
async fn completed_upload_preserves_container_and_metadata() {
    let media = Media::new(false).await;
    let bytes = include_bytes!("../fixtures/short.webm");
    let upload = media.begin("private", bytes.len(), "webm").await;
    media.part(&upload, bytes).await;
    let resume = media
        .request(
            "GET",
            &format!("/v1/admin/uploads/{}", upload["id"].as_str().unwrap()),
            Value::Null,
        )
        .await;
    assert_eq!(resume.body["parts"][0]["size"], bytes.len());
    assert!(resume.body["parts"][0]["checksum"].is_string());
    let job = media.complete(&upload).await;
    assert_eq!(wait_job(&media.state, &job).await, "complete");
    let video = mori::features::videos::get(&media.state, upload["video_id"].as_str().unwrap())
        .await
        .unwrap();
    assert_eq!(video.state, "ready");
    assert_eq!(
        (video.width, video.height, video.extension.as_str()),
        (32, 24, "webm")
    );
}

#[tokio::test]
/// Private streaming forwards exactly the requested bytes without exposing a signed source URL.
async fn private_preview_has_exact_byte_ranges() {
    let media = Media::new(false).await;
    let video = media.ready().await;
    let response = media
        .content(
            &format!("/v1/admin/videos/{}/content", video.id),
            Some("bytes=0-15"),
        )
        .await;
    assert_eq!(response.status(), StatusCode::PARTIAL_CONTENT);
    assert_eq!(response.headers()["cache-control"], "private, no-store");
    assert_eq!(
        response.into_body().collect().await.unwrap().to_bytes()[..],
        include_bytes!("../fixtures/short.webm")[..16]
    );
}

#[tokio::test]
/// Poster reads retain a separate budget when every video preview slot is occupied.
async fn posters_do_not_consume_video_preview_capacity() {
    let media = Media::new(false).await;
    let video = media.ready().await;
    let _slots = media
        .state
        .preview_slots
        .clone()
        .acquire_many_owned(2)
        .await
        .unwrap();
    let path = format!("/v1/admin/videos/{}/poster", video.id);
    let replies = futures_util::future::join_all((0..4).map(|_| media.content(&path, None))).await;
    for reply in replies {
        assert_eq!(reply.status(), StatusCode::OK);
        assert_eq!(
            reply.into_body().collect().await.unwrap().to_bytes()[..],
            [255, 216, 255, 217]
        );
    }
    assert_eq!(
        media
            .content(&format!("/v1/admin/videos/{}/content", video.id), None)
            .await
            .status(),
        StatusCode::TOO_MANY_REQUESTS
    );
}

#[tokio::test]
/// Optional CDN withdrawal recovers its persisted invalidation after the worker restarts.
async fn cloudfront_revocation_survives_worker_restart() {
    let mut media = Media::new(true).await;
    let video = media.ready().await;
    let video = media.visibility(&video, "public").await;
    assert_eq!(video.state, "published");
    let visitor = Client::visitor(&media.router);
    let path = format!("/v1/videos/{}", video.token);
    assert_eq!(
        visitor.request("GET", &path, Value::Null).await.status,
        StatusCode::OK
    );
    let revoked = media
        .request(
            "PATCH",
            &format!("/v1/admin/videos/{}", video.id),
            json!({"name":video.name,"album_id":null,"visibility":"private"}),
        )
        .await;
    assert_eq!(
        visitor.request("GET", &path, Value::Null).await.status,
        StatusCode::NOT_FOUND
    );
    media.restart_worker();
    let job = revoked.body["job_id"].as_str().unwrap();
    assert_eq!(wait_job(&media.state, job).await, "complete");
    let invalidation: Option<String> =
        sqlx::query_scalar("SELECT invalidation_id FROM jobs WHERE id=?")
            .bind(job)
            .fetch_one(&media.state.db)
            .await
            .unwrap();
    assert!(invalidation.is_some());
    assert!(!media.exists(Storage::public(&video.token, "webm")).await);
    assert!(media.exists(video.private_source()).await);
}

#[tokio::test]
/// An incorrect declared length becomes a visible terminal failure without publishing bytes.
async fn wrong_upload_size_is_not_retried_forever() {
    let media = Media::new(false).await;
    let bytes = include_bytes!("../fixtures/short.webm");
    let upload = media.begin("private", bytes.len() + 1, "webm").await;
    media.part(&upload, bytes).await;
    let job = media.complete(&upload).await;
    assert_eq!(wait_job(&media.state, &job).await, "failed");
    let video = mori::features::videos::get(&media.state, upload["video_id"].as_str().unwrap())
        .await
        .unwrap();
    assert_eq!(video.state, "failed");
    assert!(video.last_error.is_some());
    let library = media.request("GET", "/v1/admin/videos", Value::Null).await;
    assert_eq!(library.body["videos"], json!([]));
}
