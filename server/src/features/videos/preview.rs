//! Authenticated streaming with bounded buffering and HTTP range semantics.

use super::{MediaKind, get};
use crate::{
    app::AppState,
    features::auth,
    http::error::{Error, Result, bad},
};
use axum::{
    body::Body,
    extract::{Path, State},
    http::{HeaderMap, Method, StatusCode, header},
    response::Response,
};
use futures_util::StreamExt;
use std::time::Duration;
use tokio::sync::OwnedSemaphorePermit;
use tokio_util::io::ReaderStream;

/// Normalize one satisfiable HTTP byte range against an object length.
#[must_use]
pub fn byte_range(value: &str, size: u64) -> Option<(u64, u64)> {
    let value = value.strip_prefix("bytes=")?;
    if value.contains(',') || size == 0 {
        return None;
    }
    let (a, b) = value.split_once('-')?;
    if a.is_empty() {
        let suffix: u64 = b.parse().ok()?;
        if suffix == 0 {
            return None;
        }
        return Some((size.saturating_sub(suffix), size - 1));
    }
    let start: u64 = a.parse().ok()?;
    let end = if b.is_empty() {
        size - 1
    } else {
        b.parse::<u64>().ok()?.min(size - 1)
    };
    (start < size && end >= start).then_some((start, end))
}
/// Serve authenticated private video bytes with HEAD and Range support.
///
/// # Errors
/// Returns authentication, inaccessible-video, concurrency-limit, S3, or response-building errors.
#[utoipa::path(
    get,
    path = "/v1/admin/videos/{id}/content",
    params(("id" = String, Path)),
    responses(
        (status = 200, description = "Authenticated video stream"),
        (status = 206, description = "Partial video"),
        (status = 416, description = "Unsatisfiable range"),
    ),
    security(("session" = [])),
)]
pub async fn content(
    State(s): State<AppState>,
    h: HeaderMap,
    method: Method,
    Path(id): Path<String>,
) -> Result<Response> {
    stream(s, h, method, id, false).await
}
/// Serve or store a bounded authenticated JPEG poster.
///
/// # Errors
/// Returns authentication, inaccessible-poster, concurrency-limit, S3, or response-building errors.
#[utoipa::path(
    get,
    path = "/v1/admin/videos/{id}/poster",
    params(("id" = String, Path)),
    responses((status = 200, description = "Authenticated poster")),
    security(("session" = [])),
)]
pub async fn poster(
    State(s): State<AppState>,
    h: HeaderMap,
    method: Method,
    Path(id): Path<String>,
) -> Result<Response> {
    stream(s, h, method, id, true).await
}
/// Acquire an independent poster budget without rejecting a gallery's parallel cover requests.
///
/// # Errors
/// Returns 429 for exhausted video capacity or 503 for a timed-out/closed poster budget.
async fn stream_permit(s: &AppState, poster: bool) -> Result<OwnedSemaphorePermit> {
    if !poster {
        return s
            .preview_slots
            .clone()
            .try_acquire_owned()
            .map_err(|_| Error(StatusCode::TOO_MANY_REQUESTS, "Too many preview streams"));
    }
    tokio::time::timeout(
        Duration::from_secs(10),
        s.poster_slots.clone().acquire_owned(),
    )
    .await
    .map_err(|_| {
        Error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Poster service busy; retry shortly",
        )
    })?
    .map_err(|_| {
        Error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Poster service unavailable",
        )
    })
}

/// Authenticate each media request and retain the stream concurrency permit.
///
/// # Errors
/// Returns authentication, inaccessible-media, concurrency-limit, S3, or response-building errors.
async fn stream(
    s: AppState,
    h: HeaderMap,
    method: Method,
    id: String,
    poster: bool,
) -> Result<Response> {
    auth::require(&s, &h, false).await?;
    let v = get(&s, &id).await?;
    if matches!(
        v.state.as_str(),
        "uploading" | "verifying" | "failed" | "deleting"
    ) || (poster && v.poster == 0)
    {
        return Err(crate::http::error::missing());
    }
    let permit = stream_permit(&s, poster).await?;
    let key = v.content_key(poster);
    let head = s
        .storage
        .get()?
        .s3
        .head_object()
        .bucket(&s.storage.get()?.bucket)
        .key(&key)
        .send()
        .await
        .map_err(anyhow::Error::from)?;
    let size = u64::try_from(head.content_length.unwrap_or(0))
        .map_err(|_| bad("Invalid stored object length"))?;
    let mut builder = Response::builder()
        .header(
            header::CONTENT_TYPE,
            if poster {
                "image/jpeg"
            } else {
                super::content_type(&v.extension)
            },
        )
        .header(header::ACCEPT_RANGES, "bytes")
        .header(header::CACHE_CONTROL, "private, no-store")
        .header("x-content-type-options", "nosniff");
    if !poster && v.kind == MediaKind::Image {
        builder = builder.header(
            header::CONTENT_SECURITY_POLICY,
            "sandbox; default-src 'none'; img-src data:; style-src 'unsafe-inline'",
        );
    }
    if let Some(etag) = &head.e_tag {
        builder = builder.header(header::ETAG, etag);
    }
    let range = if method == Method::GET
        && h.get(header::IF_RANGE).is_none_or(|value| {
            Some(value.as_bytes()) == head.e_tag.as_ref().map(std::string::String::as_bytes)
        }) {
        h.get(header::RANGE)
            .map(|v| v.to_str().ok().and_then(|r| byte_range(r, size)))
    } else {
        None
    };
    let mut request = s
        .storage
        .get()?
        .s3
        .get_object()
        .bucket(&s.storage.get()?.bucket)
        .key(key);
    let length = match range {
        Some(Some((start, end))) => {
            builder = builder
                .status(StatusCode::PARTIAL_CONTENT)
                .header(header::CONTENT_RANGE, format!("bytes {start}-{end}/{size}"));
            request = request.range(format!("bytes={start}-{end}"));
            end - start + 1
        }
        Some(None) => {
            return builder
                .status(StatusCode::RANGE_NOT_SATISFIABLE)
                .header(header::CONTENT_RANGE, format!("bytes */{size}"))
                .body(Body::empty())
                .map_err(|_| bad("Invalid range response"));
        }
        None => size,
    };
    builder = builder.header(header::CONTENT_LENGTH, length);
    if method == Method::HEAD {
        return builder
            .body(Body::empty())
            .map_err(|_| bad("Invalid HEAD response"));
    }
    let object = request.send().await.map_err(anyhow::Error::from)?;
    let stream =
        ReaderStream::with_capacity(object.body.into_async_read(), 64 * 1024).map(move |chunk| {
            let _keep_permit = &permit;
            chunk
        });
    builder
        .body(Body::from_stream(stream))
        .map_err(|_| bad("Invalid media response"))
}
