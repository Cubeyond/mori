// SPDX-FileCopyrightText: 2026 Cubeyond
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Album cover summaries and optional descriptions preserve privacy and bounded responses.

use crate::support::{Client, lifecycle_collection, setup};
use axum::http::StatusCode;
use serde_json::{Value, json};

#[tokio::test]
/// Completed image/video previews are bounded per album and never exposed by an anonymous listing.
async fn previews_are_bounded_and_authenticated() {
    let (state, router, _directory) = setup().await;
    lifecycle_collection(&state).await;
    sqlx::query("UPDATE videos SET poster=1")
        .execute(&state.db)
        .await
        .unwrap();
    for (id, kind, status, poster, timestamp) in [
        ("image", "image", "ready", 1, 3),
        ("video", "video", "published", 1, 2),
        ("upload", "image", "uploading", 1, 9),
        ("verify", "video", "verifying", 1, 9),
        ("failed", "image", "failed", 1, 9),
        ("deleted", "image", "deleted", 1, 9),
        ("no-cover", "video", "ready", 0, 9),
    ] {
        sqlx::query("INSERT INTO videos(id,token,album_id,name,visibility,state,size,created_at,poster,kind) VALUES(?,?,'collection',?,'private',?,100,?,?,?)")
            .bind(id).bind(id).bind(id).bind(status).bind(timestamp).bind(poster).bind(kind)
            .execute(&state.db).await.unwrap();
    }
    let visitor = Client::visitor(&router);
    assert_eq!(
        visitor
            .request("GET", "/v1/admin/albums", Value::Null)
            .await
            .status,
        StatusCode::UNAUTHORIZED
    );
    let admin = Client::admin(&router).await;
    let reply = admin.request("GET", "/v1/admin/albums", Value::Null).await;
    assert_eq!(reply.status, StatusCode::OK);
    let albums = reply.body.as_array().unwrap();
    assert_eq!(albums[0]["id"], "other");
    assert_eq!(albums[0]["previews"], json!([]));
    assert_eq!(
        albums[1]["previews"],
        json!([
            {"id":"image","kind":"image","poster_url":"/v1/admin/videos/image/poster"},
            {"id":"video","kind":"video","poster_url":"/v1/admin/videos/video/poster"},
            {"id":"b","kind":"video","poster_url":"/v1/admin/videos/b/poster"}
        ])
    );
    for preview in albums[1]["previews"].as_array().unwrap() {
        assert_eq!(
            visitor
                .request("GET", preview["poster_url"].as_str().unwrap(), Value::Null)
                .await
                .status,
            StatusCode::UNAUTHORIZED
        );
    }
    let public = visitor
        .request("GET", "/v1/albums/nature", Value::Null)
        .await;
    assert!(public.body["album"].get("previews").is_none());
    let visible: Vec<&str> = public.body["videos"]
        .as_array()
        .unwrap()
        .iter()
        .map(|video| video["id"].as_str().unwrap())
        .collect();
    assert_eq!(visible, ["a"]);
}

#[tokio::test]
/// Descriptions trim safely, survive omitted updates, clear explicitly and reject invalid input.
async fn descriptions_are_optional_and_validated() {
    let (_state, router, _directory) = setup().await;
    let admin = Client::admin(&router).await;
    let reply = admin.request("POST", "/v1/admin/albums", json!({
        "name":"Nature", "slug":"nature", "visibility":"public", "description":"  Quiet trails. 森  "
    })).await;
    assert_eq!(reply.status, StatusCode::CREATED);
    assert_eq!(reply.body["description"], "Quiet trails. 森");
    let path = format!("/v1/admin/albums/{}", reply.body["id"].as_str().unwrap());
    assert_eq!(
        admin
            .request("PATCH", &path, json!({"name":"Nature"}))
            .await
            .status,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        admin.request("GET", &path, Value::Null).await.body["album"]["description"],
        "Quiet trails. 森"
    );
    for description in ["森".repeat(501), "hidden\u{0000}control".into()] {
        assert_eq!(
            admin
                .request(
                    "PATCH",
                    &path,
                    json!({"name":"Nature","description":description})
                )
                .await
                .status,
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            admin
                .request(
                    "POST",
                    "/v1/admin/albums",
                    json!({"name":"Invalid","slug":"invalid","description":description})
                )
                .await
                .status,
            StatusCode::BAD_REQUEST
        );
    }
    assert_eq!(
        admin
            .request("PATCH", &path, json!({"name":"Nature","description":""}))
            .await
            .status,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        admin.request("GET", &path, Value::Null).await.body["album"]["description"],
        ""
    );
    let empty = admin
        .request(
            "POST",
            "/v1/admin/albums",
            json!({"name":"Empty","slug":"empty"}),
        )
        .await;
    assert_eq!(empty.status, StatusCode::CREATED);
    assert_eq!(empty.body["description"], "");
}
