//! Authorize every stable public address before selecting S3 or optional CDN delivery.

use super::Video;
use crate::{
    app::{AppState, valid_token},
    http::error::{Result, missing},
};
use axum::{
    extract::{Path, State},
    http::Method,
    response::Redirect,
};

/// Resolve an issued album or standalone address without proxying video bytes.
///
/// # Errors
/// Returns 404 for private, unpublished, or mismatched paths, or storage/database errors.
pub async fn shared_resource(
    State(s): State<AppState>,
    Path((prefix, filename)): Path<(String, String)>,
    method: Method,
) -> Result<Redirect> {
    let (token, extension) = filename.rsplit_once('.').ok_or_else(missing)?;
    if !valid_token(token) {
        return Err(missing());
    }
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM videos v JOIN media_aliases a ON a.video_id=v.id WHERE v.token=? AND v.extension=? AND a.prefix=? AND v.visibility='public' AND v.state='published')",
    ).bind(token).bind(extension).bind(prefix).fetch_one(&s.db).await?;
    if !exists {
        return Err(missing());
    }
    let storage = s.storage.get()?;
    let target = if storage.distribution_id.is_some() {
        format!("{}/files/{token}.{extension}", s.config.public_url)
    } else {
        storage
            .public_url(token, extension, method == Method::HEAD)
            .await?
    };
    Ok(Redirect::temporary(&target))
}

/// Resolve an internal published video or poster when no CDN handles `/files/*`.
///
/// This namespace is not a directory. Knowing a token never bypasses visibility checks.
///
/// # Errors
/// Returns 404 for private, unpublished, absent posters, or CDN-managed resources.
pub async fn public_file(
    State(s): State<AppState>,
    Path(filename): Path<String>,
    method: Method,
) -> Result<Redirect> {
    let (token, extension) = filename.rsplit_once('.').ok_or_else(missing)?;
    if !valid_token(token) {
        return Err(missing());
    }
    let storage = s.storage.get()?;
    if storage.distribution_id.is_some() {
        return Err(missing());
    }
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM videos WHERE token=? AND visibility='public' AND state='published' AND (extension=? OR (?='jpg' AND poster!=0 AND poster_key IS NULL)))",
    ).bind(token).bind(extension).bind(extension).fetch_one(&s.db).await?;
    if !exists {
        return Err(missing());
    }
    Ok(Redirect::temporary(
        &storage
            .public_url(token, extension, method == Method::HEAD)
            .await?,
    ))
}

/// Deliver a published JPEG derivative without confusing it with a JPEG original.
///
/// # Errors
/// Returns 404 for an inaccessible/missing preview, or storage and database errors.
pub async fn public_preview(
    State(s): State<AppState>,
    Path(filename): Path<String>,
    method: Method,
) -> Result<Redirect> {
    let token = filename
        .strip_suffix(".jpg")
        .filter(|token| valid_token(token))
        .ok_or_else(missing)?;
    let media: Video = sqlx::query_as("SELECT * FROM videos WHERE token=? AND visibility='public' AND state='published' AND poster!=0 AND poster_key IS NOT NULL")
        .bind(token).fetch_optional(&s.db).await?.ok_or_else(missing)?;
    Ok(Redirect::temporary(
        &s.storage
            .get()?
            .published_url(media.public_poster(), method == Method::HEAD)
            .await?,
    ))
}
