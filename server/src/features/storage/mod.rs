//! Single-administrator storage setup, connection checks, and encrypted credential updates.

/// Safe, operation-specific setup failures without credential-bearing SDK diagnostics.
mod errors;

use errors::connection_error;

use crate::{
    app::AppState,
    features::auth,
    http::error::{Error, Result, bad},
    infra::{storage::Storage, storage_settings::config::StorageSettings},
};
use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Installed provider, distinguishing local protocol fixtures from Amazon S3.
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum StorageMode {
    /// No storage client has been configured.
    Unconfigured,
    /// Standard AWS endpoints are used for the saved configuration.
    AmazonS3,
    /// An operator-supplied endpoint overrides real AWS services.
    Test,
}

/// Whether changing the optional CDN is safe for the current publication lifecycle.
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum DeliveryChange {
    /// All media is private and no edge withdrawal remains unfinished.
    Available,
    /// Public copies or pending withdrawal must be revoked before switching delivery.
    RequiresRevocation,
}

/// Safe storage summary; credential values are never returned.
#[derive(Serialize, ToSchema)]
pub struct StorageInfo {
    /// Whether a validated client is installed.
    pub configured: bool,
    /// Provider identity; a test client must never be presented as a real AWS connection.
    pub mode: StorageMode,
    /// Existing bucket or an empty value before setup.
    pub bucket: String,
    /// Bucket region, defaulting to the setup example.
    pub region: String,
    /// Optional public-delivery distribution.
    #[schema(required = true)]
    pub distribution_id: Option<String>,
    /// Last four characters identifying the saved access key, never its complete value.
    #[schema(required = true)]
    pub access_key_hint: Option<String>,
    /// Whether a temporary token is stored, without revealing it.
    pub has_session_token: bool,
    /// Existing video records prevent moving to an unrelated bucket or region.
    pub location_locked: bool,
    /// Public or withdrawing media prevents changing the delivery mode before revocation.
    pub delivery_change: DeliveryChange,
}
/// Storage fields entered explicitly by the administrator; blank credentials retain saved values.
#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct StorageInput {
    /// Private Amazon S3 bucket.
    pub bucket: String,
    /// AWS region hosting the bucket.
    pub region: String,
    /// Optional distribution; omission selects direct, signed S3 playback.
    pub distribution_id: Option<String>,
    /// Replacement key identifier; omission retains the saved credential.
    #[schema(write_only)]
    pub access_key_id: Option<String>,
    /// Replacement secret; never included in a response.
    #[schema(write_only)]
    pub secret_access_key: Option<String>,
    /// Replacement temporary token; an empty string clears it and omission retains it.
    #[schema(write_only)]
    pub session_token: Option<String>,
}
/// Successful S3 write/read/delete probe, independent of `CloudFront` setup.
#[derive(Serialize, ToSchema)]
pub struct ConnectionCheck {
    /// The supplied S3 configuration passed the required object-permission checks.
    pub verified: bool,
    /// Provider actually checked, so a local fixture cannot be reported as a real AWS connection.
    pub mode: StorageMode,
}
/// Identify the active or proposed provider without treating client construction as a network check.
fn mode(s: &AppState, configured: bool) -> StorageMode {
    match (
        configured,
        s.config.s3_endpoint.is_some() || s.config.cloudfront_endpoint.is_some(),
    ) {
        (false, _) => StorageMode::Unconfigured,
        (true, true) => StorageMode::Test,
        (true, false) => StorageMode::AmazonS3,
    }
}
/// Build a safe view from the installed client and current metadata.
async fn info(s: &AppState) -> Result<StorageInfo> {
    let current = s.storage.current();
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM videos WHERE state!='deleted'")
        .fetch_one(&s.db)
        .await?;
    let delivery_locked: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM videos WHERE state!='deleted' AND (visibility='public' OR state IN ('publishing','published','revoking','deleting'))) OR EXISTS(SELECT 1 FROM jobs WHERE state='pending' AND invalidation_id IS NOT NULL)",
    ).fetch_one(&s.db).await?;
    Ok(StorageInfo {
        configured: current.is_some(),
        mode: mode(s, current.is_some()),
        bucket: current
            .as_ref()
            .map_or_else(String::new, |c| c.bucket.clone()),
        region: current
            .as_ref()
            .map_or_else(|| "us-east-1".into(), |c| c.region.clone()),
        distribution_id: current.as_ref().and_then(|c| c.distribution_id.clone()),
        access_key_hint: current.as_ref().map(|c| c.key_hint.clone()),
        has_session_token: current.as_ref().is_some_and(|c| c.has_session_token),
        location_locked: current.is_some() && count > 0,
        delivery_change: if delivery_locked {
            DeliveryChange::RequiresRevocation
        } else {
            DeliveryChange::Available
        },
    })
}
/// Merge write-only updates with encrypted credentials before validating a candidate.
async fn candidate(s: &AppState, input: StorageInput) -> Result<StorageSettings> {
    let previous = s.storage.stored(&s.db).await?;
    let settings = StorageSettings {
        bucket: input.bucket.trim().into(),
        region: input.region.trim().into(),
        distribution_id: input
            .distribution_id
            .filter(|v| !v.trim().is_empty())
            .map(|v| v.trim().into()),
        access_key_id: input
            .access_key_id
            .filter(|v| !v.trim().is_empty())
            .or_else(|| previous.as_ref().map(|v| v.access_key_id.clone()))
            .unwrap_or_default(),
        secret_access_key: input
            .secret_access_key
            .filter(|v| !v.trim().is_empty())
            .or_else(|| previous.as_ref().map(|v| v.secret_access_key.clone()))
            .unwrap_or_default(),
        session_token: match input.session_token {
            Some(v) if v.is_empty() => None,
            Some(v) => Some(v),
            None => previous.and_then(|v| v.session_token),
        },
    };
    settings
        .validate()
        .map_err(|_| bad("Enter a valid S3 bucket, AWS region and credentials"))?;
    Ok(settings)
}
/// Return only non-secret configured fields and credential-presence indicators.
///
/// # Errors
/// Returns authentication or metadata errors.
#[utoipa::path(get,path="/v1/admin/storage",responses((status=200,body=StorageInfo)),security(("session"=[])))]
pub async fn get(State(s): State<AppState>, h: HeaderMap) -> Result<Json<StorageInfo>> {
    auth::require(&s, &h, false).await?;
    Ok(Json(info(&s).await?))
}
/// Verify and persist settings before atomically activating the new client.
///
/// # Errors
/// Returns authentication, invalid fields, unsafe target change, connection, encryption, or database errors.
#[utoipa::path(put,path="/v1/admin/storage",request_body=StorageInput,responses((status=200,body=StorageInfo),(status=409,description="Existing media locks the storage location")),security(("session"=[])))]
pub async fn save(
    State(s): State<AppState>,
    h: HeaderMap,
    Json(input): Json<StorageInput>,
) -> Result<Json<StorageInfo>> {
    auth::require(&s, &h, true).await?;
    let _change = s.storage.exclusive().await;
    let settings = candidate(&s, input).await?;
    let current = info(&s).await?;
    if current.location_locked
        && (current.bucket != settings.bucket || current.region != settings.region)
    {
        return Err(Error(
            StatusCode::CONFLICT,
            "Storage location is locked while the library contains videos",
        ));
    }
    if matches!(current.delivery_change, DeliveryChange::RequiresRevocation)
        && current.distribution_id != settings.distribution_id
    {
        return Err(Error(
            StatusCode::CONFLICT,
            "Make all videos private and wait for pending operations before changing CloudFront",
        ));
    }
    let client = Storage::new(&settings, &s.config)?;
    client
        .probe()
        .await
        .map_err(|error| connection_error(&error))?;
    s.storage.persist(&s.db, &settings).await?;
    s.storage.install(client);
    Ok(Json(info(&s).await?))
}
/// Check the candidate against S3 without persisting or activating it.
///
/// # Errors
/// Returns authentication, invalid fields, or failed object-permission checks.
#[utoipa::path(post,path="/v1/admin/storage/test",request_body=StorageInput,responses((status=200,body=ConnectionCheck)),security(("session"=[])))]
pub async fn test(
    State(s): State<AppState>,
    h: HeaderMap,
    Json(input): Json<StorageInput>,
) -> Result<Json<ConnectionCheck>> {
    auth::require(&s, &h, true).await?;
    let settings = candidate(&s, input).await?;
    Storage::new(&settings, &s.config)?
        .probe()
        .await
        .map_err(|error| connection_error(&error))?;
    Ok(Json(ConnectionCheck {
        verified: true,
        mode: mode(&s, true),
    }))
}
