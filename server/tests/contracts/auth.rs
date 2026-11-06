// SPDX-FileCopyrightText: 2026 Cubeyond
// SPDX-License-Identifier: Elastic-2.0

//! Auth HTTP contracts with isolated state.

use crate::support::{call, login, setup};
use axum::http::StatusCode;
use serde_json::Value;
use serde_json::json;

#[tokio::test]
/// Environment credentials preserve existing sessions until the name or password changes.
async fn configured_credentials_contract() {
    let (state, router, _directory) = setup().await;
    let (cookie, _) = login(&router).await;
    mori::features::auth::configure(&state.db, "root", Some("a-long-test-password".into()))
        .await
        .unwrap();
    assert_eq!(
        call(
            &router,
            "GET",
            "/v1/admin/videos",
            Some(&cookie),
            None,
            Value::Null
        )
        .await
        .0,
        StatusCode::OK
    );
    mori::features::auth::configure(&state.db, "owner", Some("a-new-long-password".into()))
        .await
        .unwrap();
    assert_eq!(
        call(
            &router,
            "GET",
            "/v1/admin/videos",
            Some(&cookie),
            None,
            Value::Null
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    let result = call(
        &router,
        "POST",
        "/v1/auth/login",
        None,
        None,
        json!({"username":"owner","password":"a-new-long-password"}),
    )
    .await;
    assert_eq!(result.0, StatusCode::OK);
    let stored: (String, String) =
        sqlx::query_as("SELECT username,password_hash FROM admin WHERE id=1")
            .fetch_one(&state.db)
            .await
            .unwrap();
    assert_eq!(stored.0, "owner");
    assert_ne!(stored.1, "a-new-long-password");
}
