// SPDX-FileCopyrightText: 2026 Cubeyond
// SPDX-License-Identifier: Elastic-2.0

//! Preserve one active object pair while upgrading legacy paths and changing visibility.

use super::Job;
use crate::{
    app::AppState,
    features::videos::{self, Video},
    infra::storage::Storage,
};

/// Remove obsolete legacy locations only after their replacement was verified.
///
/// # Errors
/// Returns object deletion failures so the durable operation can retry.
pub(super) async fn remove_legacy(storage: &Storage, video: &Video) -> anyhow::Result<()> {
    storage.delete(Storage::source(&video.id)).await?;
    storage.delete(Storage::poster(&video.id)).await?;
    Ok(())
}

/// Restore private objects before revoking public playback, including after an interrupted move.
///
/// # Errors
/// Returns copy, integrity, or metadata failures without deleting the only available object.
pub(super) async fn restore_private(state: &AppState, video: &Video) -> anyhow::Result<()> {
    let storage = state.storage.get()?;
    storage
        .copy_verified(
            Storage::public(&video.token, &video.extension),
            Storage::private(&video.token, &video.extension),
            videos::content_type(&video.extension),
            Some(video.size),
        )
        .await?;
    if video.poster != 0 {
        storage
            .copy_verified(
                video.public_poster(),
                video
                    .poster_key
                    .clone()
                    .unwrap_or_else(|| Storage::private_poster(&video.token)),
                "image/jpeg",
                None,
            )
            .await?;
    }
    sqlx::query("UPDATE videos SET source_key=? WHERE id=?")
        .bind(Storage::private(&video.token, &video.extension))
        .bind(&video.id)
        .execute(&state.db)
        .await?;
    remove_legacy(&storage, video).await
}

/// Normalize existing objects without changing sharing URLs, identifiers, or visibility.
///
/// # Errors
/// Returns storage or database failures; the old location remains readable until replacement.
pub(super) async fn normalize(state: &AppState, job: &Job, video: &Video) -> anyhow::Result<()> {
    let storage = state.storage.get()?;
    let public = video.state == "published";
    let destination = if public {
        Storage::public(&video.token, &video.extension)
    } else {
        Storage::private(&video.token, &video.extension)
    };
    let poster = if public {
        Storage::published_preview(&video.token)
    } else {
        Storage::private_preview(&video.token)
    };
    storage
        .copy_verified(
            video.private_source(),
            destination,
            videos::content_type(&video.extension),
            Some(video.size),
        )
        .await?;
    if video.poster != 0 {
        storage
            .copy_verified(video.content_key(true), poster, "image/jpeg", None)
            .await?;
    }
    sqlx::query("UPDATE videos SET source_key=?,poster_key=? WHERE id=?")
        .bind(Storage::private(&video.token, &video.extension))
        .bind(Storage::private_preview(&video.token))
        .bind(&video.id)
        .execute(&state.db)
        .await?;
    remove_legacy(&storage, video).await?;
    remove_old_previews(&storage, video).await?;
    if public {
        storage
            .delete(Storage::private(&video.token, &video.extension))
            .await?;
        storage
            .delete(Storage::private_preview(&video.token))
            .await?;
    }
    sqlx::query("UPDATE jobs SET state='complete',error=NULL WHERE id=?")
        .bind(&job.id)
        .execute(&state.db)
        .await?;
    Ok(())
}

/// Delete only obsolete derivative paths, preserving identically named JPEG originals.
///
/// # Errors
/// Returns deletion failures; the verified replacement and durable job remain available.
async fn remove_old_previews(storage: &Storage, video: &Video) -> anyhow::Result<()> {
    storage
        .delete(format!("private/posters/{}.jpg", video.token))
        .await?;
    if video.extension != "jpg" {
        storage
            .delete(Storage::private_poster(&video.token))
            .await?;
        storage.delete(Storage::public_poster(&video.token)).await?;
    }
    Ok(())
}
