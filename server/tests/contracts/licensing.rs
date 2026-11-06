// SPDX-FileCopyrightText: 2026 Cubeyond
// SPDX-License-Identifier: Elastic-2.0

//! Browser recipients can retrieve the actual terms without an administrator account.

use crate::support::setup;
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use std::sync::Arc;
use tower::ServiceExt;

#[tokio::test]
/// Legal resources are served as exact static files for anonymous GET and HEAD requests.
async fn distribution_notices_are_public() {
    let (mut state, _, directory) = setup().await;
    let mut config = (*state.config).clone();
    config.web_dir = directory.path().to_str().unwrap().to_owned();
    state.config = Arc::new(config);
    let contents = [
        ("LICENSE.txt", include_str!("../../../LICENSE")),
        ("NOTICE.txt", include_str!("../../../NOTICE")),
        ("THIRD-PARTY-NOTICES.json", "[]"),
    ];
    for (name, text) in contents {
        std::fs::write(directory.path().join(name), text).unwrap();
    }
    let router = mori::http::router(state);
    for (name, text) in contents {
        for method in ["GET", "HEAD"] {
            let response = router
                .clone()
                .oneshot(
                    Request::builder()
                        .method(method)
                        .uri(format!("/{name}"))
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK);
            assert_eq!(response.headers()["content-length"], text.len().to_string());
            let bytes = axum::body::to_bytes(response.into_body(), 1024 * 1024)
                .await
                .unwrap();
            assert_eq!(
                bytes.as_ref(),
                if method == "GET" {
                    text.as_bytes()
                } else {
                    &[]
                }
            );
        }
    }
}
