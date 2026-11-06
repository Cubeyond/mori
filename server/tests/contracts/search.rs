// SPDX-FileCopyrightText: 2026 Cubeyond
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Full-collection search and counts must obey the same privacy scope as result pages.
use crate::support::{Client, lifecycle_collection, setup};
use axum::http::StatusCode;
use serde_json::Value;

#[tokio::test]
/// Names beyond the first page are searchable; literal wildcard characters remain literal.
async fn search_counts_all_pages_and_escapes_patterns() {
    let (state, router, _directory) = setup().await;
    for n in 0..37 {
        sqlx::query("INSERT INTO videos(id,token,name,visibility,state,size,created_at) VALUES(?,?,?,'private','ready',100,?)")
            .bind(format!("id-{n:02}")).bind(format!("token-{n:02}"))
            .bind(if n == 0 { "Rare 100%_\\ name" } else { "Common" }).bind(n)
            .execute(&state.db).await.unwrap();
    }
    let admin = Client::admin(&router).await;
    let first = admin.request("GET", "/v1/admin/videos", Value::Null).await;
    assert_eq!(first.body["total"], 37);
    assert_eq!(first.body["videos"].as_array().unwrap().len(), 30);
    assert_eq!(first.body["has_more"], true);
    let last = admin
        .request("GET", "/v1/admin/videos?offset=30", Value::Null)
        .await;
    assert_eq!(last.body["total"], 37);
    assert_eq!(last.body["videos"].as_array().unwrap().len(), 7);
    assert_eq!(last.body["has_more"], false);
    for query in ["rare", "%25", "%5F", "%5C", "100%25%5F%5C"] {
        let result = admin
            .request("GET", &format!("/v1/admin/videos?q={query}"), Value::Null)
            .await;
        assert_eq!(result.status, StatusCode::OK);
        assert_eq!(result.body["total"], 1);
        assert_eq!(result.body["videos"][0]["id"], "id-00");
    }
    assert_eq!(
        admin
            .request(
                "GET",
                &format!("/v1/admin/videos?q={}", "a".repeat(201)),
                Value::Null
            )
            .await
            .status,
        StatusCode::BAD_REQUEST
    );
}

#[tokio::test]
/// An anonymous count cannot reveal private members, another album, or unfinished uploads.
async fn counts_preserve_album_authorization() {
    let (state, router, _directory) = setup().await;
    lifecycle_collection(&state).await;
    let visitor = Client::visitor(&router);
    let public = visitor
        .request("GET", "/v1/albums/nature?q=title", Value::Null)
        .await;
    assert_eq!(public.body["total"], 1);
    assert_eq!(public.body["videos"].as_array().unwrap().len(), 1);
    let hidden = visitor
        .request("GET", "/v1/albums/nature?q=private", Value::Null)
        .await;
    assert_eq!(hidden.body["total"], 0);
    assert_eq!(hidden.body["videos"], serde_json::json!([]));
    assert_eq!(
        visitor
            .request("GET", "/v1/admin/videos?q=private", Value::Null)
            .await
            .status,
        StatusCode::UNAUTHORIZED
    );
    let admin = Client::admin(&router).await;
    assert_eq!(
        admin
            .request("GET", "/v1/admin/albums/collection?q=title", Value::Null)
            .await
            .body["total"],
        2
    );
    assert_eq!(
        admin
            .request("GET", "/v1/albums/nature?q=private", Value::Null)
            .await
            .body["total"],
        1
    );
    assert_eq!(
        admin
            .request("GET", "/v1/admin/albums/other?q=title", Value::Null)
            .await
            .body["total"],
        0
    );
}
