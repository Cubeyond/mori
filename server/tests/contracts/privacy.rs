// SPDX-FileCopyrightText: 2026 Cubeyond
// SPDX-License-Identifier: Elastic-2.0

//! Independent access-control, collection-membership, and session contracts.

use crate::support::{Client, call, lifecycle_collection, setup};
use axum::http::StatusCode;
use serde_json::{Value, json};

/// Arrange a private album with published, private, and withdrawing members plus a standalone video.
async fn library() -> (mori::app::AppState, axum::Router, tempfile::TempDir) {
    let (state, router, directory) = setup().await;
    lifecycle_collection(&state).await;
    sqlx::query("UPDATE albums SET visibility='private' WHERE id='collection'")
        .execute(&state.db)
        .await
        .unwrap();
    for (id, token, album, visibility, delivery, prefix) in [
        ("standalone", "c", None, "public", "published", "file"),
        (
            "withdrawing",
            "d",
            Some("collection"),
            "private",
            "revoking",
            "nature",
        ),
    ] {
        sqlx::query("INSERT INTO videos(id,token,album_id,name,visibility,state,size,created_at,media_prefix) VALUES(?,?,?,'Sample',?,?,100,1,?)")
            .bind(id).bind(token.repeat(32)).bind(album).bind(visibility).bind(delivery).bind(prefix).execute(&state.db).await.unwrap();
        sqlx::query("INSERT INTO media_aliases(video_id,prefix) VALUES(?,?)")
            .bind(id)
            .bind(prefix)
            .execute(&state.db)
            .await
            .unwrap();
    }
    (state, router, directory)
}

#[tokio::test]
/// A shared album address respects the current owner session, including logout.
async fn private_album_address_authorizes_its_owner() {
    let (_state, router, _directory) = library().await;
    let owner = Client::admin(&router).await;
    let path = "/v1/albums/nature";
    let page = owner.request("GET", path, Value::Null).await;
    assert_eq!(page.status, StatusCode::OK);
    assert_eq!(page.body["videos"].as_array().unwrap().len(), 3);
    let private = page.body["videos"]
        .as_array()
        .unwrap()
        .iter()
        .find(|video| video["visibility"] == "private")
        .unwrap();
    assert!(
        private["media_url"]
            .as_str()
            .unwrap()
            .starts_with("/v1/admin/videos/")
    );
    assert_eq!(
        owner
            .request("POST", "/v1/auth/logout", Value::Null)
            .await
            .status,
        StatusCode::NO_CONTENT
    );
    for caller in [&owner, &Client::visitor(&router)] {
        assert_eq!(
            caller.request("GET", path, Value::Null).await.status,
            StatusCode::NOT_FOUND
        );
    }
}

#[tokio::test]
/// Creation normalizes unique slugs, rejects system paths, and requires CSRF.
async fn album_creation_validates_slug_and_csrf() {
    let (_state, router, _directory) = setup().await;
    let admin = Client::admin(&router).await;
    let created = admin
        .request(
            "POST",
            "/v1/admin/albums",
            json!({"name":"Kyoto","slug":" KYOTO "}),
        )
        .await;
    assert_eq!(created.status, StatusCode::CREATED);
    assert_eq!(created.body["slug"], "kyoto");
    assert_eq!(created.body["visibility"], "private");
    assert_eq!(
        admin
            .request(
                "POST",
                "/v1/admin/albums",
                json!({"name":"Duplicate","slug":"kyoto"})
            )
            .await
            .status,
        StatusCode::CONFLICT
    );
    for slug in [
        "login", "api", "v1", "files", "watch", "embed", "fonts", "../test", "a--b", "bad/path",
        "videos", "albums", "settings", "users",
    ] {
        assert_eq!(
            admin
                .request(
                    "POST",
                    "/v1/admin/albums",
                    json!({"name":"Invalid","slug":slug})
                )
                .await
                .status,
            StatusCode::BAD_REQUEST
        );
    }
    let (cookie, _) = crate::support::login(&router).await;
    assert_eq!(
        call(
            &router,
            "POST",
            "/v1/admin/albums",
            Some(&cookie),
            None,
            json!({"name":"CSRF","slug":"csrf"})
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
}

#[tokio::test]
/// Visitor listings contain only published public members, without a global directory.
async fn visitor_collection_filters_private_and_withdrawing_members() {
    let (_state, router, _directory) = library().await;
    let admin = Client::admin(&router).await;
    let visitor = Client::visitor(&router);
    for path in [
        format!("/v1/videos/{}", "b".repeat(32)),
        format!("/v1/videos/{}", "d".repeat(32)),
        "/v1/videos".into(),
        "/v1/albums".into(),
        "/files/".into(),
        "/v1/albums/nature".into(),
    ] {
        assert_eq!(
            visitor.request("GET", &path, Value::Null).await.status,
            StatusCode::NOT_FOUND
        );
    }
    assert_eq!(
        visitor
            .request("GET", "/v1/admin/albums/collection", Value::Null)
            .await
            .status,
        StatusCode::UNAUTHORIZED
    );
    let members = admin
        .request("GET", "/v1/admin/albums/collection", Value::Null)
        .await;
    assert_eq!(members.body["videos"].as_array().unwrap().len(), 3);
    let edited = admin
        .request(
            "PATCH",
            "/v1/admin/albums/collection",
            json!({"name":"Nature","visibility":"public"}),
        )
        .await;
    assert_eq!(edited.status, StatusCode::NO_CONTENT);
    let page = visitor
        .request("GET", "/v1/albums/nature", Value::Null)
        .await;
    assert_eq!(page.status, StatusCode::OK);
    assert_eq!(page.body["videos"].as_array().unwrap().len(), 1);
    assert_eq!(page.body["videos"][0]["name"], "public title");
}

#[tokio::test]
/// Album visibility controls collection access independently from an individually public video.
async fn private_album_does_not_withdraw_public_video() {
    let (_state, router, _directory) = library().await;
    let admin = Client::admin(&router).await;
    let visitor = Client::visitor(&router);
    let video = format!("/v1/videos/{}", "a".repeat(32));
    for visibility in ["public", "private"] {
        assert_eq!(
            admin
                .request(
                    "PATCH",
                    "/v1/admin/albums/collection",
                    json!({"name":"Nature","visibility":visibility})
                )
                .await
                .status,
            StatusCode::NO_CONTENT
        );
        assert_eq!(
            visitor.request("GET", &video, Value::Null).await.status,
            StatusCode::OK
        );
    }
    assert_eq!(
        visitor
            .request("GET", "/v1/albums/nature", Value::Null)
            .await
            .status,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
/// A label-only update preserves its slug and privacy settings.
async fn renaming_an_album_preserves_address_and_visibility() {
    let (_state, router, _directory) = library().await;
    let admin = Client::admin(&router).await;
    assert_eq!(
        admin
            .request(
                "PATCH",
                "/v1/admin/albums/collection",
                json!({"name":"Renamed"})
            )
            .await
            .status,
        StatusCode::NO_CONTENT
    );
    let detail = admin
        .request("GET", "/v1/admin/albums/collection", Value::Null)
        .await;
    assert_eq!(detail.body["album"]["visibility"], "private");
    assert_eq!(detail.body["album"]["slug"], "nature");
    assert_eq!(detail.body["album"]["name"], "Renamed");
}

#[tokio::test]
/// Knowing a private token cannot authorize either media or poster GET/HEAD.
async fn private_content_and_posters_require_session() {
    let (_state, router, _directory) = library().await;
    let visitor = Client::visitor(&router);
    for method in ["GET", "HEAD"] {
        for resource in ["content", "poster"] {
            assert_eq!(
                visitor
                    .request(
                        method,
                        &format!("/v1/admin/videos/b/{resource}"),
                        Value::Null
                    )
                    .await
                    .status,
                StatusCode::UNAUTHORIZED
            );
        }
    }
}

#[tokio::test]
/// Membership changes update preferred addresses while preserving previously issued aliases.
async fn reassignment_preserves_existing_shared_addresses() {
    let media = crate::support::media::Media::new(false).await;
    let album = media
        .request(
            "POST",
            "/v1/admin/albums",
            json!({"name":"Nature","slug":"nature","visibility":"private"}),
        )
        .await
        .body;
    let private = media.ready().await;
    let published = media.visibility(&private, "public").await;
    let visitor = Client::visitor(&media.router);
    let video = format!("/v1/videos/{}", published.token);
    let before = visitor.request("GET", &video, Value::Null).await.body;
    assert_eq!(
        media
            .request(
                "PATCH",
                &format!("/v1/admin/videos/{}", published.id),
                json!({"name":"Renamed","album_id":album["id"],"visibility":"public"})
            )
            .await
            .status,
        StatusCode::ACCEPTED
    );
    let after = visitor.request("GET", &video, Value::Null).await.body;
    assert_eq!(before["token"], after["token"]);
    assert!(before["media_url"].as_str().unwrap().contains("/file/"));
    assert!(after["media_url"].as_str().unwrap().contains("/nature/"));
    for prefix in ["file", "nature"] {
        let response = visitor
            .request(
                "GET",
                &format!("/{prefix}/{}.webm", published.token),
                Value::Null,
            )
            .await;
        assert_eq!(response.status, StatusCode::OK);
        assert!(
            response.headers["content-security-policy"]
                .to_str()
                .unwrap()
                .contains("frame-ancestors *")
        );
        assert!(
            response.headers["permissions-policy"]
                .to_str()
                .unwrap()
                .contains("fullscreen=*")
        );
        assert!(response.headers.get("x-frame-options").is_none());
    }
}

#[tokio::test]
/// Starting withdrawal hides visitor access before a durable worker finishes deleting copies.
async fn revocation_denies_new_access_immediately() {
    let (state, router, _directory) = library().await;
    let admin = Client::admin(&router).await;
    let edited = admin
        .request(
            "PATCH",
            "/v1/admin/videos/a",
            json!({"name":"Sample","album_id":null,"visibility":"private"}),
        )
        .await;
    assert_eq!(edited.status, StatusCode::ACCEPTED);
    assert!(edited.body["job_id"].is_string());
    let visitor = Client::visitor(&router);
    assert_eq!(
        visitor
            .request(
                "GET",
                &format!("/v1/videos/{}", "a".repeat(32)),
                Value::Null
            )
            .await
            .status,
        StatusCode::NOT_FOUND
    );
    let delivery: String = sqlx::query_scalar("SELECT state FROM videos WHERE id='a'")
        .fetch_one(&state.db)
        .await
        .unwrap();
    assert_eq!(delivery, "revoking");
}

#[tokio::test]
/// A logged-out session cookie cannot be reused to regain library access.
async fn logout_revokes_session() {
    let (_state, router, _directory) = setup().await;
    let admin = Client::admin(&router).await;
    assert_eq!(
        admin
            .request("POST", "/v1/auth/logout", Value::Null)
            .await
            .status,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        admin
            .request("GET", "/v1/admin/videos", Value::Null)
            .await
            .status,
        StatusCode::UNAUTHORIZED
    );
}
