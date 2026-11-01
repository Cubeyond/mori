//! Business features for identity, albums, uploads, and video access.

/// Explicit albums with stable slugs and filtered public pages.
pub mod albums;
/// Single-administrator sessions, password verification, and CSRF.
pub mod auth;
/// Resumable S3 multipart sessions and server-generated names.
pub mod uploads;
/// Video metadata, visibility, and stable delivery links.
pub mod videos;

/// Encrypted S3 setup and credential management through administration.
pub mod storage;
