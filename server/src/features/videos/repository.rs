// SPDX-FileCopyrightText: 2026 Cubeyond
// SPDX-License-Identifier: Elastic-2.0

//! Metadata access keeps visitor filtering and collection ordering in one place.

use super::{Video, VideoPage};
use crate::{
    app::AppState,
    http::{
        error::{Result, bad, missing},
        pagination::Page,
    },
};
use sqlx::{QueryBuilder, Sqlite};

/// Load a video record that has not completed deletion.
///
/// # Errors
/// Returns 404 for absent or deleted videos, or a database error.
pub async fn get(s: &AppState, id: &str) -> Result<Video> {
    sqlx::query_as("SELECT * FROM videos WHERE id=? AND state!='deleted'")
        .bind(id)
        .fetch_optional(&s.db)
        .await?
        .ok_or_else(missing)
}
/// Closed query scopes prevent constructing an anonymous global listing.
#[derive(Clone, Copy)]
pub enum Collection<'a> {
    /// Every non-deleted video, after administrator authentication.
    Library,
    /// All non-deleted members of an administrator-visible album.
    Album(&'a str),
    /// Only published public members of an already authorized public album.
    PublicAlbum(&'a str),
}

/// Fetch one stable page and attach links appropriate to its authorized scope.
///
/// # Errors
/// Returns database or pagination conversion errors.
pub async fn collection(state: &AppState, scope: Collection<'_>, page: &Page) -> Result<VideoPage> {
    let filter = page.q.as_deref().unwrap_or_default().trim();
    if filter.chars().count() > 200 {
        return Err(bad("Search must be at most 200 characters"));
    }
    let pattern = format!(
        "%{}%",
        filter
            .replace('\\', "\\\\")
            .replace('%', "\\%")
            .replace('_', "\\_")
    );
    let mut query = QueryBuilder::<Sqlite>::new("SELECT * FROM videos WHERE ");
    constrain(&mut query, scope, &pattern);
    let mut count = QueryBuilder::<Sqlite>::new("SELECT COUNT(*) FROM videos WHERE ");
    constrain(&mut count, scope, &pattern);
    let mut tx = state.db.begin().await?;
    let total: i64 = count.build_query_scalar().fetch_one(&mut *tx).await?;
    let limit =
        i64::try_from(page.limit()).map_err(|_| bad("Page size exceeds the supported range"))?;
    query
        .push(" ORDER BY created_at DESC,id DESC LIMIT ")
        .push_bind(limit + 1)
        .push(" OFFSET ")
        .push_bind(page.offset());
    let mut rows: Vec<Video> = query.build_query_as().fetch_all(&mut *tx).await?;
    tx.commit().await?;
    let has_more = rows.len() > page.limit();
    rows.truncate(page.limit());
    Ok(VideoPage {
        total,
        videos: rows
            .into_iter()
            .map(|video| video.links(state, !matches!(scope, Collection::PublicAlbum(_))))
            .collect(),
        has_more,
    })
}

/// Apply identical visibility and literal-name predicates to counts and result pages.
fn constrain(query: &mut QueryBuilder<Sqlite>, scope: Collection<'_>, pattern: &str) {
    match scope {
        Collection::Library => {
            query.push("state!='deleted' AND state NOT IN ('failed','uploading','verifying')");
        }
        Collection::Album(id) => {
            query
                .push("state!='deleted' AND state NOT IN ('failed','uploading','verifying') AND album_id=")
                .push_bind(id);
        }
        Collection::PublicAlbum(id) => {
            query
                .push("visibility='public' AND state='published' AND album_id=")
                .push_bind(id);
        }
    }
    query
        .push(" AND name LIKE ")
        .push_bind(pattern)
        .push(" ESCAPE '\\'");
}
