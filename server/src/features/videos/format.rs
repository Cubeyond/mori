//! Shared image/video classification keeps upload policy and delivery metadata consistent.

use serde::Serialize;
use utoipa::ToSchema;

/// Media category independent of its container and browser decoding support.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, sqlx::Type, ToSchema)]
#[serde(rename_all = "lowercase")]
#[sqlx(type_name = "TEXT", rename_all = "lowercase")]
pub enum MediaKind {
    /// An original still or animated image, including browser-unsupported formats.
    Image,
    /// An original video recording, without a codec restriction.
    Video,
}

/// Resolve a registered container without guessing codecs or transforming the original bytes.
#[must_use]
pub fn content_type(extension: &str) -> &'static str {
    match extension {
        "jpg" | "jpeg" => "image/jpeg",
        "png" | "apng" => "image/png",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "avif" => "image/avif",
        "bmp" => "image/bmp",
        "ico" => "image/vnd.microsoft.icon",
        "tif" | "tiff" => "image/tiff",
        "heic" => "image/heic",
        "heif" => "image/heif",
        "jxl" => "image/jxl",
        "svg" => "image/svg+xml",
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
