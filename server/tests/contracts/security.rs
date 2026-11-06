// SPDX-FileCopyrightText: 2026 Cubeyond
// SPDX-License-Identifier: Elastic-2.0

//! Security HTTP contracts with isolated state.

use crate::support::{call, setup};
use axum::body::Body;
use axum::http::Request;
use axum::http::StatusCode;
use mori::features::albums::normalize_slug;
use mori::features::videos::preview::byte_range;
use mori::http;
use serde_json::Value;
use std::sync::Arc;
use tower::ServiceExt;

#[tokio::test]
/// Verify origin rejection, framing boundaries, and restricted SPA fallbacks.
async fn origin_framing_and_missing_routes() {
    let (s, r, _dir) = setup().await;
    let request = Request::builder()
        .method("POST")
        .uri("/v1/auth/login")
        .header("origin", "https://evil.example")
        .header("content-type", "application/json")
        .body(Body::from(r#"{"password":"a-long-test-password"}"#))
        .unwrap();
    assert_eq!(
        r.clone().oneshot(request).await.unwrap().status(),
        StatusCode::FORBIDDEN
    );
    for path in [
        "/v1/not-a-route",
        "/files/unknown.mp4",
        "/files/",
        "/watch/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    ] {
        assert_eq!(
            call(&r, "GET", path, None, None, Value::Null).await.0,
            StatusCode::NOT_FOUND
        );
    }
    for (path, ancestor) in [
        ("/login", "frame-ancestors 'none'"),
        ("/gallery", "frame-ancestors 'none'"),
        ("/albums", "frame-ancestors 'none'"),
        ("/settings/preferences", "frame-ancestors 'none'"),
        ("/settings/storage", "frame-ancestors 'none'"),
    ] {
        let (status, headers, _) = call(&r, "GET", path, None, None, Value::Null).await;
        assert_eq!(status, StatusCode::OK);
        assert!(
            headers["content-security-policy"]
                .to_str()
                .unwrap()
                .contains(ancestor)
        );
        assert!(
            headers["cache-control"]
                .to_str()
                .unwrap()
                .contains("no-store")
        );
    }
    let mut protected = s;
    protected.origin_secret = Some(Arc::new("very-long-test-origin-secret-value".into()));
    let protected = http::router(protected);
    assert_eq!(
        call(&protected, "GET", "/", None, None, Value::Null)
            .await
            .0,
        StatusCode::FORBIDDEN
    );
}

#[tokio::test]
/// Old page bookmarks retain their query strings through GET and HEAD redirects.
async fn legacy_pages_redirect_to_canonical_destinations() {
    let (_, router, _directory) = setup().await;
    for method in ["GET", "HEAD"] {
        for (path, destination) in [
            ("/videos?upload=1", "/gallery?upload=1"),
            ("/settings", "/settings/preferences"),
            (
                "/settings?appearance=theme",
                "/settings/preferences?appearance=theme",
            ),
        ] {
            let (status, headers, _) = call(&router, method, path, None, None, Value::Null).await;
            assert_eq!(status, StatusCode::PERMANENT_REDIRECT);
            assert_eq!(headers["location"], destination);
        }
    }
}

#[test]
/// Cover range normalization and reserved path validation boundaries.
fn byte_ranges_and_slug_validation() {
    for (range, result) in [
        ("bytes=0-9", Some((0, 9))),
        ("bytes=90-", Some((90, 99))),
        ("bytes=-10", Some((90, 99))),
        ("bytes=0-999", Some((0, 99))),
        ("bytes=100-", None),
        ("bytes=-0", None),
        ("bytes=9-1", None),
        ("bytes=0-1,3-4", None),
    ] {
        assert_eq!(byte_range(range, 100), result, "{range}");
    }
    assert!(normalize_slug("_app").is_err());
    assert_eq!(normalize_slug("Kyoto-2026").unwrap(), "kyoto-2026");
}
