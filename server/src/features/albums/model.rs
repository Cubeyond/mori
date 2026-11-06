// SPDX-FileCopyrightText: 2026 Cubeyond
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Album metadata, mutations, and bounded collection-preview contracts.

use crate::features::videos::{MediaKind, Video};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;

/// Explicitly created album with an editable URL slug and display label.
#[derive(Serialize, FromRow, ToSchema)]
pub struct Album {
    /// Stable internal identifier for this record.
    pub id: String,
    /// Administrator-chosen album display label, independent of its slug.
    pub name: String,
    /// Unique editable album URL component.
    pub slug: String,
    /// Whether the album page can be visited anonymously, independent of member videos.
    pub visibility: String,
    /// Optional administrator-written description; empty values render no placeholder.
    #[schema(max_length = 500)]
    pub description: String,
    /// Creation timestamp in Unix seconds.
    pub created_at: i64,
}
/// Administrator-supplied album label and separately validated slug.
#[derive(Deserialize, ToSchema)]
pub struct CreateAlbum {
    /// Administrator-chosen album display label, independent of its slug.
    pub name: String,
    /// Unique editable album URL component.
    pub slug: String,
    /// Album-page visibility; omitted values default to private.
    #[serde(default = "private_visibility")]
    pub visibility: String,
    /// Optional plain-text description, trimmed and limited to 500 characters.
    #[serde(default)]
    #[schema(max_length = 500)]
    pub description: String,
}
/// Editable album label, optional URL slug, and page visibility.
#[derive(Deserialize, ToSchema)]
pub struct EditAlbum {
    /// Requested normalized URL slug; omission retains the current address.
    pub slug: Option<String>,
    /// Administrator-chosen album display label, independent of its slug.
    pub name: String,
    /// Requested album-page visibility; omission preserves its current value.
    pub visibility: Option<String>,
    /// Omission preserves the description; an empty string clears it.
    #[schema(max_length = 500)]
    pub description: Option<String>,
}
/// Explicit destructive confirmation and optional deletion of contained media.
#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct DeleteAlbum {
    /// Exact current album display name, required for both deletion modes.
    pub confirmation: String,
    /// Queue deletion of every member; false retains and detaches the videos.
    #[serde(default)]
    pub delete_media: bool,
}
/// Durable video deletions queued by an explicitly destructive album removal.
#[derive(Serialize, ToSchema)]
pub struct AlbumDeletion {
    /// Job identifiers whose completion confirms source and public-copy removal.
    pub job_ids: Vec<String>,
}
/// Album metadata and a bounded page of videos filtered for the caller.
#[derive(Serialize, ToSchema)]
pub struct AlbumPage {
    /// Exact number of members authorized for this caller and display-name filter.
    pub total: i64,
    /// Album metadata visible to the caller.
    pub album: Album,
    /// Visibility-filtered page of video records.
    pub videos: Vec<Video>,
    /// Whether another result page is available.
    pub has_more: bool,
}

/// Default newly created collections to administrator-only album pages.
fn private_visibility() -> String {
    "private".into()
}

/// Album metadata with a bounded, administrator-only set of completed media covers.
#[derive(Serialize, ToSchema)]
pub struct AlbumSummary {
    /// Ordinary album fields remain at the top level for existing clients.
    #[serde(flatten)]
    pub album: Album,
    /// Up to three recent posters; originals and incomplete uploads are never returned.
    #[schema(max_items = 3)]
    pub previews: Vec<AlbumPreview>,
}

/// Minimal completed cover; no filenames, object keys, or direct storage URLs are exposed.
#[derive(Serialize, ToSchema)]
pub struct AlbumPreview {
    /// Stable identifier used to retain the same preview during metadata refreshes.
    pub id: String,
    /// Media category determines whether the cover has a video indicator.
    pub kind: MediaKind,
    /// Same-origin authenticated JPEG endpoint, never a signed S3 address.
    pub poster_url: String,
}
