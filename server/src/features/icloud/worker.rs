// SPDX-FileCopyrightText: 2026 Cubeyond
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Length-bounded private IPC with the Linux import worker.

use crate::{
    app::AppState,
    http::error::{Error, Result},
};
use axum::http::StatusCode;
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::Value;
use std::{path::Path, time::Duration};
#[cfg(unix)]
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// Maximum worker message size, including lists from large selected albums.
const MESSAGE_LIMIT: usize = 32 * 1024 * 1024;

/// Guidance for sessions that need another Apple verification.
const RECONNECT: &str = "Reconnect your Apple Account";
/// Guidance when Apple has not issued a token for iCloud Photos on the web.
const WEB_ACCESS: &str =
    "Open Photos on iCloud.com and check web access or pending Apple terms, then reconnect in Mori";

/// Explicit worker error categories translated without forwarding provider diagnostics.
#[derive(Debug)]
pub(super) enum WorkerError {
    /// The saved session must be renewed.
    Auth,
    /// Apple did not grant a usable web session after account verification.
    WebAccess,
    /// iCloud requires approval for access to Photos on the web.
    Approval,
    /// CPU, memory or disk pressure prevents the next task.
    Resources,
    /// The asset is larger than the configured upload limit.
    TooLarge,
    /// The supplied time zone or local time is invalid.
    Schedule,
    /// The login challenge expired.
    Expired,
    /// The supplied Apple account or password was rejected.
    Credentials,
    /// A transport or provider failure not safe to show verbatim.
    Unavailable,
}

impl WorkerError {
    /// Problems that require reconnecting instead of retrying the saved session.
    pub(super) const fn reconnect_hint(&self) -> Option<&'static str> {
        match self {
            Self::Auth => Some(RECONNECT),
            Self::WebAccess => Some(WEB_ACCESS),
            _ => None,
        }
    }
}

impl From<WorkerError> for Error {
    fn from(value: WorkerError) -> Self {
        match value {
            WorkerError::Auth | WorkerError::Expired => Self(StatusCode::CONFLICT, RECONNECT),
            WorkerError::WebAccess => Self(StatusCode::CONFLICT, WEB_ACCESS),
            WorkerError::Approval => Self(
                StatusCode::CONFLICT,
                "Approve iCloud web access on a trusted device, then try again",
            ),
            WorkerError::Resources => Self(
                StatusCode::SERVICE_UNAVAILABLE,
                "Sync is waiting for CPU, memory or disk space",
            ),
            WorkerError::TooLarge => Self(
                StatusCode::PAYLOAD_TOO_LARGE,
                "The iCloud file exceeds the upload limit",
            ),
            WorkerError::Schedule => {
                Self(StatusCode::BAD_REQUEST, "Choose a valid time and time zone")
            }
            WorkerError::Credentials => Self(
                StatusCode::BAD_REQUEST,
                "Apple did not accept those credentials",
            ),
            WorkerError::Unavailable => Self(
                StatusCode::BAD_GATEWAY,
                "iCloud Photos is unavailable. Try again",
            ),
        }
    }
}

/// Worker response envelope. Upstream text stays in the worker.
#[derive(Deserialize)]
struct Reply {
    /// Structured success value.
    result: Option<Value>,
    /// Sanitized failure category.
    error: Option<String>,
}

/// The shared worker and spool location.
pub(super) fn directory(s: &AppState) -> Result<&Path> {
    s.config.icloud_dir.as_deref().ok_or(Error(
        StatusCode::SERVICE_UNAVAILABLE,
        "Enable the iCloud worker to connect an Apple Account",
    ))
}

/// Send exactly one request on the local socket, with separate deadlines for slow media.
pub(super) async fn call<T: DeserializeOwned>(
    s: &AppState,
    request: Value,
    seconds: u64,
) -> std::result::Result<T, WorkerError> {
    let dir = s
        .config
        .icloud_dir
        .as_ref()
        .ok_or(WorkerError::Unavailable)?;
    let reply = tokio::time::timeout(
        Duration::from_secs(seconds),
        exchange(&dir.join("worker.sock"), request),
    )
    .await
    .map_err(|_| WorkerError::Unavailable)?
    .map_err(|_| WorkerError::Unavailable)?;
    if let Some(result) = reply.result {
        return serde_json::from_value(result).map_err(|_| WorkerError::Unavailable);
    }
    Err(match reply.error.as_deref() {
        Some("auth") => WorkerError::Auth,
        Some("web_access") => WorkerError::WebAccess,
        Some("approval") => WorkerError::Approval,
        Some("resources") => WorkerError::Resources,
        Some("too_large") => WorkerError::TooLarge,
        Some("schedule") => WorkerError::Schedule,
        Some("expired") => WorkerError::Expired,
        Some("credentials") => WorkerError::Credentials,
        _ => WorkerError::Unavailable,
    })
}

/// Exchange length-prefixed JSON with the Unix-domain worker.
#[cfg(unix)]
async fn exchange(path: &Path, request: Value) -> anyhow::Result<Reply> {
    let mut socket = tokio::net::UnixStream::connect(path).await?;
    let request = zeroize::Zeroizing::new(serde_json::to_vec(&request)?);
    anyhow::ensure!(request.len() <= MESSAGE_LIMIT);
    socket.write_u32(u32::try_from(request.len())?).await?;
    socket.write_all(&request).await?;
    let length = usize::try_from(socket.read_u32().await?)?;
    anyhow::ensure!(length <= MESSAGE_LIMIT);
    let mut response = zeroize::Zeroizing::new(vec![0; length]);
    socket.read_exact(&mut response).await?;
    Ok(serde_json::from_slice(&response)?)
}

/// Report the optional Linux connector as unavailable on platforms without Unix sockets.
#[cfg(not(unix))]
async fn exchange(_: &Path, _: Value) -> anyhow::Result<Reply> {
    anyhow::bail!("The iCloud worker requires Unix sockets")
}
