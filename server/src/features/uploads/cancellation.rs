// SPDX-FileCopyrightText: 2026 Cubeyond
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Server-discovered upload recovery never depends on local browser session metadata.

use super::record;
use crate::{
    app::{AppState, now, valid_token},
    features::{auth::Admin, videos},
    http::error::{Error, Result, bad},
};
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;

/// Non-secret queue metadata remains available after browser storage has been cleared.
#[derive(Serialize, FromRow, ToSchema)]
pub struct UnfinishedUpload {
    /// Multipart reservation identity; no AWS upload credentials are exposed.
    pub id: String,
    /// Media record associated with this unfinished original.
    pub video_id: String,
    /// Album whose deletion may be blocked by this upload.
    #[schema(required = true)]
    pub album_id: Option<String>,
    /// Server-assigned display hash for the unfinished original.
    pub name: String,
    /// Expected source length in bytes.
    pub size: i64,
}

/// A bounded snapshot prevents a cancellation from reaching newly started unrelated sessions.
#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CancelUploads {
    /// Exact multipart reservations explicitly cancelled by the administrator.
    pub ids: Vec<String>,
}

/// Discover reservations independently of optional local resume fingerprints.
///
/// # Errors
/// Returns authentication or database failures.
#[utoipa::path(get, path = "/v1/admin/uploads", responses((status = 200, body = Vec<UnfinishedUpload>)), security(("session" = [])))]
pub async fn unfinished(
    State(s): State<AppState>,
    _admin: Admin,
) -> Result<Json<Vec<UnfinishedUpload>>> {
    Ok(Json(sqlx::query_as("SELECT u.id,u.video_id,v.album_id,v.name,v.size FROM uploads u JOIN videos v ON v.id=u.video_id WHERE u.state IN ('uploading','canceling') ORDER BY v.created_at,u.id LIMIT 100").fetch_all(&s.db).await?))
}

/// Cancel an explicit queue snapshot, retaining successful cancellations if storage requires retry.
///
/// # Errors
/// Returns authentication, CSRF, invalid selection, verification conflicts, or storage failures.
#[utoipa::path(delete, path = "/v1/admin/uploads", request_body = CancelUploads, responses((status = 204, description = "Uploads cancelled")), security(("session" = [])))]
pub async fn cancel_many(
    State(s): State<AppState>,
    _admin: Admin,
    Json(input): Json<CancelUploads>,
) -> Result<StatusCode> {
    if input.ids.is_empty() || input.ids.len() > 100 || input.ids.iter().any(|id| !valid_token(id))
    {
        return Err(bad("Select between 1 and 100 upload reservations"));
    }
    for id in input.ids {
        cancel_one(&s, &id, false).await?;
    }
    Ok(StatusCode::NO_CONTENT)
}

/// Cancel one original; retrying a cancelled or interrupted cancellation remains safe.
///
/// # Errors
/// Returns authentication, CSRF, missing reservation, verification conflicts, or storage failures.
#[utoipa::path(delete, path = "/v1/admin/uploads/{id}", params(("id" = String, Path)), responses((status = 204, description = "Aborted")), security(("session" = [])))]
pub async fn cancel(
    State(s): State<AppState>,
    _admin: Admin,
    Path(id): Path<String>,
) -> Result<StatusCode> {
    cancel_one(&s, &id, false).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Persist cancellation before S3 operations so the cleanup worker can retry after a process crash.
async fn cancel_one(s: &AppState, id: &str, expired_only: bool) -> Result<()> {
    let reservation = record(s, id).await?;
    let _transition = s.upload_lifecycle.lock(&reservation.video_id).await;
    let reservation = record(s, id).await?;
    if expired_only
        && !(reservation.state == "canceling"
            || (reservation.state == "uploading" && reservation.expires_at <= now()))
    {
        return Ok(());
    }
    if reservation.state == "canceled" {
        return Ok(());
    }
    let changed = sqlx::query(
        "UPDATE uploads SET state='canceling' WHERE id=? AND state IN ('uploading','canceling')",
    )
    .bind(id)
    .execute(&s.db)
    .await?
    .rows_affected();
    if changed == 0 {
        if record(s, id).await?.state == "canceled" {
            return Ok(());
        }
        return Err(Error(
            StatusCode::CONFLICT,
            "Cannot cancel during verification",
        ));
    }
    let video = videos::get(s, &reservation.video_id).await?;
    let storage = s.storage.get()?;
    storage
        .abort(&video.private_source(), &reservation.upload_id)
        .await?;
    storage.delete(video.private_source()).await?;
    storage.delete(video.private_poster()).await?;
    let mut tx = s.db.begin().await?;
    sqlx::query(
        "UPDATE videos SET state='failed',visibility='private' WHERE id=? AND state='uploading'",
    )
    .bind(&reservation.video_id)
    .execute(&mut *tx)
    .await?;
    sqlx::query("UPDATE uploads SET state='canceled' WHERE id=? AND state='canceling'")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

/// Recheck expiration under the lifecycle lock before retrying durable cancellation.
///
/// # Errors
/// Returns database or object-storage errors; a later cleanup pass can safely retry.
pub(crate) async fn cleanup(s: &AppState, id: &str) -> Result<()> {
    cancel_one(s, id, true).await
}
