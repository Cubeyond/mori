// SPDX-FileCopyrightText: 2026 Cubeyond
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Bounded administrator album covers, read without contacting object storage.

use super::{Album, AlbumPreview, AlbumSummary};
use crate::{app::AppState, features::videos::MediaKind, http::error::Result};
use sqlx::FromRow;
use std::collections::HashMap;

/// Minimal cover metadata keeps the album response independent of full media records.
#[derive(FromRow)]
struct PreviewRow {
    /// Owning collection used only to assemble the administrator response.
    album_id: String,
    /// Stable media identifier for authenticated poster requests.
    id: String,
    /// Images omit the video play affordance.
    kind: MediaKind,
}

/// Read one consistent listing with at most three completed covers per album.
///
/// # Errors
/// Returns database failures; callers must authenticate before invoking this query.
pub(super) async fn summaries(state: &AppState) -> Result<Vec<AlbumSummary>> {
    let mut tx = state.db.begin().await?;
    let albums: Vec<Album> = sqlx::query_as(
        "SELECT id,name,slug,visibility,description,created_at FROM albums ORDER BY created_at DESC,id DESC",
    )
    .fetch_all(&mut *tx)
    .await?;
    let rows: Vec<PreviewRow> = sqlx::query_as(
        r"SELECT v.album_id,v.id,v.kind FROM albums a JOIN videos v ON v.id IN (
            SELECT id FROM videos WHERE album_id=a.id AND poster!=0
            AND state IN ('ready','published') ORDER BY created_at DESC,id DESC LIMIT 3
        ) ORDER BY v.created_at DESC,v.id DESC",
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    let mut previews: HashMap<String, Vec<AlbumPreview>> = HashMap::new();
    for row in rows {
        let poster_url = format!("/v1/admin/videos/{}/poster", row.id);
        previews
            .entry(row.album_id)
            .or_default()
            .push(AlbumPreview {
                id: row.id,
                kind: row.kind,
                poster_url,
            });
    }
    Ok(albums
        .into_iter()
        .map(|album| AlbumSummary {
            previews: previews.remove(&album.id).unwrap_or_default(),
            album,
        })
        .collect())
}
