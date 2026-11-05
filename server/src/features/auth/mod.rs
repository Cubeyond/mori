//! Single-administrator sessions, password verification, and CSRF enforcement.

/// Environment-backed account provisioning.
mod bootstrap;
pub use bootstrap::configure;
/// Early administrator authorization and request-scoped storage coordination.
mod admin;
pub use admin::{Admin, SameOrigin};

use crate::{
    app::{AppState, now, random_id},
    http::error::{Error, Result, bad},
};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier, password_hash::SaltString};
use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode, header},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use utoipa::ToSchema;

/// Password submitted to establish the single administrator session.
#[derive(Deserialize, ToSchema)]
pub struct Login {
    /// Configured administrator name; omitted legacy requests use root.
    #[serde(default = "default_username")]
    username: String,
    /// Administrator password; never persisted or logged in plaintext.
    password: String,
}
/// Preserve the default account name for older password-only clients.
fn default_username() -> String {
    "root".into()
}
/// Session discovery response, including CSRF only for authenticated callers.
#[derive(Serialize, ToSchema)]
pub struct Session {
    /// Whether the supplied administrator session is valid.
    pub authenticated: bool,
    /// Session-bound CSRF value returned only to its authenticated browser.
    #[schema(required = true)]
    pub csrf: Option<String>,
}

/// Hash a password using the memory-bounded Argon2id configuration.
///
/// # Errors
/// Returns an Argon2 password-hashing error.
pub fn hash_password(password: &str) -> anyhow::Result<String> {
    Ok(Argon2::default()
        .hash_password(
            password.as_bytes(),
            &SaltString::generate(&mut rand::rngs::OsRng),
        )?
        .to_string())
}
/// Hash an opaque session value before storing or looking it up.
fn digest(token: &str) -> String {
    hex::encode(Sha256::digest(token.as_bytes()))
}
/// Choose the secure host-only cookie name for the configured origin.
fn cookie_name(s: &AppState) -> &'static str {
    if s.config.public_url.starts_with("https:") {
        "__Host-mori"
    } else {
        "mori_dev"
    }
}
/// Extract a syntactically valid session value from the request cookies.
fn token(s: &AppState, h: &HeaderMap) -> Option<String> {
    h.get(header::COOKIE)?
        .to_str()
        .ok()?
        .split(';')
        .find_map(|part| {
            let (name, value) = part.trim().split_once('=')?;
            (name == cookie_name(s)
                && value.len() == 64
                && value.bytes().all(|b| b.is_ascii_hexdigit()))
            .then(|| value.to_owned())
        })
}
/// Reject mutations whose browser origin differs from the configured origin.
///
/// # Errors
/// Returns 403 when the Origin header is absent or differs from the configured origin.
pub fn check_origin(s: &AppState, h: &HeaderMap) -> Result<()> {
    if h.get(header::ORIGIN).and_then(|v| v.to_str().ok()) != Some(s.config.public_url.as_str()) {
        return Err(Error(StatusCode::FORBIDDEN, "Invalid request origin"));
    }
    Ok(())
}
/// Validate the administrator session and, for mutations, Origin and CSRF.
///
/// # Errors
/// Returns 401 for invalid sessions, 403 for Origin or CSRF failures, or a database error.
pub async fn require(s: &AppState, h: &HeaderMap, mutation: bool) -> Result<String> {
    let token = token(s, h).ok_or(Error(StatusCode::UNAUTHORIZED, "Log in required"))?;
    let csrf: Option<String> =
        sqlx::query_scalar("SELECT csrf FROM sessions WHERE token_hash=? AND expires_at>?")
            .bind(digest(&token))
            .bind(now())
            .fetch_optional(&s.db)
            .await?;
    let csrf = csrf.ok_or(Error(StatusCode::UNAUTHORIZED, "Session expired"))?;
    if mutation {
        check_origin(s, h)?;
        if h.get("x-csrf-token").and_then(|v| v.to_str().ok()) != Some(csrf.as_str()) {
            return Err(Error(StatusCode::FORBIDDEN, "Invalid CSRF token"));
        }
    }
    Ok(csrf)
}

/// Verify the administrator password and establish a revocable session.
///
/// # Errors
/// Returns authentication, rate-limit, bootstrap, header, password-worker, or database errors.
#[utoipa::path(
    post,
    path = "/v1/auth/login",
    request_body = Login,
    responses(
        (status = 200, body = Session),
        (status = 401, description = "Invalid password"),
        (status = 429, description = "Try later"),
    ),
)]
pub async fn login(
    State(s): State<AppState>,
    _origin: SameOrigin,
    Json(input): Json<Login>,
) -> Result<(HeaderMap, Json<Session>)> {
    if input.password.len() > 1024 {
        return Err(bad("Password too long"));
    }
    let slot = s
        .password_slots
        .clone()
        .try_acquire_owned()
        .map_err(|_| Error(StatusCode::TOO_MANY_REQUESTS, "Try again shortly"))?;
    {
        let backoff = s.login_backoff.lock().await;
        if backoff.1 > now() {
            return Err(Error(StatusCode::TOO_MANY_REQUESTS, "Try again shortly"));
        }
    }
    let stored: Option<(String, String)> =
        sqlx::query_as("SELECT username,password_hash FROM admin WHERE id=1")
            .fetch_optional(&s.db)
            .await?;
    let stored = stored.ok_or(Error(
        StatusCode::SERVICE_UNAVAILABLE,
        "Administrator has not been initialized",
    ))?;
    let valid = tokio::task::spawn_blocking(move || {
        let _slot = slot;
        let password_matches = PasswordHash::new(&stored.1).is_ok_and(|hash| {
            Argon2::default()
                .verify_password(input.password.as_bytes(), &hash)
                .is_ok()
        });
        password_matches && input.username == stored.0
    })
    .await
    .map_err(|_| {
        Error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Password verification failed",
        )
    })?;
    let mut backoff = s.login_backoff.lock().await;
    if !valid {
        backoff.0 = (backoff.0 + 1).min(8);
        backoff.1 = now() + (1i64 << backoff.0);
        return Err(Error(StatusCode::UNAUTHORIZED, "Invalid credentials"));
    }
    *backoff = (0, 0);
    drop(backoff);
    let value = format!("{}{}", random_id(), random_id());
    let csrf = random_id();
    let mut tx = s.db.begin().await?;
    sqlx::query("DELETE FROM sessions WHERE expires_at<=?")
        .bind(now())
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO sessions VALUES(?,?,?)")
        .bind(digest(&value))
        .bind(&csrf)
        .bind(now() + 86400)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    let secure = if s.config.public_url.starts_with("https:") {
        "; Secure"
    } else {
        ""
    };
    let mut headers = HeaderMap::new();
    headers.insert(
        header::SET_COOKIE,
        format!(
            "{}={value}; Path=/; HttpOnly; SameSite=Lax; Max-Age=86400{secure}",
            cookie_name(&s)
        )
        .parse()
        .map_err(|_| bad("Invalid cookie configuration"))?,
    );
    Ok((
        headers,
        Json(Session {
            authenticated: true,
            csrf: Some(csrf),
        }),
    ))
}
/// Report session validity and its associated CSRF token.
///
/// # Errors
/// Returns a database error; absent or expired sessions produce an unauthenticated response.
#[utoipa::path(
    get,
    path = "/v1/auth/session",
    responses((status = 200, body = Session)),
)]
pub async fn session(State(s): State<AppState>, h: HeaderMap) -> Result<Json<Session>> {
    match require(&s, &h, false).await {
        Ok(csrf) => Ok(Json(Session {
            authenticated: true,
            csrf: Some(csrf),
        })),
        Err(Error(StatusCode::UNAUTHORIZED, _)) => Ok(Json(Session {
            authenticated: false,
            csrf: None,
        })),
        Err(e) => Err(e),
    }
}
/// Revoke the current session and expire its browser cookie.
///
/// # Errors
/// Returns authentication, Origin, CSRF, cookie-header, or database errors.
#[utoipa::path(
    post,
    path = "/v1/auth/logout",
    responses((status = 204, description = "Session revoked")),
    security(("session" = [])),
)]
pub async fn logout(State(s): State<AppState>, h: HeaderMap) -> Result<(StatusCode, HeaderMap)> {
    require(&s, &h, true).await?;
    if let Some(value) = token(&s, &h) {
        sqlx::query("DELETE FROM sessions WHERE token_hash=?")
            .bind(digest(&value))
            .execute(&s.db)
            .await?;
    }
    let secure = if s.config.public_url.starts_with("https:") {
        "; Secure"
    } else {
        ""
    };
    let mut headers = HeaderMap::new();
    headers.insert(
        header::SET_COOKIE,
        format!(
            "{}=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0{secure}",
            cookie_name(&s)
        )
        .parse()
        .map_err(|_| bad("Invalid cookie configuration"))?,
    );
    Ok((StatusCode::NO_CONTENT, headers))
}
