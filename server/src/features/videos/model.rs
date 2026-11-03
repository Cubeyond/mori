//! Video contracts and context-dependent links, independent of HTTP handlers.

use super::MediaKind;
use crate::app::AppState;
use crate::infra::storage::Storage;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;

/// Image/video metadata; the legacy DTO name and API routes preserve existing integrations.
#[derive(Serialize, FromRow, ToSchema)]
pub struct Video {
    /// Original media category, independent of extension or browser codec support.
    pub kind: MediaKind,
    /// Stable internal identifier for this record.
    pub id: String,
    /// Unpredictable public video identifier independent of display names.
    pub token: String,
    /// Immutable original container extension used in public resource URLs.
    pub extension: String,
    /// Current public URL namespace, stored separately from the display name.
    #[serde(skip_serializing)]
    pub media_prefix: String,
    /// Private object location; absent values retain the pre-migration source convention.
    #[serde(skip_serializing)]
    pub source_key: Option<String>,
    /// Private preview location; absent values preserve historical video poster paths.
    #[serde(skip_serializing)]
    pub poster_key: Option<String>,
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
    /// Select the current original or derivative after the caller has authorized access.
    #[must_use]
    pub(crate) fn content_key(&self, poster: bool) -> String {
        match (self.state.as_str(), poster) {
            ("published", true) => self.public_poster(),
            ("published", false) => Storage::public(&self.token, &self.extension),
            (_, true) => self.private_poster(),
            (_, false) => self.private_source(),
        }
    }
    /// Resolve private storage without invalidating unfinished legacy multipart sessions.
    #[must_use]
    pub fn private_source(&self) -> String {
        self.source_key
            .clone()
            .unwrap_or_else(|| Storage::source(&self.id))
    }
    /// Resolve the poster using the same storage generation as its private video.
    #[must_use]
    pub fn private_poster(&self) -> String {
        if let Some(key) = &self.poster_key {
            return key.clone();
        }
        if self.source_key.is_some() {
            Storage::private_poster(&self.token)
        } else {
            Storage::poster(&self.id)
        }
    }
    /// Published preview key remains distinct from JPEG image originals and legacy posters.
    #[must_use]
    pub fn public_poster(&self) -> String {
        if self.poster_key.is_some() {
            Storage::published_preview(&self.token)
        } else {
            Storage::public_poster(&self.token)
        }
    }
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
                format!(
                    "{}/{}/{}.jpg",
                    s.config.public_url,
                    if self.poster_key.is_some() {
                        "posters"
                    } else {
                        "files"
                    },
                    self.token
                )
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
    /// Exact authorized result count after applying the display-name filter.
    pub total: i64,
    /// Visibility-filtered page of video records.
    pub videos: Vec<Video>,
    /// Whether another result page is available.
    pub has_more: bool,
}
