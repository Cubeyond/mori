//! Authenticate administrator requests before parsing bodies or acquiring storage gates.

use super::{check_origin, require};
use crate::{app::AppState, http::error::Result};
use axum::{
    extract::FromRequestParts,
    http::{HeaderMap, Method, request::Parts},
};
use tokio::sync::OwnedRwLockReadGuard;

/// Login checks its browser origin before accepting a password-bearing request body.
pub struct SameOrigin;

impl FromRequestParts<AppState> for SameOrigin {
    /// Origin failures use the same sanitized error contract as authenticated mutations.
    type Rejection = crate::http::error::Error;

    /// Reject absent and foreign origins before reading JSON or entering the password worker.
    fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> impl std::future::Future<Output = Result<Self>> + Send {
        std::future::ready(check_origin(state, &parts.headers).map(|()| Self))
    }
}

/// An authenticated request retains its media mutation gate until its handler returns.
pub struct Admin {
    /// Read-only metadata remains available while storage configuration is being replaced.
    _operation: Option<OwnedRwLockReadGuard<()>>,
}

impl Admin {
    /// Authorize server-rendered read operations through the same session contract as HTTP.
    ///
    /// # Errors
    /// Returns invalid sessions or database failures before exposing administrator metadata.
    pub(crate) async fn read(state: &AppState, headers: &HeaderMap) -> Result<Self> {
        require(state, headers, false).await?;
        Ok(Self { _operation: None })
    }
}

impl FromRequestParts<AppState> for Admin {
    /// Authentication and CSRF failures are normal sanitized HTTP errors.
    type Rejection = crate::http::error::Error;

    /// Reject unauthenticated or cross-origin requests before consuming an arbitrary body.
    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self> {
        let mutation = parts.method != Method::GET && parts.method != Method::HEAD;
        require(state, &parts.headers, mutation).await?;
        let configuration = matches!(
            parts.uri.path(),
            "/v1/admin/storage" | "/v1/admin/storage/test"
        );
        let operation = if mutation && !configuration {
            Some(state.storage.operation().await)
        } else {
            None
        };
        Ok(Self {
            _operation: operation,
        })
    }
}
