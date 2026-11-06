// SPDX-FileCopyrightText: 2026 Cubeyond
// SPDX-License-Identifier: Elastic-2.0

//! Albums HTTP contracts with isolated state.

use crate::support::{call, lifecycle_collection, login, setup};
use axum::http::StatusCode;
use serde_json::Value;
use serde_json::json;

#[tokio::test]
/// Public empty albums remain readable while invalid visibility is rejected.
async fn empty_public_album_contract() {
    let (_state, router, _directory) = setup().await;
    let (cookie, csrf) = login(&router).await;
    let (status, _, _) = call(
        &router,
        "POST",
        "/v1/admin/albums",
        Some(&cookie),
        Some(&csrf),
        json!({"name":"Empty","slug":"empty-public","visibility":"public"}),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let (status, _, page) = call(
        &router,
        "GET",
        "/v1/albums/empty-public",
        None,
        None,
        Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(page["videos"].as_array().unwrap().len(), 0);
    assert_eq!(page["album"]["visibility"], "public");
    assert_eq!(
        call(
            &router,
            "POST",
            "/v1/admin/albums",
            Some(&cookie),
            Some(&csrf),
            json!({"name":"Invalid","slug":"invalid-visibility","visibility":"unlisted"}),
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
}

#[tokio::test]
/// Slug edits validate uniqueness and update member resources while releasing the old address.
async fn album_slug_edit_contract() {
    let media = collection().await;
    let router = &media.router;
    let (cookie, csrf) = login(router).await;
    for (slug, expected) in [
        ("login", StatusCode::BAD_REQUEST),
        ("other", StatusCode::CONFLICT),
        ("New-Nature", StatusCode::NO_CONTENT),
    ] {
        assert_eq!(
            call(
                router,
                "PATCH",
                "/v1/admin/albums/collection",
                Some(&cookie),
                Some(&csrf),
                json!({"name":"Nature","slug":slug})
            )
            .await
            .0,
            expected
        );
    }
    let token = "a".repeat(32);
    assert_eq!(
        call(
            router,
            "GET",
            &format!("/nature/{token}.mp4"),
            None,
            None,
            Value::Null
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        call(
            router,
            "GET",
            &format!("/new-nature/{token}.mp4"),
            None,
            None,
            Value::Null
        )
        .await
        .0,
        StatusCode::OK
    );
    let video = call(
        router,
        "GET",
        &format!("/v1/videos/{token}"),
        None,
        None,
        Value::Null,
    )
    .await
    .2;
    assert!(
        video["media_url"]
            .as_str()
            .unwrap()
            .contains("/new-nature/")
    );
    assert_eq!(
        call(
            router,
            "POST",
            "/v1/admin/albums",
            Some(&cookie),
            Some(&csrf),
            json!({"name":"Reused","slug":"nature"})
        )
        .await
        .0,
        StatusCode::CREATED
    );
    assert_eq!(
        call(
            router,
            "GET",
            &format!("/nature/{token}.mp4"),
            None,
            None,
            Value::Null
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
/// Collection-only deletion preserves both visibility policies and releases its address safely.
async fn album_retention_deletion_contract() {
    let media = collection().await;
    let state = &media.state;
    let router = &media.router;
    let (cookie, csrf) = login(router).await;
    assert_eq!(
        call(
            router,
            "DELETE",
            "/v1/admin/albums/collection",
            Some(&cookie),
            Some(&csrf),
            json!({"confirmation":"nature"})
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        call(
            router,
            "DELETE",
            "/v1/admin/albums/collection",
            Some(&cookie),
            Some(&csrf),
            json!({"confirmation":"Nature"})
        )
        .await
        .0,
        StatusCode::NO_CONTENT
    );
    let rows: Vec<(Option<String>, String, String, String)> =
        sqlx::query_as("SELECT album_id,media_prefix,visibility,state FROM videos ORDER BY id")
            .fetch_all(&state.db)
            .await
            .unwrap();
    assert_eq!(
        rows,
        vec![
            (None, "file".into(), "public".into(), "published".into()),
            (None, "file".into(), "private".into(), "ready".into())
        ]
    );
    let token = "a".repeat(32);
    assert_eq!(
        call(
            router,
            "GET",
            &format!("/file/{token}.mp4"),
            None,
            None,
            Value::Null
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        call(
            router,
            "GET",
            &format!("/file/{}.mp4", "b".repeat(32)),
            None,
            None,
            Value::Null
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        call(
            router,
            "POST",
            "/v1/admin/albums",
            Some(&cookie),
            Some(&csrf),
            json!({"name":"Replacement","slug":"nature"})
        )
        .await
        .0,
        StatusCode::CREATED
    );
    assert_eq!(
        call(
            router,
            "GET",
            &format!("/nature/{token}.mp4"),
            None,
            None,
            Value::Null
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
/// Bulk removal rolls back every change if a member is busy, otherwise queues existing deletion jobs.
async fn album_media_deletion_atomic_contract() {
    let (state, router, _directory) = setup().await;
    lifecycle_collection(&state).await;
    sqlx::query("INSERT INTO videos(id,token,album_id,name,visibility,state,size,created_at,media_prefix) VALUES('z',?,'collection','Busy','private','verifying',100,1,'nature')").bind("z".repeat(32)).execute(&state.db).await.unwrap();
    let (cookie, csrf) = login(&router).await;
    let payload = json!({"confirmation":"Nature","delete_media":true});
    assert_eq!(
        call(
            &router,
            "DELETE",
            "/v1/admin/albums/collection",
            Some(&cookie),
            Some(&csrf),
            payload.clone()
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM jobs")
        .fetch_one(&state.db)
        .await
        .unwrap();
    assert_eq!(count, 0);
    let public: String = sqlx::query_scalar("SELECT state FROM videos WHERE id='a'")
        .fetch_one(&state.db)
        .await
        .unwrap();
    assert_eq!(public, "published");
    sqlx::query("UPDATE videos SET state='failed' WHERE id='z'")
        .execute(&state.db)
        .await
        .unwrap();
    let (status, _, result) = call(
        &router,
        "DELETE",
        "/v1/admin/albums/collection",
        Some(&cookie),
        Some(&csrf),
        payload,
    )
    .await;
    assert_eq!(status, StatusCode::ACCEPTED);
    assert_eq!(result["job_ids"].as_array().unwrap().len(), 3);
    let kinds: Vec<String> = sqlx::query_scalar("SELECT kind FROM jobs ORDER BY video_id")
        .fetch_all(&state.db)
        .await
        .unwrap();
    assert_eq!(kinds, vec!["delete", "delete_private", "delete_private"]);
    let count:i64=sqlx::query_scalar("SELECT COUNT(*) FROM videos WHERE state='deleting' AND visibility='private' AND album_id IS NULL").fetch_one(&state.db).await.unwrap();
    assert_eq!(count, 3);
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM albums WHERE slug='nature'")
        .fetch_one(&state.db)
        .await
        .unwrap();
    assert_eq!(count, 0);
}

/// Supply actual S3 bytes for collection-address contracts without mocking delivery responses.
async fn collection() -> crate::support::media::Media {
    let media = crate::support::media::Media::new(false).await;
    lifecycle_collection(&media.state).await;
    let storage = media.state.storage.get().unwrap();
    storage
        .s3
        .put_object()
        .bucket(&storage.bucket)
        .key(mori::infra::storage::Storage::public(
            &"a".repeat(32),
            "mp4",
        ))
        .body(aws_sdk_s3::primitives::ByteStream::from(vec![0; 100]))
        .send()
        .await
        .unwrap();
    media
}
