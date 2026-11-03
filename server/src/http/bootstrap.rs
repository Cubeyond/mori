//! Request-scoped initial UI data prevents anonymous shells and placeholder-title flashes.

use crate::{
    app::AppState,
    features::{
        albums::{self, AlbumPage, AlbumSummary},
        auth::{self, Session},
    },
    http::{
        error::{Error, Result},
        pagination::Page,
    },
};
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
};
use serde::Serialize;

/// Initial authorized metadata delivered only with this browser's uncached document.
#[derive(Serialize)]
struct PageBootstrap {
    /// Exact application path associated with the initial collection data.
    path: String,
    /// Validated session state and its existing CSRF value; never an authentication cookie.
    session: Session,
    /// Initial administrator album listing when requested by a collection route.
    albums: Option<Vec<AlbumSummary>>,
    /// Initial authorized collection and its first bounded media page.
    album_page: Option<AlbumPage>,
}

/// Read the immutable build and attach fresh, escaped, caller-authorized initial metadata.
///
/// # Errors
/// Returns document availability, database, or serialization failures without exposing secrets.
pub(super) async fn document(state: &AppState, headers: &HeaderMap, path: &str) -> Result<String> {
    let mut body = tokio::fs::read_to_string(format!("{}/index.html", state.config.web_dir))
        .await
        .map_err(|_| {
            Error(
                StatusCode::SERVICE_UNAVAILABLE,
                "Frontend build unavailable",
            )
        })?;
    let session = auth::session(State(state.clone()), headers.clone())
        .await?
        .0;
    let admin_album = session.authenticated && (path == "/albums" || path.starts_with("/albums/"));
    let albums = if admin_album {
        Some(albums::list(State(state.clone()), headers.clone()).await?.0)
    } else {
        None
    };
    let result = if session.authenticated
        && let Some(id) = path.strip_prefix("/albums/")
    {
        Some(
            albums::detail(
                State(state.clone()),
                headers.clone(),
                Path(id.into()),
                Query(Page::default()),
            )
            .await,
        )
    } else if let Some(slug) = path
        .strip_prefix('/')
        .filter(|slug| !slug.contains('/') && albums::normalize_slug(slug).is_ok())
    {
        Some(
            albums::public(
                State(state.clone()),
                headers.clone(),
                Path(slug.into()),
                Query(Page::default()),
            )
            .await,
        )
    } else {
        None
    };
    let album_page = match result {
        Some(Ok(page)) => Some(page.0),
        Some(Err(Error(StatusCode::NOT_FOUND, _))) | None => None,
        Some(Err(error)) => return Err(error),
    };
    let title = page_title(path, album_page.as_ref());
    if let Some(start) = body.find("<title>")
        && let Some(end) = body[start..].find("</title>")
    {
        body.replace_range(start + 7..start + end, &title);
    }
    let data = serde_json::to_string(&PageBootstrap {
        path: path.into(),
        session,
        albums,
        album_page,
    })
    .map_err(|_| {
        Error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Page initialization failed",
        )
    })?
    .replace('<', "\\u003c");
    let script = format!("<script id=\"mori-bootstrap\" type=\"application/json\">{data}</script>");
    if let Some(head) = body.find("</head>") {
        body.insert_str(head, &script);
    }
    Ok(body)
}

/// Set the initial tab title without exposing inaccessible albums or accepting HTML markup.
fn page_title(path: &str, album: Option<&AlbumPage>) -> String {
    let label = match path {
        "/login" => Some("Login"),
        "/videos" | "/gallery" => Some("Gallery"),
        "/albums" => Some("Albums"),
        "/settings" => Some("Preferences"),
        "/settings/storage" => Some("Storage"),
        _ => album.map(|page| page.album.name.as_str()),
    };
    label.map_or_else(
        || "Mori".into(),
        |name| {
            format!(
                "{} – Mori",
                name.trim()
                    .replace('&', "&amp;")
                    .replace('<', "&lt;")
                    .replace('>', "&gt;")
            )
        },
    )
}
