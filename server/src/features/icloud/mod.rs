// SPDX-FileCopyrightText: 2026 Cubeyond
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Administrator-owned iCloud connections, album mappings, and durable schedules.

mod import;
mod model;
mod worker;

use self::{
    model::{
        AlbumRule, AlbumSelection, ChallengeKind, ConnectInput, ConnectResult, Frequency,
        Optimization, PhoneChoice, PreferenceInput, Schedule, ScheduleInput, SourceAlbum,
        SyncActivity, SyncStatus, UploadPreference,
    },
    worker::{WorkerError, call, directory},
};
use crate::{
    app::{AppState, now, random_id, valid_token},
    features::{auth::Admin, videos},
    http::error::{Error, Result, bad},
};
use axum::{Json, extract::State, http::StatusCode};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use sqlx::FromRow;
use std::{
    collections::HashSet,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use tokio::sync::{Mutex, MutexGuard, Notify};
use zeroize::Zeroizing;

/// Wake scheduled work and prevent changes of account or mappings mid-run.
#[derive(Clone, Default)]
pub struct SyncControl {
    /// A single owner prepares or changes one session at a time.
    gate: Arc<Mutex<()>>,
    /// Scheduler notification for manual requests.
    wake: Arc<Notify>,
    /// Public indication that the synchronization worker is active.
    running: Arc<AtomicBool>,
}

/// An encrypted provider session associated with the signed-in Apple account.
#[derive(FromRow)]
struct Connection {
    /// Unique version of the current connection.
    generation: String,
    /// Normalized account address.
    account: String,
    /// Encrypted trust token and cookies.
    credentials: Vec<u8>,
    /// A permanent upstream session failure requires user action.
    needs_auth: bool,
}

/// Cookie material retained after successful verification.
#[derive(Deserialize, Serialize)]
struct ProviderSession {
    /// Saved session cookies consumed by rclone.
    cookies: String,
    /// Provider trust token.
    trust_token: String,
}

/// A worker result may also contain a renewed Apple session.
#[derive(Deserialize)]
struct ProviderResult<T> {
    /// Result of the requested provider operation.
    value: T,
    /// Rotation since the last successful operation, if any.
    session: Option<ProviderSession>,
}

impl Connection {
    /// Pass only the current decrypted session to the private worker.
    fn request(&self, s: &AppState) -> Result<Value> {
        let bytes = s.storage.vault().open_bytes(&self.credentials)?;
        let session: ProviderSession =
            serde_json::from_slice(&bytes).map_err(anyhow::Error::from)?;
        Ok(json!({"generation": self.generation, "account": self.account, "session": session}))
    }

    /// Keep renewed cookies durable across worker restarts.
    async fn request_provider<T: DeserializeOwned>(
        &mut self,
        s: &AppState,
        mut request: Value,
        seconds: u64,
    ) -> std::result::Result<T, WorkerError> {
        request["connection"] = self.request(s).map_err(|_| WorkerError::Unavailable)?;
        let result: ProviderResult<T> = call(s, request, seconds).await?;
        if let Some(session) = result.session {
            let bytes =
                Zeroizing::new(serde_json::to_vec(&session).map_err(|_| WorkerError::Unavailable)?);
            self.credentials = s
                .storage
                .vault()
                .seal(&bytes)
                .map_err(|_| WorkerError::Unavailable)?;
            sqlx::query("UPDATE icloud_connection SET credentials=? WHERE generation=?")
                .bind(&self.credentials)
                .bind(&self.generation)
                .execute(&s.db)
                .await
                .map_err(|_| WorkerError::Unavailable)?;
        }
        Ok(result.value)
    }
}

/// Get the singleton connection, including disconnected installations.
async fn connection(s: &AppState) -> Result<Option<Connection>> {
    Ok(sqlx::query_as(
        "SELECT generation,account,credentials,needs_auth FROM icloud_connection WHERE id=1",
    )
    .fetch_optional(&s.db)
    .await?)
}

/// Read only usable session material for external requests.
async fn connected(s: &AppState) -> Result<Connection> {
    let current = connection(s)
        .await?
        .ok_or(bad("Connect an Apple Account first"))?;
    if current.needs_auth {
        return Err(WorkerError::Auth.into());
    }
    Ok(current)
}

/// Stored schedule with defaults established in SQLite.
async fn schedule(s: &AppState) -> Result<Schedule> {
    Ok(sqlx::query_as("SELECT * FROM icloud_schedule WHERE id=1")
        .fetch_one(&s.db)
        .await?)
}

/// Return only mappings whose destination album still exists.
async fn rules(s: &AppState) -> Result<Vec<AlbumRule>> {
    Ok(
        sqlx::query_as("SELECT source,album_id FROM icloud_albums ORDER BY source")
            .fetch_all(&s.db)
            .await?,
    )
}

/// Read a shared preference without replacing a preexisting browser default.
async fn preference(s: &AppState) -> Result<Option<Optimization>> {
    let value: Option<String> =
        sqlx::query_scalar("SELECT optimization FROM upload_preferences WHERE id=1")
            .fetch_optional(&s.db)
            .await?;
    Ok(value.as_deref().map(Optimization::read))
}

/// Refuse incompatible connection changes while the active run owns the worker.
fn control(s: &AppState) -> Result<MutexGuard<'_, ()>> {
    directory(s)?;
    s.icloud.gate.try_lock().map_err(|_| {
        Error(
            StatusCode::CONFLICT,
            "iCloud is busy. Try again after the current operation",
        )
    })
}

/// Read connection, schedule, selected albums and last-run feedback.
/// # Errors
/// Returns administrator authentication or database failures.
#[utoipa::path(get, path = "/v1/admin/icloud", responses((status = 200, body = SyncStatus)), security(("session" = [])))]
pub async fn status(State(s): State<AppState>, _admin: Admin) -> Result<Json<SyncStatus>> {
    let account = connection(&s).await?;
    let schedule = schedule(&s).await?;
    let needs_auth = account.as_ref().is_some_and(|value| value.needs_auth);
    Ok(Json(SyncStatus {
        enabled: s.config.icloud_dir.is_some(),
        needs_auth,
        account: account.map(|value| value.account),
        activity: if s.icloud.running.load(Ordering::Relaxed) {
            SyncActivity::Running
        } else if schedule.queued_at.is_some() && !needs_auth {
            SyncActivity::Queued
        } else {
            SyncActivity::Idle
        },
        schedule: schedule.settings(),
        upload_optimization: preference(&s).await?.unwrap_or_default(),
        albums: rules(&s).await?,
        next_sync: schedule.next_sync,
        last_sync: schedule.last_sync,
        last_error: schedule.last_error,
        imported: schedule.imported,
        skipped: schedule.skipped,
        failed: schedule.failed,
    }))
}

/// Private authentication result before the session material is removed.
#[derive(Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
enum ProviderAuth {
    /// The worker needs another verification step.
    Challenge {
        /// Server-issued challenge identifier.
        id: String,
        /// Next verification step.
        kind: ChallengeKind,
        /// Masked alternatives for SMS.
        phones: Vec<PhoneChoice>,
        /// Whether the previous answer was rejected.
        retry: bool,
    },
    /// Authentication is complete.
    Connected {
        /// Verified account identifier.
        account: String,
        /// Session secrets to save through the vault.
        session: ProviderSession,
    },
    /// The worker dismissed the challenge.
    Canceled,
}

/// Bind an Apple account through a short-lived interactive challenge.
/// # Errors
/// Returns administrator, validation, provider, encryption or database failures.
#[utoipa::path(post, path = "/v1/admin/icloud/connect", request_body = ConnectInput, responses((status = 200, body = ConnectResult)), security(("session" = [])))]
pub async fn connect(
    State(s): State<AppState>,
    _admin: Admin,
    Json(input): Json<ConnectInput>,
) -> Result<Json<ConnectResult>> {
    let _control = control(&s)?;
    let (request, chosen) = match input {
        ConnectInput::Start {
            account,
            password,
            upload_optimization,
        } => {
            let account = account.trim().to_lowercase();
            if account.len() > 254
                || !account.contains('@')
                || account.chars().any(char::is_whitespace)
                || password.is_empty()
                || password.len() > 1024
                || account.chars().any(char::is_control)
            {
                return Err(bad("Enter a valid Apple Account and password"));
            }
            (
                json!({"op":"start","challenge":random_id(),"account":account,"password":password}),
                Some(upload_optimization),
            )
        }
        ConnectInput::Answer { challenge, answer }
            if valid_token(&challenge) && (1..=128).contains(&answer.len()) =>
        {
            (
                json!({"op":"answer","challenge":challenge,"answer":answer.trim()}),
                None,
            )
        }
        ConnectInput::Cancel { challenge } if valid_token(&challenge) => {
            (json!({"op":"cancel","challenge":challenge}), None)
        }
        _ => return Err(bad("Enter a valid verification response")),
    };
    let result: ProviderAuth = call(&s, request, 45).await?;
    if let Some(chosen) = chosen {
        sqlx::query("INSERT OR IGNORE INTO upload_preferences(id,optimization) VALUES(1,?)")
            .bind(chosen.as_str())
            .execute(&s.db)
            .await?;
    }
    Ok(Json(match result {
        ProviderAuth::Challenge {
            id,
            kind,
            phones,
            retry,
        } => ConnectResult::Challenge {
            id,
            kind,
            phones,
            retry,
        },
        ProviderAuth::Connected { account, session } => {
            let bytes = Zeroizing::new(serde_json::to_vec(&session).map_err(anyhow::Error::from)?);
            let secret = s.storage.vault().seal(&bytes)?;
            let mut tx = s.db.begin_with("BEGIN IMMEDIATE").await?;
            let previous: Option<String> =
                sqlx::query_scalar("SELECT account FROM icloud_connection WHERE id=1")
                    .fetch_optional(&mut *tx)
                    .await?;
            if previous.as_deref() != Some(&account) {
                reset_rules(&mut tx).await?;
            }
            sqlx::query("INSERT INTO icloud_connection(id,generation,account,credentials,needs_auth) VALUES(1,?,?,?,0) ON CONFLICT(id) DO UPDATE SET generation=excluded.generation,account=excluded.account,credentials=excluded.credentials,needs_auth=0")
                .bind(random_id()).bind(account).bind(secret).execute(&mut *tx).await?;
            sqlx::query("UPDATE icloud_schedule SET last_error=NULL WHERE id=1")
                .execute(&mut *tx)
                .await?;
            tx.commit().await?;
            s.icloud.wake.notify_one();
            ConnectResult::Connected
        }
        ProviderAuth::Canceled => ConnectResult::Canceled,
    }))
}

/// Dismiss account-specific rules while retaining already imported media and its deduplication records.
async fn reset_rules(db: &mut sqlx::SqliteConnection) -> Result<()> {
    sqlx::query("DELETE FROM icloud_albums")
        .execute(&mut *db)
        .await?;
    sqlx::query("UPDATE icloud_schedule SET automatic=0,next_sync=NULL,queued_at=NULL,retries=0,last_sync=NULL,last_error=NULL,imported=0,skipped=0,failed=0 WHERE id=1")
        .execute(&mut *db).await?;
    Ok(())
}

/// Forget the saved session and stop pending jobs without deleting gallery items.
/// # Errors
/// Returns administrator, worker availability or database failures.
#[utoipa::path(delete, path = "/v1/admin/icloud/connect", responses((status = 204, description = "Disconnected")), security(("session" = [])))]
pub async fn disconnect(State(s): State<AppState>, _admin: Admin) -> Result<StatusCode> {
    let _control = control(&s)?;
    let mut tx = s.db.begin_with("BEGIN IMMEDIATE").await?;
    sqlx::query("DELETE FROM icloud_connection WHERE id=1")
        .execute(&mut *tx)
        .await?;
    reset_rules(&mut tx).await?;
    tx.commit().await?;
    let _: std::result::Result<Value, _> = call(&s, json!({"op":"forget"}), 5).await;
    Ok(StatusCode::NO_CONTENT)
}

/// Read the currently connected iCloud account's album paths.
/// # Errors
/// Returns administrator, connection, or provider failures.
#[utoipa::path(get, path = "/v1/admin/icloud/albums", responses((status = 200, body = Vec<SourceAlbum>)), security(("session" = [])))]
pub async fn discover(State(s): State<AppState>, _admin: Admin) -> Result<Json<Vec<SourceAlbum>>> {
    let _control = control(&s)?;
    let mut current = connected(&s).await?;
    let result = current
        .request_provider(&s, json!({"op":"albums"}), 130)
        .await;
    if let Some(message) = result.as_ref().err().and_then(WorkerError::reconnect_hint) {
        mark_auth(&s, message).await?;
    }
    Ok(Json(result?))
}

/// Validate that a source is a relative iCloud Photos path.
fn album_path(path: &str) -> bool {
    path.len() <= 4096
        && !path.chars().any(char::is_control)
        && path.split_once('/').is_some_and(|(library, album)| {
            (library == "PrimarySync" || library.starts_with("SharedSync-"))
                && album
                    .split('/')
                    .all(|part| !matches!(part, "" | "." | ".."))
        })
}

/// Validate and atomically replace album mappings. An initial import is queued on change.
/// # Errors
/// Returns invalid input, administrator, connection, or database failures.
#[utoipa::path(put, path = "/v1/admin/icloud/albums", request_body = AlbumSelection, responses((status = 204, description = "Mappings saved")), security(("session" = [])))]
pub async fn save_albums(
    State(s): State<AppState>,
    _admin: Admin,
    Json(selection): Json<AlbumSelection>,
) -> Result<StatusCode> {
    let _control = control(&s)?;
    connected(&s).await?;
    let mut paths = HashSet::new();
    if selection.albums.len() > 250
        || selection.albums.iter().any(|rule| {
            !album_path(&rule.source) || !valid_token(&rule.album_id) || !paths.insert(&rule.source)
        })
    {
        return Err(bad("Choose valid iCloud sources and existing Mori albums"));
    }
    if !selection.albums.is_empty() {
        s.storage.get()?;
    }
    let mut tx = s.db.begin_with("BEGIN IMMEDIATE").await?;
    sqlx::query("DELETE FROM icloud_albums")
        .execute(&mut *tx)
        .await?;
    for rule in &selection.albums {
        videos::album_prefix(&mut tx, Some(&rule.album_id)).await?;
        sqlx::query("INSERT INTO icloud_albums(source,album_id) VALUES(?,?)")
            .bind(&rule.source)
            .bind(&rule.album_id)
            .execute(&mut *tx)
            .await?;
    }
    sqlx::query("UPDATE icloud_schedule SET queued_at=?,retries=0,last_error=NULL WHERE id=1")
        .bind((!selection.albums.is_empty()).then(now))
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    s.icloud.wake.notify_one();
    Ok(StatusCode::NO_CONTENT)
}

/// Calculate the next timezone-aware due time through the system timezone database.
async fn next(s: &AppState, input: &ScheduleInput) -> Result<Option<i64>> {
    let due = call(
        s,
        json!({
            "op":"next","frequency":input.frequency.as_str(),"time":input.local_time,
            "timezone":input.timezone,"weekday":input.weekday,"after":now()
        }),
        5,
    )
    .await?;
    Ok(input.automatic.then_some(due))
}

/// Save an opt-in schedule and an optional optimization override.
/// # Errors
/// Returns administrator, connection, scheduling or database failures.
#[utoipa::path(put, path = "/v1/admin/icloud/schedule", request_body = ScheduleInput, responses((status = 204, description = "Schedule saved")), security(("session" = [])))]
pub async fn save_schedule(
    State(s): State<AppState>,
    _admin: Admin,
    Json(input): Json<ScheduleInput>,
) -> Result<StatusCode> {
    directory(&s)?;
    let current = connection(&s)
        .await?
        .ok_or(bad("Connect an Apple Account first"))?;
    if input.local_time.len() != 5 || input.timezone.len() > 128 {
        return Err(bad("Choose a valid time and time zone"));
    }
    if matches!(input.frequency, Frequency::Weekly) && input.weekday.is_none() {
        return Err(bad("Choose a day of the week"));
    }
    let due = next(&s, &input).await?;
    let changed = sqlx::query("UPDATE icloud_schedule SET automatic=?,frequency=?,local_time=?,timezone=?,weekday=coalesce(?,weekday),optimization=?,next_sync=? WHERE id=1 AND EXISTS(SELECT 1 FROM icloud_connection WHERE generation=?)")
        .bind(input.automatic).bind(input.frequency.as_str()).bind(input.local_time)
        .bind(input.timezone).bind(input.weekday).bind(input.optimization.map(Optimization::as_str))
        .bind(due).bind(current.generation).execute(&s.db).await?;
    if changed.rows_affected() == 0 {
        return Err(Error(StatusCode::CONFLICT, "The Apple Account changed"));
    }
    s.icloud.wake.notify_one();
    Ok(StatusCode::NO_CONTENT)
}

/// Queue a manual run without creating overlapping requests for the same account.
/// # Errors
/// Returns administrator, connection, storage or database failures.
#[utoipa::path(post, path = "/v1/admin/icloud/sync", responses((status = 202, description = "Sync queued")), security(("session" = [])))]
pub async fn sync_now(State(s): State<AppState>, _admin: Admin) -> Result<StatusCode> {
    directory(&s)?;
    connected(&s).await?;
    s.storage.get()?;
    if rules(&s).await?.is_empty() {
        return Err(bad("Choose at least one album to sync"));
    }
    sqlx::query("UPDATE icloud_schedule SET queued_at=min(COALESCE(queued_at,?),?),retries=0 WHERE id=1 AND EXISTS(SELECT 1 FROM icloud_connection WHERE id=1)")
        .bind(now()).bind(now()).execute(&s.db).await?;
    s.icloud.wake.notify_one();
    Ok(StatusCode::ACCEPTED)
}

/// Read the account's cross-device upload preset.
/// # Errors
/// Returns administrator or database failures.
#[utoipa::path(get, path = "/v1/admin/upload-preference", responses((status = 200, body = UploadPreference)), security(("session" = [])))]
pub async fn upload_preference(
    State(s): State<AppState>,
    _admin: Admin,
) -> Result<Json<UploadPreference>> {
    Ok(Json(UploadPreference {
        optimization: preference(&s).await?,
    }))
}

/// Change the account's preset for new manual uploads and inherited syncs.
/// # Errors
/// Returns administrator authentication or database failures.
#[utoipa::path(put, path = "/v1/admin/upload-preference", request_body = PreferenceInput, responses((status = 204, description = "Preference saved")), security(("session" = [])))]
pub async fn save_upload_preference(
    State(s): State<AppState>,
    _admin: Admin,
    Json(input): Json<PreferenceInput>,
) -> Result<StatusCode> {
    sqlx::query("INSERT INTO upload_preferences(id,optimization) VALUES(1,?) ON CONFLICT(id) DO UPDATE SET optimization=excluded.optimization")
        .bind(input.optimization.as_str()).execute(&s.db).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Disable unattended retries of expired sessions while preserving the configured cadence.
async fn mark_auth(s: &AppState, message: &str) -> Result<()> {
    let mut tx = s.db.begin_with("BEGIN IMMEDIATE").await?;
    sqlx::query("UPDATE icloud_connection SET needs_auth=1 WHERE id=1")
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE icloud_schedule SET last_error=?,retries=0 WHERE id=1")
        .bind(message)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

/// Check for work or resume a persisted run after a restart.
pub async fn run(s: AppState) {
    loop {
        if due_run(&s).await.is_err() {
            tracing::warn!("Could not finish the iCloud sync check");
        }
        tokio::select! {
            () = s.icloud.wake.notified() => {}
            () = tokio::time::sleep(Duration::from_secs(10)) => {}
        }
    }
}

/// Claim a due job, then persist its completion or bounded retry time.
async fn due_run(s: &AppState) -> Result<()> {
    if !is_due(&schedule(s).await?) {
        return Ok(());
    }
    let Ok(_control) = s.icloud.gate.try_lock() else {
        return Ok(());
    };
    let Some(mut current) = connection(s).await?.filter(|item| !item.needs_auth) else {
        return Ok(());
    };
    let settings = schedule(s).await?;
    if !is_due(&settings) {
        return Ok(());
    }
    let selected = rules(s).await?;
    let started = now();
    let rule = settings.settings();
    let calendar = matches!(rule.frequency, Frequency::Daily | Frequency::Weekly);
    let next_due = if (calendar || selected.is_empty())
        && settings.next_sync.is_some_and(|due| due <= started)
    {
        next(s, &rule).await?
    } else {
        None
    };
    // Persist the next calendar slot alongside the recovery marker for this run.
    sqlx::query("UPDATE icloud_schedule SET queued_at=?,next_sync=CASE WHEN next_sync<=? THEN coalesce(?,next_sync) ELSE next_sync END WHERE id=1")
        .bind((!selected.is_empty()).then_some(started))
        .bind(started)
        .bind(next_due)
        .execute(&s.db)
        .await?;
    if selected.is_empty() {
        return Ok(());
    }
    let optimization = settings.optimization.as_deref().map(Optimization::read);
    let optimization = optimization.or(preference(s).await?).unwrap_or_default();
    s.icloud.running.store(true, Ordering::Relaxed);
    let result = import::sync(s, &mut current, &selected, optimization).await;
    s.icloud.running.store(false, Ordering::Relaxed);
    if let Some(message) = result.as_ref().err().and_then(WorkerError::reconnect_hint) {
        return mark_auth(s, message).await;
    }
    let completed = now();
    let settings = schedule(s).await?;
    let automatic = if settings.next_sync.is_some_and(|due| due <= completed) {
        next(s, &settings.settings()).await.ok().flatten()
    } else {
        None
    };
    record_run(s, result, automatic, completed, settings.retries).await
}

/// Record the run and bound retries of failures separately from resource deferrals.
async fn record_run(
    s: &AppState,
    result: std::result::Result<import::Report, WorkerError>,
    automatic: Option<i64>,
    completed: i64,
    previous: i64,
) -> Result<()> {
    let retryable = previous < 2;
    let (report, last_error) = match result {
        Ok(report) => {
            let message = if report.deferred {
                Some("Sync is waiting for CPU, memory or disk space")
            } else if report.failed > 0 && retryable {
                Some("Some items could not be imported. A retry is queued")
            } else if report.failed > 0 {
                Some("Some items could not be imported. Run Sync now to retry")
            } else {
                None
            };
            (report, message)
        }
        Err(error) => (
            import::Report {
                failed: 1,
                deferred: matches!(error, WorkerError::Resources),
                ..import::Report::default()
            },
            Some(Error::from(error).1),
        ),
    };
    let retry = (report.deferred || (report.failed > 0 && retryable)).then(|| now() + 300);
    let retries = if report.deferred {
        previous
    } else if retry.is_some() {
        previous + 1
    } else {
        0
    };
    sqlx::query("UPDATE icloud_schedule SET queued_at=?,next_sync=CASE WHEN next_sync<=? THEN coalesce(?,next_sync) ELSE next_sync END,last_sync=coalesce(?,last_sync),last_error=?,retries=?,imported=?,skipped=?,failed=? WHERE id=1")
        .bind(retry).bind(completed).bind(automatic).bind((!report.deferred).then_some(completed)).bind(last_error)
        .bind(retries).bind(report.imported).bind(report.skipped).bind(report.failed).execute(&s.db).await?;
    Ok(())
}

/// Check the persisted manual or periodic due time before claiming the worker.
fn is_due(settings: &Schedule) -> bool {
    let current = now();
    settings.queued_at.is_some_and(|time| time <= current)
        || (settings.automatic && settings.next_sync.is_some_and(|time| time <= current))
}
