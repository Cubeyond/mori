//! Video metadata, stable sharing links, and persisted visibility transitions.

/// Shared video contracts and naming conventions.
mod model;
/// Scoped metadata queries and deterministic collection pagination.
mod repository;

/// Atomic scheduling for confirmed video and album deletion.
pub(crate) mod deletion;
/// Public resource authorization and optional CDN delivery.
mod delivery;
/// Authenticated private HEAD and Range streaming.
pub mod preview;
use crate::{
    app::{AppState, now, random_id, valid_name, valid_token},
    features::auth,
    http::{
        error::{Error, Result, bad, missing},
        pagination::Page,
    },
};
use axum::{
    Json,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
};
pub use delivery::{public_file, shared_resource};
pub use model::{Accepted, EditVideo, Video, VideoPage, content_type};
pub use preview::*;
pub use repository::get;
pub(crate) use repository::{Collection, collection};

/// Resolve the collection URL namespace without using its editable display label.
///
/// # Errors
/// Returns invalid membership or database errors.
pub async fn album_prefix(db: &mut sqlx::SqliteConnection, id: Option<&str>) -> Result<String> {
    if let Some(id) = id {
        sqlx::query_scalar("SELECT slug FROM albums WHERE id=?")
            .bind(id)
            .fetch_optional(db)
            .await?
            .ok_or_else(|| bad("Album does not exist"))
    } else {
        Ok("file".into())
    }
}
/// Return only published public content, hiding private and missing targets alike.
///
/// # Errors
/// Returns 404 for invalid, private, unpublished, or missing tokens, or a database error.
#[utoipa::path(
    get,
    path = "/v1/videos/{token}",
    params(("token" = String, Path)),
    responses((status = 200, body = Video), (status = 404, description = "Not public")),
)]
pub async fn public(State(s): State<AppState>, Path(token): Path<String>) -> Result<Json<Video>> {
    if !valid_token(&token) {
        return Err(missing());
    }
    let v: Video = sqlx::query_as(
        "SELECT * FROM videos WHERE token=? AND visibility='public' AND state='published'",
    )
    .bind(token)
    .fetch_optional(&s.db)
    .await?
    .ok_or_else(missing)?;
    Ok(Json(v.links(&s, false)))
}
/// Return a bounded collection visible to the authenticated administrator.
///
/// # Errors
/// Returns authentication, page-bound conversion, or database errors.
#[utoipa::path(
    get,
    path = "/v1/admin/videos",
    params(Page),
    responses((status = 200, body = VideoPage)),
    security(("session" = [])),
)]
pub async fn list(
    State(s): State<AppState>,
    h: HeaderMap,
    Query(page): Query<Page>,
) -> Result<Json<VideoPage>> {
    auth::require(&s, &h, false).await?;
    Ok(Json(collection(&s, Collection::Library, &page).await?))
}
/// Apply metadata changes and atomically schedule visibility transitions.
///
/// # Errors
/// Returns authentication, CSRF, metadata, membership, delivery-configuration, or revision errors.
#[utoipa::path(
    patch,
    path = "/v1/admin/videos/{id}",
    params(("id" = String, Path)),
    request_body = EditVideo,
    responses(
        (status = 202, body = Accepted),
        (status = 409, description = "Operation in progress"),
    ),
    security(("session" = [])),
)]
pub async fn edit(
    State(s): State<AppState>,
    h: HeaderMap,
    Path(id): Path<String>,
    Json(input): Json<EditVideo>,
) -> Result<(StatusCode, Json<Accepted>)> {
    auth::require(&s, &h, true).await?;
    if !valid_name(&input.name) || !matches!(input.visibility.as_str(), "public" | "private") {
        return Err(bad("Invalid name or visibility"));
    }
    let v = get(&s, &id).await?;
    if !matches!(v.state.as_str(), "ready" | "published") {
        return Err(Error(
            StatusCode::CONFLICT,
            "Wait for the current operation to finish",
        ));
    }
    let (state, kind) = match (v.state.as_str(), input.visibility.as_str()) {
        ("ready", "public") => ("publishing", Some("publish")),
        ("published", "private") => ("revoking", Some("revoke")),
        _ => (v.state.as_str(), None),
    };
    let mut tx = s.db.begin_with("BEGIN IMMEDIATE").await?;
    let prefix = album_prefix(&mut tx, input.album_id.as_deref()).await?;
    let result = sqlx::query(
        r"UPDATE videos SET name=?, album_id=?, media_prefix=?, visibility=?, state=?,
revision=revision+1, last_error=NULL WHERE id=? AND revision=? AND state=?",
    )
    .bind(input.name.trim())
    .bind(input.album_id)
    .bind(&prefix)
    .bind(&input.visibility)
    .bind(state)
    .bind(&id)
    .bind(v.revision)
    .bind(&v.state)
    .execute(&mut *tx)
    .await?;
    if result.rows_affected() != 1 {
        return Err(Error(
            StatusCode::CONFLICT,
            "Video changed; refresh and retry",
        ));
    }
    sqlx::query("INSERT INTO media_aliases(video_id,prefix) VALUES(?,?) ON CONFLICT DO NOTHING")
        .bind(&id)
        .bind(prefix)
        .execute(&mut *tx)
        .await?;
    let job_id = if let Some(kind) = kind {
        let job = random_id();
        sqlx::query("INSERT INTO jobs(id,video_id,kind,revision,created_at) VALUES(?,?,?,?,?)")
            .bind(&job)
            .bind(id)
            .bind(kind)
            .bind(v.revision + 1)
            .bind(now())
            .execute(&mut *tx)
            .await?;
        Some(job)
    } else {
        None
    };
    tx.commit().await?;
    Ok((StatusCode::ACCEPTED, Json(Accepted { job_id })))
}
/// Schedule revision-guarded deletion and confirmed withdrawal of any public copies.
///
/// # Errors
/// Returns authentication, CSRF, state/revision conflict, delivery-configuration, or database
/// errors.
#[utoipa::path(
    delete,
    path = "/v1/admin/videos/{id}",
    params(("id" = String, Path)),
    responses((status = 202, body = Accepted)),
    security(("session" = [])),
)]
pub async fn delete(
    State(s): State<AppState>,
    h: HeaderMap,
    Path(id): Path<String>,
) -> Result<(StatusCode, Json<Accepted>)> {
    auth::require(&s, &h, true).await?;
    let mut tx = s.db.begin_with("BEGIN IMMEDIATE").await?;
    let job = deletion::enqueue(&mut tx, &id).await?;
    tx.commit().await?;
    Ok((StatusCode::ACCEPTED, Json(Accepted { job_id: Some(job) })))
}
