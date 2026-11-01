//! Video contracts and context-dependent links, independent of HTTP handlers.

use crate::app::AppState;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;

/// Select a safe Content-Type from a container extension without enforcing a video codec.
#[must_use]
pub fn content_type(extension: &str) -> &'static str {
    match extension {
        "mp4" | "m4v" => "video/mp4",
        "mov" => "video/quicktime",
        "webm" => "video/webm",
        "mkv" => "video/x-matroska",
        "avi" => "video/x-msvideo",
        "ogv" => "video/ogg",
        "mpeg" | "mpg" => "video/mpeg",
        "ts" | "mts" | "m2ts" => "video/mp2t",
        "3gp" => "video/3gpp",
        "wmv" => "video/x-ms-wmv",
        "flv" => "video/x-flv",
        _ => "application/octet-stream",
    }
}
/// Video metadata and visibility-dependent delivery URLs.
#[derive(Serialize, FromRow, ToSchema)]
pub struct Video {
    /// Stable internal identifier for this record.
    pub id: String,
    /// Unpredictable public video identifier independent of display names.
    pub token: String,
    /// Immutable original container extension used in public resource URLs.
    pub extension: String,
    /// Current public URL namespace, stored separately from the display name.
    #[serde(skip_serializing)]
    pub media_prefix: String,
    /// Optional explicit album membership; absent means standalone.
    #[schema(required = true)]
    pub album_id: Option<String>,
    /// Display name; uploads receive a fresh server-generated hash.
    pub name: String,
    /// Requested public or private access policy.
    pub visibility: String,
    /// Persisted progress or terminal state of the operation.
    pub state: String,
    /// Expected or validated object length in bytes.
    pub size: i64,
    /// Validated movie duration in seconds.
    pub duration: f64,
    /// Displayed video width after track rotation is accounted for.
    pub width: i64,
    /// Displayed video height after track rotation is accounted for.
    pub height: i64,
    /// Whether a JPEG poster has been stored.
    pub poster: i64,
    /// Creation timestamp in Unix seconds.
    pub created_at: i64,
    /// Monotonic version guarding obsolete asynchronous operations.
    pub revision: i64,
    /// Sanitized actionable failure shown in administrator management.
    #[schema(required = true)]
    pub last_error: Option<String>,
    /// Public direct URL or authenticated private content endpoint.
    #[sqlx(skip)]
    pub media_url: String,
    /// Optional public or authenticated poster endpoint.
    #[sqlx(skip)]
    #[schema(required = true)]
    pub poster_url: Option<String>,
}
impl Video {
    /// Attach context-appropriate media and stable sharing URLs.
    #[must_use]
    pub fn links(mut self, s: &AppState, admin: bool) -> Self {
        self.media_url = if admin && self.state != "published" {
            format!("/v1/admin/videos/{}/content", self.id)
        } else {
            format!(
                "{}/{}/{}.{}",
                s.config.public_url, self.media_prefix, self.token, self.extension
            )
        };
        if self.poster != 0 {
            self.poster_url = Some(if admin && self.state != "published" {
                format!("/v1/admin/videos/{}/poster", self.id)
            } else {
                format!("{}/files/{}.jpg", s.config.public_url, self.token)
            });
        }
        self
    }
}
/// Editable metadata, optional album membership, and requested visibility.
#[derive(Deserialize, ToSchema)]
pub struct EditVideo {
    /// Display name; uploads receive a fresh server-generated hash.
    pub name: String,
    /// Optional explicit album membership; absent means standalone.
    pub album_id: Option<String>,
    /// Requested public or private access policy.
    pub visibility: String,
}
/// Mutation acknowledgement with an optional durable background job.
#[derive(Serialize, ToSchema)]
pub struct Accepted {
    /// Optional durable task identifier for an asynchronous transition.
    #[schema(required = true)]
    pub job_id: Option<String>,
}
/// Bounded video collection with continuation information.
#[derive(Serialize, ToSchema)]
pub struct VideoPage {
    /// Visibility-filtered page of video records.
    pub videos: Vec<Video>,
    /// Whether another result page is available.
    pub has_more: bool,
}
