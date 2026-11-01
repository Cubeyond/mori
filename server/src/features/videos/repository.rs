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
    let mut query = QueryBuilder::<Sqlite>::new("SELECT * FROM videos WHERE ");
    match scope {
        Collection::Library => {
            query.push("state!='deleted'");
        }
        Collection::Album(id) => {
            query.push("state!='deleted' AND album_id=").push_bind(id);
        }
        Collection::PublicAlbum(id) => {
            query
                .push("visibility='public' AND state='published' AND album_id=")
                .push_bind(id);
        }
    }
    let limit =
        i64::try_from(page.limit()).map_err(|_| bad("Page size exceeds the supported range"))?;
    query
        .push(" ORDER BY created_at DESC,id DESC LIMIT ")
        .push_bind(limit + 1)
        .push(" OFFSET ")
        .push_bind(page.offset());
    let mut rows: Vec<Video> = query.build_query_as().fetch_all(&state.db).await?;
    let has_more = rows.len() > page.limit();
    rows.truncate(page.limit());
    Ok(VideoPage {
        videos: rows
            .into_iter()
            .map(|video| video.links(state, !matches!(scope, Collection::PublicAlbum(_))))
            .collect(),
        has_more,
    })
}
