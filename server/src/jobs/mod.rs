//! Durable publication, verified revocation, retries, and orphan cleanup.

/// Object migration and verified relocation remain independent of job scheduling.
mod objects;

use crate::{
    app::{AppState, now, random_id},
    features::{uploads::PART_SIZE, videos},
    http::error::{Result, missing},
    infra::storage::Storage,
};
use anyhow::Context;
use axum::{
    Json,
    extract::{Path, State},
    http::HeaderMap,
};
use serde::Serialize;
use sqlx::FromRow;
use utoipa::ToSchema;

/// Persisted storage operation with revision guards, retry state, and CDN progress.
#[derive(FromRow, Serialize, ToSchema)]
pub struct Job {
    /// Stable internal identifier for this record.
    pub id: String,
    /// Identifier of the associated video record.
    pub video_id: String,
    /// Persisted publication, verification, revocation, or deletion operation.
    pub kind: String,
    /// Monotonic version guarding obsolete asynchronous operations.
    pub revision: i64,
    /// Persisted progress or terminal state of the operation.
    pub state: String,
    /// `CloudFront` invalidation being monitored before confirming withdrawal.
    #[schema(required = true)]
    pub invalidation_id: Option<String>,
    /// Number of failed storage attempts for backoff calculation.
    pub attempts: i64,
    /// Earliest timestamp at which this job may be retried.
    pub next_at: i64,
    /// Sanitized explanatory message safe to return to the caller.
    #[schema(required = true)]
    pub error: Option<String>,
    /// Creation timestamp in Unix seconds.
    pub created_at: i64,
}
/// Return authenticated progress for a durable upload or background job.
///
/// # Errors
/// Returns a database error while preserving the persisted operation state.
#[utoipa::path(
    get,
    path = "/v1/admin/jobs/{id}",
    params(("id" = String, Path)),
    responses((status = 200, body = Job)),
    security(("session" = [])),
)]
pub async fn status(
    State(s): State<AppState>,
    h: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<Job>> {
    crate::features::auth::require(&s, &h, false).await?;
    Ok(Json(
        sqlx::query_as("SELECT * FROM jobs WHERE id=?")
            .bind(id)
            .fetch_optional(&s.db)
            .await?
            .ok_or_else(missing)?,
    ))
}
/// Drive persisted storage jobs and periodic cleanup until shutdown.
pub async fn run(s: AppState) {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(3));
    let mut ticks = 0;
    loop {
        tokio::select! {
            _ = interval.tick() => {},
            () = s.job_wakeup.notified() => {},
        }
        ticks = (ticks + 1) % 20;
        // Drain a bounded batch without inserting a polling delay between verification and publication.
        for _ in 0..8 {
            let next = sqlx::query_as::<_, Job>(
                r"SELECT * FROM jobs WHERE state='pending' AND next_at<=?
ORDER BY created_at,id LIMIT 1",
            )
            .bind(now())
            .fetch_optional(&s.db)
            .await;
            match next {
                Ok(Some(job)) => {
                    if step(&s, &job).await.is_err() {
                        tracing::warn!(kind=%job.kind,"background storage task failed; will retry");
                        let _ = schedule_retry(&s, &job).await;
                    }
                }
                Ok(None) => break,
                Err(_) => {
                    tracing::warn!("could not query background tasks");
                    break;
                }
            }
        }
        if ticks == 0 {
            let _ = cleanup(&s).await;
        }
    }
}

/// Persist bounded backoff and a sanitized administrator-visible failure.
///
/// # Errors
/// Returns a database error; the existing pending record is retained for recovery.
async fn schedule_retry(s: &AppState, job: &Job) -> anyhow::Result<()> {
    let delay = (1_i64 << (job.attempts + 1).min(10)).min(900);
    let mut tx = s.db.begin().await?;
    sqlx::query("UPDATE jobs SET attempts=attempts+1,next_at=?,error=? WHERE id=?")
        .bind(now() + delay)
        .bind("Storage operation failed; retry scheduled")
        .bind(&job.id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE videos SET last_error=? WHERE id=? AND revision=?")
        .bind("Storage operation failed; automatic retry scheduled")
        .bind(&job.video_id)
        .bind(job.revision)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

/// Advance one revision-guarded storage operation without losing recovery state.
///
/// # Errors
/// Returns a database error while preserving the persisted operation state.
async fn step(s: &AppState, job: &Job) -> anyhow::Result<()> {
    let _operation = s.storage.operation().await;
    let v = videos::get(s, &job.video_id)
        .await
        .map_err(|e| anyhow::anyhow!("video unavailable: {}", e.1))?;
    if v.revision != job.revision {
        sqlx::query("UPDATE jobs SET state='superseded' WHERE id=?")
            .bind(&job.id)
            .execute(&s.db)
            .await?;
        return Ok(());
    }

    match job.kind.as_str() {
        "verify" => verify(s, job, &v).await,
        "publish" => publish(s, job, &v).await,
        "normalize" => objects::normalize(s, job, &v).await,
        "revoke" | "delete" | "delete_private" => withdraw(s, job, &v).await,
        _ => anyhow::bail!("unsupported persisted job kind"),
    }
}
/// Expire sessions and reconcile abandoned uploads and failed sources.
///
/// # Errors
/// Returns a database error while preserving the persisted operation state.
async fn cleanup(s: &AppState) -> anyhow::Result<()> {
    let _operation = s.storage.operation().await;
    let rows: Vec<(String, String, String)> = sqlx::query_as(
        r"SELECT id, video_id, upload_id FROM uploads WHERE (expires_at<? AND
state='uploading') OR state='canceling'",
    )
    .bind(now())
    .fetch_all(&s.db)
    .await?;
    for (id, video, upload) in rows {
        let v = videos::get(s, &video)
            .await
            .map_err(|_| anyhow::anyhow!("Upload video unavailable"))?;
        if s.storage
            .get()?
            .abort(&v.private_source(), &upload)
            .await
            .is_ok()
        {
            sqlx::query("UPDATE uploads SET state='canceled' WHERE id=?")
                .bind(id)
                .execute(&s.db)
                .await?;
            sqlx::query("UPDATE videos SET state='failed' WHERE id=? AND state='uploading'")
                .bind(video)
                .execute(&s.db)
                .await?;
        }
    }
    let failed: Vec<videos::Video> =
        sqlx::query_as("SELECT * FROM videos WHERE state='failed' LIMIT 16")
            .fetch_all(&s.db)
            .await?;
    for v in failed {
        let _ = s.storage.get()?.delete(v.private_source()).await;
        let _ = s.storage.get()?.delete(v.private_poster()).await;
    }
    sqlx::query("DELETE FROM sessions WHERE expires_at<=?")
        .bind(now())
        .execute(&s.db)
        .await?;
    sqlx::query("INSERT INTO jobs(id,video_id,kind,revision,created_at) SELECT lower(hex(randomblob(16))),v.id,'normalize',v.revision,? FROM videos v WHERE v.source_key IS NULL AND v.state IN ('ready','published') AND NOT EXISTS(SELECT 1 FROM jobs j WHERE j.video_id=v.id AND j.revision=v.revision AND j.kind='normalize' AND j.state='pending') LIMIT 16")
        .bind(now()).execute(&s.db).await?;
    // Recover reservations left by a process crash before an upload session was persisted.
    sqlx::query(
        r"UPDATE videos SET state='failed' WHERE state='uploading' AND created_at<? AND
NOT EXISTS(SELECT 1 FROM uploads WHERE uploads.video_id=videos.id)",
    )
    .bind(now() - 3600)
    .execute(&s.db)
    .await?;
    Ok(())
}

/// Record a terminal validation error before cleaning private source objects.
///
/// # Errors
/// Returns a database error while preserving the persisted operation state.
async fn fail_verification(
    s: &AppState,
    job: &Job,
    video: &videos::Video,
    message: &str,
) -> anyhow::Result<()> {
    let mut tx = s.db.begin().await?;
    sqlx::query("UPDATE videos SET state='failed',last_error=? WHERE id=?")
        .bind(message)
        .bind(&video.id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE uploads SET state='failed' WHERE video_id=?")
        .bind(&video.id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE jobs SET state='failed',error=? WHERE id=?")
        .bind(message)
        .bind(&job.id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    let _ = s.storage.get()?.delete(video.private_source()).await;
    let _ = s.storage.get()?.delete(video.private_poster()).await;
    Ok(())
}

/// Validate multipart integrity and size before activating a private source.
///
/// # Errors
/// Returns storage or transaction errors, leaving the durable job available for retry.
async fn verify(s: &AppState, job: &Job, v: &videos::Video) -> anyhow::Result<()> {
    let (upload_id, upload_state): (String, String) =
        sqlx::query_as("SELECT upload_id,state FROM uploads WHERE video_id=?")
            .bind(&v.id)
            .fetch_one(&s.db)
            .await?;
    if upload_state == "verifying" {
        // A completed object may already exist after a crash between S3 completion and DB update.
        let exists = s
            .storage
            .get()?
            .s3
            .head_object()
            .bucket(&s.storage.get()?.bucket)
            .key(v.private_source())
            .send()
            .await;
        if !exists
            .as_ref()
            .is_ok_and(|h| h.content_length == Some(v.size))
            && let Err(error) = s
                .storage
                .get()?
                .complete(&v.private_source(), &upload_id, v.size, PART_SIZE)
                .await
        {
            if error
                .downcast_ref::<crate::infra::storage::InvalidUpload>()
                .is_some()
            {
                fail_verification(
                    s,
                    job,
                    v,
                    "Upload parts do not match the declared file size; upload again",
                )
                .await?;
                let _ = s
                    .storage
                    .get()?
                    .abort(&v.private_source(), &upload_id)
                    .await;
                return Ok(());
            }
            return Err(error);
        }
        sqlx::query("UPDATE uploads SET state='uploaded' WHERE video_id=?")
            .bind(&v.id)
            .execute(&s.db)
            .await?;
    }
    let stored = s
        .storage
        .get()?
        .s3
        .head_object()
        .bucket(&s.storage.get()?.bucket)
        .key(v.private_source())
        .send()
        .await?;
    if stored.content_length != Some(v.size) {
        fail_verification(s, job, v, "Stored file size does not match the upload").await?;
        return Ok(());
    }
    let public = v.visibility == "public";
    let mut tx = s.db.begin().await?;
    sqlx::query("UPDATE videos SET state=?,last_error=NULL WHERE id=?")
        .bind(if public { "publishing" } else { "ready" })
        .bind(&v.id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE uploads SET state='complete' WHERE video_id=?")
        .bind(&v.id)
        .execute(&mut *tx)
        .await?;
    if public {
        sqlx::query(
            "INSERT INTO jobs(id,video_id,kind,revision,created_at) VALUES(?,?,'publish',?,?)",
        )
        .bind(random_id())
        .bind(&v.id)
        .bind(v.revision)
        .bind(now())
        .execute(&mut *tx)
        .await?;
    }
    sqlx::query("UPDATE jobs SET state='complete',error=NULL WHERE id=?")
        .bind(&job.id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

/// Relocate verified objects and expose publication only after private duplicates are removed.
///
/// # Errors
/// Returns storage or transaction errors, leaving the durable job available for retry.
async fn publish(s: &AppState, job: &Job, v: &videos::Video) -> anyhow::Result<()> {
    s.storage
        .get()?
        .copy_verified(
            v.private_source(),
            Storage::public(&v.token, &v.extension),
            videos::content_type(&v.extension),
            Some(v.size),
        )
        .await?;
    if v.poster != 0 {
        s.storage
            .get()?
            .copy_verified(v.private_poster(), v.public_poster(), "image/jpeg", None)
            .await?;
    }
    let storage = s.storage.get()?;
    storage.delete(v.private_source()).await?;
    storage.delete(v.private_poster()).await?;
    objects::remove_legacy(&storage, v).await?;
    storage
        .delete(Storage::private(&v.token, &v.extension))
        .await?;
    storage.delete(Storage::private_poster(&v.token)).await?;
    let mut tx = s.db.begin().await?;
    sqlx::query("UPDATE videos SET state='published',source_key=?,last_error=NULL WHERE id=? AND revision=?")
        .bind(Storage::private(&v.token, &v.extension))
        .bind(&v.id)
        .bind(v.revision)
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE jobs SET state='complete',error=NULL WHERE id=?")
        .bind(&job.id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

/// Remove public copies and confirm withdrawal before changing the terminal state.
///
/// # Errors
/// Returns storage or transaction errors, leaving the durable job available for retry.
async fn withdraw(s: &AppState, job: &Job, v: &videos::Video) -> anyhow::Result<()> {
    if job.kind == "revoke" {
        objects::restore_private(s, v).await?;
    }
    s.storage
        .get()?
        .delete(Storage::public(&v.token, &v.extension))
        .await?;
    s.storage.get()?.delete(v.public_poster()).await?;
    if job.kind != "delete_private" && !invalidation_finished(s, job, v).await? {
        return Ok(());
    }

    if matches!(job.kind.as_str(), "delete" | "delete_private") {
        let storage = s.storage.get()?;
        storage.delete(v.private_source()).await?;
        storage.delete(v.private_poster()).await?;
        storage
            .delete(Storage::private(&v.token, &v.extension))
            .await?;
        storage.delete(Storage::private_poster(&v.token)).await?;
        objects::remove_legacy(&storage, v).await?;
    }
    let mut tx = s.db.begin().await?;
    sqlx::query(
        "UPDATE videos SET state=?,visibility='private',source_key=?,last_error=NULL WHERE id=? AND revision=?",
    )
    .bind(
        if matches!(job.kind.as_str(), "delete" | "delete_private") {
            "deleted"
        } else {
            "ready"
        },
    )
    .bind(Storage::private(&v.token, &v.extension))
    .bind(&v.id)
    .bind(v.revision)
    .execute(&mut *tx)
    .await?;
    sqlx::query("UPDATE jobs SET state='complete',error=NULL WHERE id=?")
        .bind(&job.id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

/// Create or poll the idempotent `CloudFront` invalidation without declaring early success.
///
/// # Errors
/// Returns storage or transaction errors, leaving the durable job available for retry.
async fn invalidation_finished(s: &AppState, job: &Job, v: &videos::Video) -> anyhow::Result<bool> {
    let storage = s.storage.get()?;
    let Some(distribution) = storage.distribution_id.as_ref() else {
        anyhow::ensure!(
            job.invalidation_id.is_none(),
            "CloudFront must remain configured until public revocation completes"
        );
        return Ok(true);
    };
    let cf = storage
        .cf
        .as_ref()
        .context("CloudFront client is unavailable")?;
    if let Some(invalidation) = &job.invalidation_id {
        let result = cf
            .get_invalidation()
            .distribution_id(distribution)
            .id(invalidation)
            .send()
            .await?;
        if result
            .invalidation()
            .map(aws_sdk_cloudfront::types::Invalidation::status)
            != Some("Completed")
        {
            sqlx::query("UPDATE jobs SET next_at=? WHERE id=?")
                .bind(now() + 10)
                .bind(&job.id)
                .execute(&s.db)
                .await?;
            return Ok(false);
        }
    } else {
        use aws_sdk_cloudfront::types::{InvalidationBatch, Paths};
        let paths = Paths::builder()
            .quantity(2)
            .items(format!("/files/{}.{}", v.token, v.extension))
            .items(format!(
                "/{}",
                v.public_poster().trim_start_matches("public/")
            ))
            .build()?;
        let batch = InvalidationBatch::builder()
            .caller_reference(format!("mori-{}-{}", v.id, v.revision))
            .paths(paths)
            .build()?;
        let result = cf
            .create_invalidation()
            .distribution_id(distribution)
            .invalidation_batch(batch)
            .send()
            .await?;
        let id = result.invalidation().context("missing invalidation")?.id();
        sqlx::query("UPDATE jobs SET invalidation_id=?,next_at=? WHERE id=?")
            .bind(id)
            .bind(now() + 10)
            .bind(&job.id)
            .execute(&s.db)
            .await?;
        return Ok(false);
    }

    Ok(true)
}
