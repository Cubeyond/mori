// SPDX-FileCopyrightText: 2026 Cubeyond
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Atomic bulk operations share album validation and durable media deletion services.

use super::{album_prefix, deletion};
use crate::{
    app::{AppState, valid_token},
    features::{
        albums::{self, Album, CreateAlbum},
        auth::Admin,
    },
    http::error::{Error, Result, bad, missing},
};
use axum::{Json, extract::State, http::StatusCode};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use utoipa::ToSchema;

/// One bounded, explicitly selected set; unknown fields cannot broaden destructive scope.
#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct BatchInput {
    /// Unique internal media identifiers, capped at one hundred per operation.
    pub ids: Vec<String>,
    /// Membership change, new collection, or permanently confirmed deletion.
    pub action: BatchAction,
}

/// Existing or newly created album membership remains independent of media visibility.
#[derive(Deserialize, ToSchema)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum BatchAction {
    /// Set album membership; a null identifier detaches originals into Gallery.
    Move {
        /// Existing album identifier or explicit standalone membership.
        album_id: Option<String>,
    },
    /// Validate and create an album in the same transaction as all membership changes.
    CreateAlbum {
        /// The normal album creation contract, including a separately validated URL slug.
        album: CreateAlbum,
    },
    /// Queue durable deletion of every selected original without touching unselected records.
    Delete,
}

/// Successful selection changes, optionally containing jobs and a newly created album.
#[derive(Serialize, ToSchema)]
pub struct BatchResult {
    /// Number of distinct selected records changed atomically.
    pub affected: usize,
    /// Persisted jobs whose completion confirms permanent object removal.
    pub job_ids: Vec<String>,
    /// New collection created by this operation; absent for existing membership or deletion.
    #[schema(required = true)]
    pub album: Option<Album>,
}

/// Apply exactly one atomic operation to a bounded, authorized selection.
///
/// # Errors
/// Returns authentication, CSRF, invalid selection/album, missing records, or state conflicts.
#[utoipa::path(post, path = "/v1/admin/videos/batch", request_body = BatchInput,
    responses((status = 202, body = BatchResult), (status = 409, description = "Media changed")),
    security(("session" = [])))]
pub async fn apply(
    State(s): State<AppState>,
    _admin: Admin,
    Json(input): Json<BatchInput>,
) -> Result<(StatusCode, Json<BatchResult>)> {
    if input.ids.is_empty()
        || input.ids.len() > 100
        || input.ids.iter().any(|id| !valid_token(id))
        || input.ids.iter().collect::<HashSet<_>>().len() != input.ids.len()
    {
        return Err(bad("Select between 1 and 100 distinct media items"));
    }
    let mut tx = s.db.begin_with("BEGIN IMMEDIATE").await?;
    let mut jobs = Vec::new();
    let album = match input.action {
        BatchAction::Delete => {
            for id in &input.ids {
                jobs.push(deletion::enqueue(&mut tx, id).await?);
            }
            None
        }
        BatchAction::Move { album_id } => {
            move_selection(&mut tx, &input.ids, album_id.as_deref()).await?;
            None
        }
        BatchAction::CreateAlbum { album } => {
            let created = albums::insert(&mut tx, album).await?;
            move_selection(&mut tx, &input.ids, Some(&created.id)).await?;
            Some(created)
        }
    };
    tx.commit().await?;
    s.job_wakeup.notify_one();
    Ok((
        StatusCode::ACCEPTED,
        Json(BatchResult {
            affected: input.ids.len(),
            job_ids: jobs,
            album,
        }),
    ))
}

/// Change membership in the caller's transaction without touching source keys or access policy.
async fn move_selection(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    ids: &[String],
    album: Option<&str>,
) -> Result<()> {
    let prefix = album_prefix(tx, album).await?;
    for media in ids {
        let state: String =
            sqlx::query_scalar("SELECT state FROM videos WHERE id=? AND state!='deleted'")
                .bind(media)
                .fetch_optional(&mut **tx)
                .await?
                .ok_or_else(missing)?;
        if !matches!(state.as_str(), "ready" | "published") {
            return Err(Error(
                StatusCode::CONFLICT,
                "Wait for selected media operations to finish",
            ));
        }
        sqlx::query("UPDATE videos SET album_id=?,media_prefix=?,revision=revision+1 WHERE id=?")
            .bind(album)
            .bind(&prefix)
            .bind(media)
            .execute(&mut **tx)
            .await?;
        sqlx::query("INSERT OR IGNORE INTO media_aliases(video_id,prefix) VALUES(?,?)")
            .bind(media)
            .bind(&prefix)
            .execute(&mut **tx)
            .await?;
    }
    Ok(())
}
