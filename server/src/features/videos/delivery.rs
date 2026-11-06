// SPDX-FileCopyrightText: 2026 Cubeyond
// SPDX-License-Identifier: Elastic-2.0

//! Authorize stable public addresses before streaming original bytes from private S3 storage.

use super::{Video, preview};
use crate::{
    app::{AppState, valid_token},
    http::error::{Result, missing},
};
use axum::{
    extract::{Path, State},
    http::{HeaderMap, Method},
    response::Response,
};

/// Serve an issued album or standalone address without redirecting the browser.
///
/// # Errors
/// Returns 404 for private, unpublished, or mismatched paths, or storage/database errors.
pub async fn shared_resource(
    State(s): State<AppState>,
    Path((prefix, filename)): Path<(String, String)>,
    headers: HeaderMap,
    method: Method,
) -> Result<Response> {
    let (token, extension) = filename.rsplit_once('.').ok_or_else(missing)?;
    if !valid_token(token) {
        return Err(missing());
    }
    let media: Video = sqlx::query_as(
        "SELECT v.* FROM videos v JOIN media_aliases a ON a.video_id=v.id WHERE v.token=? AND v.extension=? AND a.prefix=? AND v.visibility='public' AND v.state='published'",
    ).bind(token).bind(extension).bind(prefix).fetch_optional(&s.db).await?.ok_or_else(missing)?;
    preview::resource(&s, &headers, method, &media, false).await
}

/// Preserve internal original and historical poster addresses without exposing S3 signatures.
///
/// # Errors
/// Returns 404 for inaccessible resources or CDN-owned internal addresses, or storage errors.
pub async fn public_file(
    State(s): State<AppState>,
    Path(filename): Path<String>,
    headers: HeaderMap,
    method: Method,
) -> Result<Response> {
    let (token, extension) = filename.rsplit_once('.').ok_or_else(missing)?;
    if !valid_token(token) || s.storage.get()?.distribution_id.is_some() {
        return Err(missing());
    }
    let media: Video = sqlx::query_as(
        "SELECT * FROM videos WHERE token=? AND visibility='public' AND state='published' AND (extension=? OR (?='jpg' AND poster!=0 AND extension!='jpg'))",
    ).bind(token).bind(extension).bind(extension).fetch_optional(&s.db).await?.ok_or_else(missing)?;
    let poster = media.extension != extension;
    preview::resource(&s, &headers, method, &media, poster).await
}

/// Serve a published JPEG derivative independently of an original JPEG image.
///
/// # Errors
/// Returns 404 for an inaccessible/missing preview, or storage and database errors.
pub async fn public_preview(
    State(s): State<AppState>,
    Path(filename): Path<String>,
    headers: HeaderMap,
    method: Method,
) -> Result<Response> {
    let token = filename
        .strip_suffix(".jpg")
        .filter(|token| valid_token(token))
        .ok_or_else(missing)?;
    let media: Video = sqlx::query_as("SELECT * FROM videos WHERE token=? AND visibility='public' AND state='published' AND poster!=0")
        .bind(token).fetch_optional(&s.db).await?.ok_or_else(missing)?;
    preview::resource(&s, &headers, method, &media, true).await
}
