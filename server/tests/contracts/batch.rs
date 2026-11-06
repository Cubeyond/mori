// SPDX-FileCopyrightText: 2026 Cubeyond
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Bulk mutations are bounded and atomic, preserving original bytes and independent visibility.
use crate::support::{Client, media::Media, wait_job};
use axum::http::StatusCode;
use serde_json::{Value, json};

#[tokio::test]
/// Move, detach, create-and-move and delete share the normal original-media lifecycle.
async fn grouped_membership_and_deletion_preserve_unselected_originals() {
    let media = Media::new(false).await;
    let first = media.ready().await;
    let second = media.ready().await;
    let second = media.visibility(&second, "public").await;
    let ids = json!([first.id, second.id]);
    let created = media.request("POST", "/v1/admin/videos/batch", json!({"ids":ids,"action":{"type":"create_album","album":{"name":"Selected","slug":"selected","visibility":"private"}}})).await;
    assert_eq!(created.status, StatusCode::ACCEPTED);
    let album = &created.body["album"];
    for (id, visibility) in [(&first.id, "private"), (&second.id, "public")] {
        let current = mori::features::videos::get(&media.state, id).await.unwrap();
        assert_eq!(current.album_id.as_deref(), album["id"].as_str());
        assert_eq!(current.visibility, visibility);
        assert!(
            current
                .links(&media.state, true)
                .media_url
                .contains(if visibility == "public" {
                    "/selected/"
                } else {
                    "/content"
                })
        );
    }
    for album_id in [Value::Null, album["id"].clone()] {
        let moved = media
            .request(
                "POST",
                "/v1/admin/videos/batch",
                json!({"ids":ids,"action":{"type":"move","album_id":album_id}}),
            )
            .await;
        assert_eq!(moved.status, StatusCode::ACCEPTED);
    }
    let deleted = media
        .request(
            "POST",
            "/v1/admin/videos/batch",
            json!({"ids":[first.id],"action":{"type":"delete"}}),
        )
        .await;
    assert_eq!(deleted.status, StatusCode::ACCEPTED);
    for job in deleted.body["job_ids"].as_array().unwrap() {
        assert_eq!(
            wait_job(&media.state, job.as_str().unwrap()).await,
            "complete"
        );
    }
    assert!(!media.exists(first.private_source()).await);
    assert_eq!(
        mori::features::videos::get(&media.state, &second.id)
            .await
            .unwrap()
            .state,
        "published"
    );
}

#[tokio::test]
/// A late missing member or reserved slug rolls back the album and every earlier membership update.
async fn invalid_selection_cannot_partially_create_or_move() {
    let media = Media::new(false).await;
    let ready = media.ready().await;
    for (ids, slug, expected) in [
        (
            json!([ready.id, "0".repeat(32)]),
            "atomic",
            StatusCode::NOT_FOUND,
        ),
        (json!([ready.id]), "login", StatusCode::BAD_REQUEST),
        (
            json!([ready.id, ready.id]),
            "duplicate",
            StatusCode::BAD_REQUEST,
        ),
    ] {
        let result=media.request("POST","/v1/admin/videos/batch",json!({"ids":ids,"action":{"type":"create_album","album":{"name":"Atomic","slug":slug}}})).await;
        assert_eq!(result.status, expected);
        assert!(
            mori::features::videos::get(&media.state, &ready.id)
                .await
                .unwrap()
                .album_id
                .is_none()
        );
        assert_eq!(
            media
                .request("GET", "/v1/admin/albums", Value::Null)
                .await
                .body,
            json!([])
        );
    }
    assert_eq!(
        Client::visitor(&media.router)
            .request(
                "POST",
                "/v1/admin/videos/batch",
                json!({"ids":[ready.id],"action":{"type":"delete"}})
            )
            .await
            .status,
        StatusCode::UNAUTHORIZED
    );
}
