//! Transactional deletion scheduling shared by video and album administration.

use crate::{
    app::{now, random_id},
    http::error::{Error, Result, missing},
};
use axum::http::StatusCode;
use sqlx::{Sqlite, Transaction};

/// Queue one deletion within the caller's atomic mutation.
///
/// # Errors
/// Returns missing video, active-operation conflict, or database errors.
pub async fn enqueue(tx: &mut Transaction<'_, Sqlite>, id: &str) -> Result<String> {
    let (delivery, revision): (String, i64) =
        sqlx::query_as("SELECT state,revision FROM videos WHERE id=? AND state!='deleted'")
            .bind(id)
            .fetch_optional(&mut **tx)
            .await?
            .ok_or_else(missing)?;
    if !matches!(delivery.as_str(), "ready" | "published" | "failed") {
        return Err(Error(
            StatusCode::CONFLICT,
            "Wait for current video operations or cancel unfinished uploads",
        ));
    }
    let kind = if delivery == "published" {
        "delete"
    } else {
        "delete_private"
    };
    let job = random_id();
    sqlx::query(
        "UPDATE videos SET state='deleting',visibility='private',revision=revision+1 WHERE id=?",
    )
    .bind(id)
    .execute(&mut **tx)
    .await?;
    sqlx::query("INSERT INTO jobs(id,video_id,kind,revision,created_at) VALUES(?,?,?,?,?)")
        .bind(&job)
        .bind(id)
        .bind(kind)
        .bind(revision + 1)
        .bind(now())
        .execute(&mut **tx)
        .await?;
    Ok(job)
}
