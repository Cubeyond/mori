// SPDX-FileCopyrightText: 2026 Cubeyond
// SPDX-License-Identifier: Elastic-2.0

//! Sanitized HTTP errors shared by request handlers.

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use utoipa::ToSchema;

/// HTTP status and sanitized message for an application failure.
#[derive(Debug)]
pub struct Error(
    /// HTTP status exposed to the caller.
    pub StatusCode,
    /// Static explanation that never includes upstream credentials or signed URLs.
    pub &'static str,
);
/// Stable JSON error envelope returned to API callers.
#[derive(Serialize, ToSchema)]
pub struct ErrorBody {
    /// Sanitized explanatory message safe to return to the caller.
    pub error: String,
}
impl IntoResponse for Error {
    /// Map an application error into its sanitized JSON HTTP response.
    fn into_response(self) -> Response {
        (
            self.0,
            Json(ErrorBody {
                error: self.1.into(),
            }),
        )
            .into_response()
    }
}
impl From<sqlx::Error> for Error {
    /// Translate an upstream failure into a non-sensitive application error.
    fn from(e: sqlx::Error) -> Self {
        if let sqlx::Error::Database(db) = &e
            && db.is_unique_violation()
        {
            return Self(StatusCode::CONFLICT, "Already exists");
        }
        tracing::error!(error = %e, "database operation failed");
        Self(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Database operation failed",
        )
    }
}
impl From<anyhow::Error> for Error {
    /// Translate an upstream failure into a non-sensitive application error.
    fn from(e: anyhow::Error) -> Self {
        let _ = e;
        tracing::error!("storage operation failed");
        Self(
            StatusCode::BAD_GATEWAY,
            "Storage operation failed; retry later",
        )
    }
}
impl From<crate::infra::storage_settings::Unconfigured> for Error {
    /// Return a setup requirement for media operations on an unconfigured installation.
    fn from(_: crate::infra::storage_settings::Unconfigured) -> Self {
        Self(
            StatusCode::SERVICE_UNAVAILABLE,
            "Configure Storage before uploading or playing private media",
        )
    }
}
/// Handler result that exposes only sanitized application errors.
pub type Result<T> = std::result::Result<T, Error>;
/// Construct a sanitized invalid-request response.
#[must_use]
pub const fn bad(message: &'static str) -> Error {
    Error(StatusCode::BAD_REQUEST, message)
}
/// Construct the shared inaccessible-or-missing target response.
#[must_use]
pub const fn missing() -> Error {
    Error(StatusCode::NOT_FOUND, "Not found")
}
