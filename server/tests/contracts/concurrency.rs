// SPDX-FileCopyrightText: 2026 Cubeyond
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Adversarial request ordering remains isolated from real S3 and production metadata.

use crate::support::{login, media::Media, setup};
use axum::{
    body::{Body, Bytes},
    http::{Request, StatusCode},
};
use futures_util::{poll, stream};
use std::{convert::Infallible, time::Duration};
use tokio::sync::oneshot;
use tower::ServiceExt;

#[tokio::test]
/// Anonymous slow bodies are rejected before consuming upload buffers or configuration gates.
async fn unauthorized_upload_does_not_wait_for_its_body() {
    let (_, router, _directory) = setup().await;
    let body = Body::from_stream(stream::pending::<Result<Bytes, Infallible>>());
    let request = Request::builder()
        .method("POST")
        .uri("/v1/admin/uploads")
        .header("origin", "http://localhost:3000")
        .header("content-type", "application/json")
        .body(body)
        .unwrap();
    let response = tokio::time::timeout(Duration::from_millis(500), router.oneshot(request)).await;
    assert!(
        response.is_ok(),
        "unauthorized upload waited for an unfinished body"
    );
    assert_eq!(
        response.unwrap().unwrap().status(),
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
/// A queued settings writer cannot deadlock a poster already admitted to the storage gate.
async fn poster_finishes_when_storage_rotation_is_queued() {
    let media = Media::new(false).await;
    let upload = media.begin("private", 1024, "jpg").await;
    let (cookie, csrf) = login(&media.router).await;
    let (entered, accepted) = oneshot::channel();
    let (resume, released) = oneshot::channel();
    let body = Body::from_stream(stream::once(async move {
        entered.send(()).unwrap();
        released.await.unwrap();
        Ok::<_, Infallible>(Bytes::from_static(&[255, 216, 255, 217]))
    }));
    let request = Request::builder()
        .method("PUT")
        .uri(format!(
            "/v1/admin/videos/{}/poster",
            upload["video_id"].as_str().unwrap()
        ))
        .header("origin", "http://localhost:3000")
        .header("cookie", cookie)
        .header("x-csrf-token", csrf)
        .header("content-type", "image/jpeg")
        .body(body)
        .unwrap();
    let mut poster = tokio::spawn(media.router.clone().oneshot(request));
    accepted.await.unwrap();
    let mut rotation = Box::pin(media.state.storage.exclusive());
    assert!(poll!(rotation.as_mut()).is_pending());
    resume.send(()).unwrap();
    let response = tokio::time::timeout(Duration::from_secs(3), &mut poster).await;
    drop(rotation);
    if response.is_err() {
        poster.abort();
    }
    assert!(
        response.is_ok(),
        "poster recursively waited behind a queued configuration writer"
    );
    assert_eq!(
        response.unwrap().unwrap().unwrap().status(),
        StatusCode::NO_CONTENT
    );
}

#[tokio::test]
/// Cancellation must remove a poster whose S3 write was already in flight.
async fn cancellation_cleans_an_inflight_poster() {
    let media = Media::new(false).await;
    let upload = media.begin("private", 1024, "jpg").await;
    let id = upload["video_id"].as_str().unwrap();
    let video = mori::features::videos::get(&media.state, id).await.unwrap();
    let (cookie, csrf) = login(&media.router).await;
    let pause = media.pause_put();
    let request = Request::builder()
        .method("PUT")
        .uri(format!("/v1/admin/videos/{id}/poster"))
        .header("origin", "http://localhost:3000")
        .header("cookie", &cookie)
        .header("x-csrf-token", &csrf)
        .body(Body::from(vec![255, 216, 255, 217]))
        .unwrap();
    let poster = tokio::spawn(media.router.clone().oneshot(request));
    pause.entered().await;
    let request = Request::builder()
        .method("DELETE")
        .uri(format!(
            "/v1/admin/uploads/{}",
            upload["id"].as_str().unwrap()
        ))
        .header("origin", "http://localhost:3000")
        .header("cookie", cookie)
        .header("x-csrf-token", csrf)
        .body(Body::empty())
        .unwrap();
    let mut cancel = tokio::spawn(media.router.clone().oneshot(request));
    let early = tokio::time::timeout(Duration::from_millis(100), &mut cancel).await;
    pause.release();
    assert_eq!(
        poster.await.unwrap().unwrap().status(),
        StatusCode::NO_CONTENT
    );
    let canceled = match early {
        Ok(response) => response,
        Err(_) => cancel.await,
    };
    assert_eq!(canceled.unwrap().unwrap().status(), StatusCode::NO_CONTENT);
    assert!(
        !media.exists(video.private_poster()).await,
        "canceled upload left an orphan poster"
    );
}

#[tokio::test]
/// A hostile login origin is rejected without awaiting an attacker-controlled body.
async fn cross_origin_login_rejects_an_unfinished_body() {
    let (_, router, _directory) = setup().await;
    let request = Request::builder()
        .method("POST")
        .uri("/v1/auth/login")
        .header("origin", "https://evil.example")
        .header("content-type", "application/json")
        .body(Body::from_stream(stream::pending::<
            Result<Bytes, Infallible>,
        >()))
        .unwrap();
    let response = tokio::time::timeout(Duration::from_millis(500), router.oneshot(request)).await;
    assert!(response.is_ok(), "cross-origin login waited for a body");
    assert_eq!(response.unwrap().unwrap().status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
/// Every documented administrator endpoint authenticates before parsing an arbitrary body.
async fn admin_authentication_covers_the_entire_api() {
    let (_, router, _directory) = setup().await;
    let (cookie, csrf) = login(&router).await;
    let spec = serde_json::to_value(mori::http::openapi()).unwrap();
    for (path, operations) in spec["paths"].as_object().unwrap() {
        if !path.starts_with("/v1/admin/") {
            continue;
        }
        let path = path
            .split('/')
            .map(|segment| {
                if segment.starts_with('{') {
                    if segment == "{n}" {
                        "1"
                    } else {
                        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                    }
                } else {
                    segment
                }
            })
            .collect::<Vec<_>>()
            .join("/");
        for method in operations.as_object().unwrap().keys() {
            if !["get", "post", "put", "patch", "delete"].contains(&method.as_str()) {
                continue;
            }
            let variants = if method == "get" {
                vec![false]
            } else {
                vec![false, true]
            };
            for authenticated in variants {
                let mut request = Request::builder()
                    .method(method.to_uppercase().as_str())
                    .uri(&path)
                    .header("origin", "https://evil.example")
                    .header("content-type", "application/json");
                if authenticated {
                    request = request
                        .header("cookie", &cookie)
                        .header("x-csrf-token", &csrf);
                }
                let request = request
                    .body(Body::from_stream(stream::pending::<
                        Result<Bytes, Infallible>,
                    >()))
                    .unwrap();
                let response = tokio::time::timeout(
                    Duration::from_millis(500),
                    router.clone().oneshot(request),
                )
                .await;
                assert!(
                    response.is_ok(),
                    "{method} {path} read an unauthorized body"
                );
                assert_eq!(
                    response.unwrap().unwrap().status(),
                    if authenticated {
                        StatusCode::FORBIDDEN
                    } else {
                        StatusCode::UNAUTHORIZED
                    },
                    "{method} {path}"
                );
            }
        }
    }
}

#[tokio::test]
/// Saturation rejects extra upstream work while keeping the UI and health endpoint usable.
async fn control_admission_is_bounded_and_released() {
    let (state, router, _directory) = setup().await;
    let count = u32::try_from(state.request_slots.available_permits()).unwrap();
    let occupied = state.request_slots.acquire_many(count).await.unwrap();
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/v1/auth/session")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    drop(occupied);
    let response = router
        .oneshot(
            Request::builder()
                .uri("/v1/auth/session")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}
