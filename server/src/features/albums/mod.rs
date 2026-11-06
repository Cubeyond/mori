// SPDX-FileCopyrightText: 2026 Cubeyond
// SPDX-License-Identifier: Elastic-2.0

//! Explicit album management and visibility-filtered album queries.

/// Bounded administrator collection previews.
mod catalog;
/// Shared album metadata and mutation contracts.
mod model;
pub use model::*;

use crate::{
    app::{AppState, now, random_id, valid_name},
    features::{
        auth::{self, Admin},
        videos::{Collection, collection, deletion},
    },
    http::{
        error::{Result, bad, missing},
        pagination::Page,
    },
};
use axum::{
    Json,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};

/// Normalize and validate a non-reserved album URL component.
///
/// # Errors
/// Returns 400 for invalid length, syntax, or a reserved system route.
pub fn normalize_slug(input: &str) -> Result<String> {
    let slug = input.trim().to_ascii_lowercase();
    let reserved = [
        "login", "logout", "api", "v1", "manage", "admin", "embed", "watch", "files", "assets",
        "fonts", "health", "_app", "favicon", "robots", "openapi", "videos", "albums", "settings",
        "users", "file", "storage", "posters", "gallery",
    ];
    if !(3..=64).contains(&slug.len())
        || reserved.contains(&slug.as_str())
        || slug.starts_with('-')
        || slug.ends_with('-')
        || slug.contains("--")
        || !slug
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    {
        return Err(bad("Invalid or reserved album slug"));
    }
    Ok(slug)
}
/// List active albums for the administrator, including albums with no public videos.
///
/// # Errors
/// Returns authentication or database errors.
#[utoipa::path(
    get,
    path = "/v1/admin/albums",
    responses((status = 200, body = Vec<AlbumSummary>)),
    security(("session" = [])),
)]
pub async fn list(State(s): State<AppState>, _admin: Admin) -> Result<Json<Vec<AlbumSummary>>> {
    Ok(Json(catalog::summaries(&s).await?))
}
/// Return one administrator-visible collection, including private members and empty albums.
///
/// # Errors
/// Returns authentication, missing album, pagination, or database errors.
#[utoipa::path(
    get,
    path = "/v1/admin/albums/{id}",
    params(("id" = String, Path), Page),
    responses((status = 200, body = AlbumPage), (status = 404, description = "Not found")),
    security(("session" = [])),
)]
pub async fn detail(
    State(s): State<AppState>,
    _admin: Admin,
    Path(id): Path<String>,
    Query(page): Query<Page>,
) -> Result<Json<AlbumPage>> {
    let album: Album = sqlx::query_as(
        "SELECT id,name,slug,visibility,description,created_at FROM albums WHERE id=?",
    )
    .bind(id)
    .fetch_optional(&s.db)
    .await?
    .ok_or_else(missing)?;
    let page = collection(&s, Collection::Album(&album.id), &page).await?;
    Ok(Json(AlbumPage {
        total: page.total,
        album,
        videos: page.videos,
        has_more: page.has_more,
    }))
}
/// Create an album while atomically reserving its normalized slug.
///
/// # Errors
/// Returns authentication, CSRF, invalid label/slug, duplicate-slug, or database errors.
#[utoipa::path(
    post,
    path = "/v1/admin/albums",
    request_body = CreateAlbum,
    responses((status = 201, body = Album), (status = 409, description = "Slug already used")),
    security(("session" = [])),
)]
pub async fn create(
    State(s): State<AppState>,
    _admin: Admin,
    Json(input): Json<CreateAlbum>,
) -> Result<(StatusCode, Json<Album>)> {
    let mut tx = s.db.begin_with("BEGIN IMMEDIATE").await?;
    let album = insert(&mut tx, input).await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(album)))
}

/// Create a validated collection inside a caller-owned transaction, including batch moves.
///
/// # Errors
/// Returns invalid names, visibility, reserved/duplicate slugs, or database failures.
pub(crate) async fn insert(db: &mut sqlx::SqliteConnection, input: CreateAlbum) -> Result<Album> {
    if !valid_name(&input.name) {
        return Err(bad("Name must be 1-200 characters"));
    }
    if !matches!(input.visibility.as_str(), "public" | "private") {
        return Err(bad("Visibility must be public or private"));
    }
    let album = Album {
        id: random_id(),
        name: input.name.trim().into(),
        slug: normalize_slug(&input.slug)?,
        visibility: input.visibility,
        description: normalize_description(&input.description)?,
        created_at: now(),
    };
    sqlx::query(
        "INSERT INTO albums(id,name,slug,visibility,description,created_at) VALUES(?,?,?,?,?,?)",
    )
    .bind(&album.id)
    .bind(&album.name)
    .bind(&album.slug)
    .bind(&album.visibility)
    .bind(&album.description)
    .bind(album.created_at)
    .execute(db)
    .await?;
    Ok(album)
}
/// Update album label and page visibility without changing member-video access policies.
///
/// # Errors
/// Returns authentication, CSRF, invalid label/visibility, missing album, or database errors.
#[utoipa::path(
    patch,
    path = "/v1/admin/albums/{id}",
    params(("id" = String, Path)),
    request_body = EditAlbum,
    responses((status = 204, description = "Album updated")),
    security(("session" = [])),
)]
pub async fn edit(
    State(s): State<AppState>,
    _admin: Admin,
    Path(id): Path<String>,
    Json(input): Json<EditAlbum>,
) -> Result<StatusCode> {
    if !valid_name(&input.name) {
        return Err(bad("Invalid name"));
    }
    if input
        .visibility
        .as_deref()
        .is_some_and(|value| !matches!(value, "public" | "private"))
    {
        return Err(bad("Visibility must be public or private"));
    }
    let slug = input.slug.as_deref().map(normalize_slug).transpose()?;
    let description = input
        .description
        .as_deref()
        .map(normalize_description)
        .transpose()?;
    let mut tx = s.db.begin_with("BEGIN IMMEDIATE").await?;
    let old: String = sqlx::query_scalar("SELECT slug FROM albums WHERE id=?")
        .bind(&id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(missing)?;
    sqlx::query("UPDATE albums SET name=?,slug=COALESCE(?,slug),visibility=COALESCE(?,visibility),description=COALESCE(?,description) WHERE id=?")
        .bind(input.name.trim()).bind(&slug).bind(input.visibility).bind(description).bind(&id)
        .execute(&mut *tx).await?;
    if let Some(slug) = slug.filter(|slug| *slug != old) {
        sqlx::query("DELETE FROM media_aliases WHERE prefix=?")
            .bind(old)
            .execute(&mut *tx)
            .await?;
        sqlx::query("UPDATE videos SET media_prefix=? WHERE album_id=?")
            .bind(&slug)
            .bind(&id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("INSERT OR IGNORE INTO media_aliases(video_id,prefix) SELECT id,? FROM videos WHERE album_id=?")
            .bind(slug).bind(&id).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}
/// Delete a confirmed album, retaining its videos unless media deletion was explicitly requested.
///
/// # Errors
/// Returns authentication, confirmation, active media-operation, or database errors.
#[utoipa::path(
    delete,
    path = "/v1/admin/albums/{id}",
    params(("id" = String, Path)),
    request_body = DeleteAlbum,
    responses(
        (status = 204, description = "Album removed; videos retained without album membership"),
        (status = 202, body = AlbumDeletion, description = "Album removed; video deletion jobs queued"),
        (status = 400, description = "Confirmation does not match"),
        (status = 404, description = "Album not found"),
        (status = 409, description = "A member video has an operation in progress"),
    ),
    security(("session" = [])),
)]
pub async fn delete(
    State(s): State<AppState>,
    _admin: Admin,
    Path(id): Path<String>,
    Json(input): Json<DeleteAlbum>,
) -> Result<Response> {
    let mut tx = s.db.begin_with("BEGIN IMMEDIATE").await?;
    let (name, slug): (String, String) = sqlx::query_as("SELECT name,slug FROM albums WHERE id=?")
        .bind(&id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(missing)?;
    if input.confirmation != name {
        return Err(bad("Enter the exact album name to confirm deletion"));
    }
    let mut job_ids = Vec::new();
    if input.delete_media {
        let ids: Vec<String> = sqlx::query_scalar(
            "SELECT id FROM videos WHERE album_id=? AND state!='deleted' ORDER BY id",
        )
        .bind(&id)
        .fetch_all(&mut *tx)
        .await?;
        for video in ids {
            job_ids.push(deletion::enqueue(&mut tx, &video).await?);
        }
    }
    sqlx::query("DELETE FROM media_aliases WHERE prefix=?")
        .bind(slug)
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT OR IGNORE INTO media_aliases(video_id,prefix) SELECT id,'file' FROM videos WHERE album_id=?")
        .bind(&id).execute(&mut *tx).await?;
    sqlx::query("UPDATE videos SET album_id=NULL,media_prefix='file' WHERE album_id=?")
        .bind(&id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM albums WHERE id=?")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    s.job_wakeup.notify_one();
    Ok(if input.delete_media {
        (StatusCode::ACCEPTED, Json(AlbumDeletion { job_ids })).into_response()
    } else {
        StatusCode::NO_CONTENT.into_response()
    })
}
/// Return album members authorized by the current administrator session or public visibility.
///
/// # Errors
/// Returns 404 for inaccessible or missing albums, or an authentication/database error.
#[utoipa::path(
    get,
    path = "/v1/albums/{slug}",
    params(("slug" = String, Path), Page),
    responses((status = 200, body = AlbumPage), (status = 404, description = "Not found")),
)]
pub async fn public(
    State(s): State<AppState>,
    h: HeaderMap,
    Path(slug): Path<String>,
    Query(page): Query<Page>,
) -> Result<Json<AlbumPage>> {
    let owner = auth::session(State(s.clone()), h).await?.0.authenticated;
    let album: Album = sqlx::query_as(
        r"SELECT id,name,slug,visibility,description,created_at FROM albums WHERE slug=? AND (visibility='public' OR ?)",
    )
    .bind(slug)
    .bind(owner)
    .fetch_optional(&s.db)
    .await?
    .ok_or_else(missing)?;
    let scope = if owner {
        Collection::Album(&album.id)
    } else {
        Collection::PublicAlbum(&album.id)
    };
    let page = collection(&s, scope, &page).await?;
    Ok(Json(AlbumPage {
        total: page.total,
        album,
        videos: page.videos,
        has_more: page.has_more,
    }))
}

/// Trim optional plain text while rejecting oversized descriptions and hidden control characters.
fn normalize_description(input: &str) -> Result<String> {
    let description = input.trim();
    if description.chars().count() > 500
        || description
            .chars()
            .any(|value| value.is_control() && !matches!(value, '\n' | '\t'))
    {
        return Err(bad(
            "Description must be at most 500 characters of plain text",
        ));
    }
    Ok(description.into())
}
