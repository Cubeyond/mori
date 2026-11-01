//! HTTP routing, generated OpenAPI, static pages, and response security policies.

/// Fresh authorized UI initialization attached to uncached HTML documents.
mod bootstrap;
/// Sanitized API errors and JSON response mapping.
pub mod error;
/// Shared collection query bounds.
pub mod pagination;
/// Binary HTTP schema definitions shared by media operations.
pub mod schema;
use crate::{
    app::{AppState, valid_token},
    features::{albums, auth, storage, uploads, videos},
    jobs,
};
use axum::{
    Router,
    extract::{OriginalUri, Path, Request, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::get,
};
use tower_http::services::ServeDir;
use utoipa_axum::{router::OpenApiRouter, routes};

/// Register HTTP handlers and their generated API descriptions together.
fn api() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(auth::login))
        .routes(routes!(auth::session))
        .routes(routes!(auth::logout))
        .routes(routes!(storage::get, storage::save))
        .routes(routes!(storage::test))
        .routes(routes!(albums::list, albums::create))
        .routes(routes!(albums::detail, albums::edit, albums::delete))
        .routes(routes!(albums::public))
        .routes(routes!(videos::public))
        .routes(routes!(videos::list))
        .routes(routes!(videos::edit, videos::delete))
        .routes(routes!(videos::content))
        .routes(routes!(videos::poster, uploads::poster))
        .routes(routes!(uploads::create))
        .routes(routes!(uploads::policy))
        .routes(routes!(uploads::part))
        .routes(routes!(uploads::complete))
        .routes(routes!(uploads::cancel, uploads::status))
        .routes(routes!(jobs::status))
}
/// Generate a deterministic contract with globally unique operation identifiers.
pub fn openapi() -> utoipa::openapi::OpenApi {
    let (_, mut spec) = api().split_for_parts();
    spec.info = utoipa::openapi::Info::new("Mori API", env!("CARGO_PKG_VERSION"));
    spec.info.description = Some(
        concat!(
            "Administrative requests require a session cookie. Mutations require the ",
            "exact MORI_PUBLIC_URL Origin and X-CSRF-Token from the session response. ",
            "HTTPS uses __Host-mori; localhost development uses mori_dev. Private and ",
            "missing public targets both return 404. Visibility changes and deletion ",
            "return asynchronous job IDs; private completion waits for public-object ",
            "deletion and, when configured, confirmed CloudFront revocation.",
        )
        .into(),
    );
    for (path, item) in &mut spec.paths.paths {
        for operation in [
            &mut item.post,
            &mut item.put,
            &mut item.patch,
            &mut item.delete,
        ]
        .into_iter()
        .flatten()
        {
            use utoipa::openapi::security::SecurityRequirement;
            let mut security = SecurityRequirement::new("origin", Vec::<String>::new());
            if path != "/v1/auth/login" {
                security = security
                    .add("session", Vec::<String>::new())
                    .add("csrf", Vec::<String>::new());
            }
            operation.security = Some(vec![security]);
        }
    }

    let components = spec.components.get_or_insert_with(Default::default);
    components.add_security_scheme(
        "session",
        utoipa::openapi::security::SecurityScheme::ApiKey(
            utoipa::openapi::security::ApiKey::Cookie(utoipa::openapi::security::ApiKeyValue::new(
                "__Host-mori",
            )),
        ),
    );
    for (name, header) in [("csrf", "X-CSRF-Token"), ("origin", "Origin")] {
        components.add_security_scheme(
            name,
            utoipa::openapi::security::SecurityScheme::ApiKey(
                utoipa::openapi::security::ApiKey::Header(
                    utoipa::openapi::security::ApiKeyValue::new(header),
                ),
            ),
        );
    }
    for path in [
        "/v1/admin/videos/{id}/content",
        "/v1/admin/videos/{id}/poster",
    ] {
        if let Some(item) = spec.paths.paths.get_mut(path) {
            item.head = item.get.clone();
            if let Some(op) = &mut item.head {
                op.operation_id = Some(format!(
                    "{}_head",
                    op.operation_id.as_deref().unwrap_or("media")
                ));
            }
        }
    }
    for (path, item) in &mut spec.paths.paths {
        let suffix = path
            .split(|c: char| !c.is_ascii_alphanumeric())
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join("_");
        for (method, operation) in [
            ("get", &mut item.get),
            ("post", &mut item.post),
            ("put", &mut item.put),
            ("patch", &mut item.patch),
            ("delete", &mut item.delete),
            ("head", &mut item.head),
        ] {
            if let Some(operation) = operation {
                operation.operation_id = Some(format!("{method}_{suffix}"));
            }
        }
    }
    spec
}
/// Assemble API routes, restricted SPA fallbacks, and response security policies.
pub fn router(s: AppState) -> Router {
    let (api, _) = api().split_for_parts();
    let assets = format!("{}/_app", s.config.web_dir);
    api.route(
        "/v1/openapi.yml",
        get(|| async {
            (
                [(header::CONTENT_TYPE, "application/yaml")],
                openapi()
                    .to_yaml()
                    .unwrap_or_else(|_| String::from("error: specification serialization failed")),
            )
        }),
    )
    .route("/health", get(|| async { StatusCode::NO_CONTENT }))
    .route("/", get(index))
    .route("/login", get(index))
    .route("/videos", get(index))
    .route("/albums", get(index))
    .route("/albums/{id}", get(index))
    .route("/settings", get(index))
    .route("/settings/storage", get(index))
    .route("/files/{filename}", get(videos::public_file))
    .route("/{prefix}/{filename}", get(videos::shared_resource))
    .route("/{slug}", get(album_page))
    .nest_service("/_app", ServeDir::new(assets))
    .nest_service(
        "/fonts",
        ServeDir::new(format!("{}/fonts", s.config.web_dir)),
    )
    .route_service(
        "/favicon.svg",
        tower_http::services::ServeFile::new(format!("{}/favicon.svg", s.config.web_dir)),
    )
    .fallback(|| async { error::missing() })
    .layer(axum::extract::DefaultBodyLimit::max(1_048_576))
    .layer(middleware::from_fn_with_state(s.clone(), security))
    .with_state(s)
}
/// Serve a fresh authorized page seed alongside the immutable frontend build.
async fn index(State(s): State<AppState>, h: HeaderMap, OriginalUri(uri): OriginalUri) -> Response {
    match bootstrap::document(&s, &h, uri.path()).await {
        Ok(body) => (
            [
                (header::CONTENT_TYPE, "text/html; charset=utf-8"),
                (header::CACHE_CONTROL, "private,no-store"),
                (header::VARY, "Cookie"),
            ],
            body,
        )
            .into_response(),
        Err(error) => error.into_response(),
    }
}
/// Serve a valid public album-shaped page without disclosing private collection data.
async fn album_page(
    State(s): State<AppState>,
    h: HeaderMap,
    OriginalUri(uri): OriginalUri,
    Path(slug): Path<String>,
) -> Response {
    if albums::normalize_slug(&slug).is_err() {
        return error::missing().into_response();
    }
    index(State(s), h, OriginalUri(uri)).await
}
/// Recognize direct media paths so resource redirects can be framed cross-site.
fn media_path(path: &str) -> bool {
    let mut segments = path.trim_start_matches('/').split('/');
    let Some(prefix) = segments.next() else {
        return false;
    };
    let Some(filename) = segments.next() else {
        return false;
    };
    if prefix.is_empty() || segments.next().is_some() {
        return false;
    }
    filename.rsplit_once('.').is_some_and(|(token, extension)| {
        valid_token(token)
            && !extension.is_empty()
            && extension.len() <= 12
            && extension
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
    })
}
/// Enforce origin authentication and apply route-specific browser security headers.
async fn security(State(s): State<AppState>, request: Request, next: Next) -> Response {
    if request.uri().path() != "/health"
        && let Some(secret) = &s.origin_secret
        && request
            .headers()
            .get("x-mori-origin")
            .and_then(|h| h.to_str().ok())
            != Some(secret.as_str())
    {
        return StatusCode::FORBIDDEN.into_response();
    }
    let embed = media_path(request.uri().path());
    let assets = request.uri().path().starts_with("/_app/immutable/")
        || request.uri().path().starts_with("/fonts/");
    let _storage_operation = if request.uri().path().starts_with("/v1/admin/")
        && !request.uri().path().starts_with("/v1/admin/storage")
    {
        Some(s.storage.operation().await)
    } else {
        None
    };
    let mut response = next.run(request).await;
    let h = response.headers_mut();
    h.insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );
    h.insert("referrer-policy", HeaderValue::from_static("no-referrer"));
    h.insert(
        "permissions-policy",
        HeaderValue::from_static(if embed {
            "camera=(), microphone=(), geolocation=(), fullscreen=*"
        } else {
            "camera=(), microphone=(), geolocation=(), fullscreen=(self)"
        }),
    );
    let ancestors = if embed { "*" } else { "'none'" };
    let connect = s.config.s3_endpoint.as_ref().map_or_else(
        || String::from("https://*.amazonaws.com https://*.amazonaws.com.cn"),
        |endpoint| {
            url::Url::parse(endpoint)
                .map(|u| u.origin().ascii_serialization())
                .unwrap_or_default()
        },
    );
    let script_csp = &s.script_csp;
    let media = s.storage.current().map_or_else(String::new, |storage| {
        if storage.distribution_id.is_some() {
            String::new()
        } else if s.config.s3_endpoint.is_some() {
            connect.clone()
        } else {
            storage.browser_sources()
        }
    });
    let csp = format!(
        concat!(
            "default-src 'self'; script-src 'self' {script_csp}; ",
            "style-src 'self' 'unsafe-inline'; img-src 'self' blob: data: {media}; ",
            "media-src 'self' blob: {media}; connect-src 'self' {connect}; ",
            "font-src 'self'; object-src 'none'; base-uri 'none'; ",
            "frame-ancestors {ancestors}",
        ),
        script_csp = script_csp,
        connect = connect,
        media = media,
        ancestors = ancestors,
    );
    if let Ok(value) = csp.parse() {
        h.insert("content-security-policy", value);
    }
    h.entry(header::CACHE_CONTROL)
        .or_insert(HeaderValue::from_static(if assets {
            "public,max-age=31536000,immutable"
        } else {
            "no-store"
        }));
    response
}
