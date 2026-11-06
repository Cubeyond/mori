// SPDX-FileCopyrightText: 2026 Cubeyond
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Cancellation remains recoverable without browser persistence and releases album reservations.
use crate::support::media::Media;
use axum::http::StatusCode;
use serde_json::{Value, json};

#[tokio::test]
/// Retrying a successful cancellation is harmless and leaves completed originals untouched.
async fn cancel_retry_is_idempotent() {
    let media = Media::new(false).await;
    let upload = media.begin("private", 1024, "jpg").await;
    let original = mori::features::videos::get(&media.state, upload["video_id"].as_str().unwrap())
        .await
        .unwrap();
    media.poster(&original.id).await;
    let path = format!("/v1/admin/uploads/{}", upload["id"].as_str().unwrap());
    for _ in 0..2 {
        assert_eq!(
            media.request("DELETE", &path, Value::Null).await.status,
            StatusCode::NO_CONTENT
        );
    }
    assert!(!media.exists(original.private_poster()).await);
}

#[tokio::test]
/// Server-discovered uploads can be cancelled as a group without the browser's resume metadata.
async fn server_queue_can_be_cancelled_after_browser_state_is_lost() {
    let media = Media::new(false).await;
    let first = media.begin("private", 1024, "jpg").await;
    let second = media.begin("private", 1024, "jpg").await;
    let response = media.request("GET", "/v1/admin/uploads", Value::Null).await;
    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(response.body.as_array().unwrap().len(), 2);
    let response = media
        .request(
            "DELETE",
            "/v1/admin/uploads",
            json!({"ids":[first["id"],second["id"]]}),
        )
        .await;
    assert_eq!(response.status, StatusCode::NO_CONTENT);
    assert_eq!(
        media
            .request("GET", "/v1/admin/uploads", Value::Null)
            .await
            .body,
        json!([])
    );
}

#[tokio::test]
/// Cancelling an orphaned album upload releases deletion without removing already completed media.
async fn cancelling_releases_album_deletion_and_retains_ready_media() {
    let media = Media::new(false).await;
    let ready = media.ready().await;
    let album = media
        .request(
            "POST",
            "/v1/admin/albums",
            json!({"name":"Paused","slug":"paused"}),
        )
        .await
        .body;
    let attached = media
        .request(
            "PATCH",
            &format!("/v1/admin/videos/{}", ready.id),
            json!({"name":ready.name,"album_id":album["id"],"visibility":"private"}),
        )
        .await;
    assert_eq!(attached.status, StatusCode::ACCEPTED);
    let upload = media
        .request(
            "POST",
            "/v1/admin/uploads",
            json!({"album_id":album["id"],"size":1024,"extension":"jpg","visibility":"private"}),
        )
        .await;
    let deleted = media
        .request(
            "DELETE",
            &format!("/v1/admin/albums/{}", album["id"].as_str().unwrap()),
            json!({"confirmation":"Paused","delete_media":true}),
        )
        .await;
    assert_eq!(deleted.status, StatusCode::CONFLICT);
    assert_eq!(
        media
            .request(
                "DELETE",
                "/v1/admin/uploads",
                json!({"ids":[upload.body["id"]]})
            )
            .await
            .status,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        media
            .request(
                "DELETE",
                &format!("/v1/admin/albums/{}", album["id"].as_str().unwrap()),
                json!({"confirmation":"Paused","delete_media":false})
            )
            .await
            .status,
        StatusCode::NO_CONTENT
    );
    let current = mori::features::videos::get(&media.state, &ready.id)
        .await
        .unwrap();
    assert!(current.album_id.is_none());
    assert_eq!(current.state, "ready");
    assert!(media.exists(current.private_source()).await);
}
