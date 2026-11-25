// SPDX-FileCopyrightText: 2026 Cubeyond
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Bounded file processing and idempotent import into Mori's existing upload lifecycle.

use super::{
    Connection,
    model::{AlbumRule, Optimization},
    worker::{WorkerError, call, directory},
};
use crate::{
    app::{AppState, now},
    features::uploads::{self, CreateUpload},
    http::error::{Result, bad},
};
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use sqlx::FromRow;
use std::{path::Path, time::Duration};

/// File metadata from the selected album, without loading its content.
#[derive(Deserialize)]
struct Asset {
    /// Backend-relative file name or path.
    path: String,
    /// Size in bytes, if available from the provider.
    size: i64,
    /// Provider listing timestamp used for quick repeat detection.
    modified: String,
}

/// One bounded page of an album inventory.
#[derive(Deserialize)]
struct AssetPage {
    /// Files in provider order.
    assets: Vec<Asset>,
    /// Whether this inventory has additional pages.
    more: bool,
}

/// Validated file held in the shared disk directory.
#[derive(Deserialize)]
struct Prepared {
    /// Safe file name of the exact bytes to transfer.
    file: String,
    /// Checksum of the source before any encoding.
    source_digest: String,
    /// Checksum of the final file, stable across resumed requests.
    payload_digest: String,
    /// Container of the prepared bytes.
    extension: String,
    /// Size of the final file.
    size: i64,
    /// Width of the prepared media, when known.
    width: i64,
    /// Height of the prepared media, when known.
    height: i64,
    /// Duration in seconds for video.
    duration: f64,
    /// Whether a bounded JPEG poster is available.
    poster: bool,
}

/// Database state of a prepared or previously imported source.
#[derive(FromRow)]
struct Previous {
    /// Mori's media ID.
    video_id: String,
    /// Existing multipart reservation.
    upload_id: String,
    /// Hash of the exact bytes bound to that reservation.
    payload_digest: String,
    /// Current state of the multipart reservation.
    state: String,
    /// Epoch second when the reservation expires.
    expires_at: i64,
}

/// Safe summary of one synchronization pass.
#[derive(Default)]
pub(super) struct Report {
    /// Successfully verified new media.
    pub imported: i64,
    /// Previously imported or unsupported files.
    pub skipped: i64,
    /// Failed files or album listings.
    pub failed: i64,
    /// Resource pressure stopped admission of new work.
    pub deferred: bool,
}

/// Additive pull of selected albums, with one file in the processing pipeline.
pub(super) async fn sync(
    s: &AppState,
    current: &mut Connection,
    rules: &[AlbumRule],
    optimization: Optimization,
) -> std::result::Result<Report, WorkerError> {
    if s.storage.current().is_none() {
        return Err(WorkerError::Unavailable);
    }
    let mut report = Report::default();
    for rule in rules {
        match sync_album(s, current, rule, optimization, &mut report).await {
            Ok(()) => {}
            Err(WorkerError::Resources) => {
                report.deferred = true;
                break;
            }
            Err(error @ (WorkerError::Auth | WorkerError::WebAccess | WorkerError::Approval)) => {
                return Err(error);
            }
            Err(_) => report.failed += 1,
        }
    }
    Ok(report)
}

/// Read one album in bounded worker messages, importing one file at a time.
async fn sync_album(
    s: &AppState,
    current: &mut Connection,
    rule: &AlbumRule,
    optimization: Optimization,
    report: &mut Report,
) -> std::result::Result<(), WorkerError> {
    let mut offset = 0usize;
    loop {
        let page: AssetPage = current
            .request_provider(
                s,
                json!({
                    "op":"files","album":rule.source,"offset":offset,
                }),
                630,
            )
            .await?;
        if page.assets.len() > 256 || (page.assets.is_empty() && page.more) {
            return Err(WorkerError::Unavailable);
        }
        offset += page.assets.len();
        for asset in page.assets {
            match process(s, current, rule, &asset, optimization).await {
                Ok(true) => report.imported += 1,
                Ok(false) | Err(WorkerError::TooLarge) => report.skipped += 1,
                Err(
                    error @ (WorkerError::Resources
                    | WorkerError::Auth
                    | WorkerError::WebAccess
                    | WorkerError::Approval),
                ) => return Err(error),
                Err(_) => report.failed += 1,
            }
        }
        if !page.more {
            return Ok(());
        }
    }
}

/// Distinguish source and staged fingerprints before invoking the expensive worker.
async fn process(
    s: &AppState,
    current: &mut Connection,
    rule: &AlbumRule,
    asset: &Asset,
    optimization: Optimization,
) -> std::result::Result<bool, WorkerError> {
    let Some(extension) = Path::new(&asset.path)
        .extension()
        .and_then(|value| value.to_str())
    else {
        return Ok(false);
    };
    let extension = extension.to_ascii_lowercase();
    let video = s.config.video_extensions.contains(&extension);
    if !video && !s.config.image_extensions.contains(&extension) {
        return Ok(false);
    }
    let max = if video {
        s.config.max_upload_bytes
    } else {
        s.config.max_image_upload_bytes
    };
    if asset.size > max {
        return Ok(false);
    }
    let fingerprint = hash(&json!([
        rule.source,
        asset.path,
        asset.size,
        asset.modified
    ]));
    let seen: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM icloud_seen WHERE account=? AND album_id=? AND fingerprint=?)",
    )
    .bind(&current.account)
    .bind(&rule.album_id)
    .bind(&fingerprint)
    .fetch_one(&s.db)
    .await
    .map_err(|_| WorkerError::Unavailable)?;
    if seen {
        return Ok(false);
    }
    let stage = hash(&json!([
        current.account,
        fingerprint,
        optimization.as_str()
    ]));
    let prepared: Prepared = current.request_provider(s, json!({
        "op":"prepare","stage":stage,"album":rule.source,"file":asset.path,
        "size":asset.size,"max":max,"extension":extension,"video":video,"optimization":optimization.as_str(),
    }), 1900).await?;
    let result = install(s, current, rule, &fingerprint, &stage, prepared).await;
    if result.is_ok() {
        let _: std::result::Result<Value, _> =
            call(s, json!({"op":"remove","stage":stage}), 10).await;
    }
    result.map_err(|_| WorkerError::Unavailable)
}

/// Hash stable structured input without path-separator ambiguity.
fn hash(value: &Value) -> String {
    hex::encode(Sha256::digest(value.to_string()))
}

/// Store an import mapping before transferring bytes, allowing stable random IDs on retries.
async fn install(
    s: &AppState,
    current: &Connection,
    rule: &AlbumRule,
    fingerprint: &str,
    stage: &str,
    prepared: Prepared,
) -> Result<bool> {
    let work = directory(s)?.join("stage").join(stage);
    if !matches!(prepared.file.as_str(), "source" | "optimized") {
        return Err(bad("Invalid prepared media"));
    }
    let access = s.storage.operation().await;
    let previous: Option<Previous> = sqlx::query_as(
        "SELECT i.video_id,i.upload_id,i.payload_digest,u.state,u.expires_at FROM icloud_imports i JOIN uploads u ON u.id=i.upload_id WHERE account=? AND album_id=? AND digest=?",
    ).bind(&current.account).bind(&rule.album_id).bind(&prepared.source_digest)
        .fetch_optional(&s.db).await?;
    let (upload_id, imported) = if let Some(previous) = previous.filter(|previous| {
        matches!(
            previous.state.as_str(),
            "complete" | "verifying" | "uploaded"
        ) || (previous.state == "uploading"
            && previous.expires_at > now()
            && previous.payload_digest == prepared.payload_digest)
    }) {
        if previous.state == "uploading" {
            transfer(s, &previous.upload_id, &previous.video_id, &work, &prepared).await?;
            (previous.upload_id, true)
        } else {
            (previous.upload_id, false)
        }
    } else {
        let pending: Option<(String,)> = sqlx::query_as("SELECT u.id FROM icloud_imports i JOIN uploads u ON u.id=i.upload_id WHERE account=? AND album_id=? AND digest=? AND u.state IN ('uploading','canceling')")
            .bind(&current.account).bind(&rule.album_id).bind(&prepared.source_digest).fetch_optional(&s.db).await?;
        if let Some((id,)) = pending {
            uploads::cancellation::cancel_one(s, &id, false).await?;
        }
        let reservation = uploads::reserve(
            s,
            CreateUpload {
                album_id: Some(rule.album_id.clone()),
                size: prepared.size,
                visibility: "private".into(),
                extension: prepared.extension.clone(),
                width: prepared.width,
                height: prepared.height,
                duration: prepared.duration,
            },
            Some(&rule.album_id),
        )
        .await?;
        sqlx::query("INSERT INTO icloud_imports(account,album_id,digest,video_id,upload_id,payload_digest) VALUES(?,?,?,?,?,?) ON CONFLICT(account,album_id,digest) DO UPDATE SET video_id=excluded.video_id,upload_id=excluded.upload_id,payload_digest=excluded.payload_digest")
            .bind(&current.account).bind(&rule.album_id).bind(&prepared.source_digest)
            .bind(&reservation.video_id).bind(&reservation.id).bind(&prepared.payload_digest)
            .execute(&s.db).await?;
        transfer(s, &reservation.id, &reservation.video_id, &work, &prepared).await?;
        (reservation.id, true)
    };
    drop(access);
    uploads::await_verification(s, &upload_id).await?;
    sqlx::query("INSERT OR IGNORE INTO icloud_seen(account,album_id,fingerprint) VALUES(?,?,?)")
        .bind(&current.account)
        .bind(&rule.album_id)
        .bind(fingerprint)
        .execute(&s.db)
        .await?;
    Ok(imported)
}

/// Upload only the prepared bytes and await durable S3 verification.
async fn transfer(
    s: &AppState,
    id: &str,
    video: &str,
    work: &Path,
    prepared: &Prepared,
) -> Result<()> {
    let file = work.join(&prepared.file);
    uploads::transfer_file(s, id, &file).await?;
    if prepared.poster {
        let poster = tokio::time::timeout(
            Duration::from_secs(2),
            tokio::fs::read(work.join("poster.jpg")),
        )
        .await
        .map_err(anyhow::Error::from)?
        .map_err(anyhow::Error::from)?;
        uploads::store_poster(s, video, poster.into()).await?;
    }
    uploads::finalize(s, id).await?;
    Ok(())
}
