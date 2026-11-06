// SPDX-FileCopyrightText: 2026 Cubeyond
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Validated storage identity and write-only credentials persisted inside the encrypted vault.

use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

/// Internal configuration; never serialize this type into an HTTP response or log.
#[derive(Clone, Serialize, Deserialize)]
pub struct StorageSettings {
    /// Existing private Amazon S3 bucket.
    pub bucket: String,
    /// AWS region hosting the bucket.
    pub region: String,
    /// Optional CDN distribution; absent means signed S3 playback of published copies.
    pub distribution_id: Option<String>,
    /// AWS credential identifier, kept inside the encrypted record.
    pub access_key_id: String,
    /// AWS secret; neither response DTOs nor logging expose it.
    pub secret_access_key: String,
    /// Optional temporary-credential session token.
    pub session_token: Option<String>,
}
impl StorageSettings {
    /// Validate a normal Amazon S3 location and bounded credential fields.
    ///
    /// # Errors
    /// Returns invalid naming, region, distribution, or credential input.
    pub fn validate(&self) -> Result<()> {
        ensure!(
            (3..=63).contains(&self.bucket.len())
                && self
                    .bucket
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b".-".contains(&b))
                && self.bucket.starts_with(|c: char| c.is_ascii_alphanumeric())
                && self.bucket.ends_with(|c: char| c.is_ascii_alphanumeric())
                && !self.bucket.contains("..")
                && self.bucket.parse::<std::net::Ipv4Addr>().is_err()
                && !["xn--", "sthree-", "amzn-s3-demo-"]
                    .iter()
                    .any(|prefix| self.bucket.starts_with(prefix))
                && !["-s3alias", "--ol-s3", ".mrap", "--x-s3", "--table-s3"]
                    .iter()
                    .any(|suffix| self.bucket.ends_with(suffix)),
            "Invalid S3 bucket name"
        );
        ensure!(
            (3..=64).contains(&self.region.len())
                && self
                    .region
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-'),
            "Invalid AWS region"
        );
        ensure!(
            self.distribution_id.as_ref().is_none_or(|id| !id.is_empty()
                && id.len() <= 64
                && id.bytes().all(|b| b.is_ascii_alphanumeric())),
            "Invalid CloudFront distribution ID"
        );
        ensure!(
            !self.access_key_id.is_empty()
                && self.access_key_id.len() <= 256
                && self
                    .access_key_id
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric()),
            "Invalid access key ID"
        );
        ensure!(
            !self.secret_access_key.is_empty()
                && self.secret_access_key.len() <= 4096
                && self.secret_access_key.bytes().all(|b| b.is_ascii_graphic()),
            "Invalid secret access key"
        );
        ensure!(
            self.session_token.as_ref().is_none_or(
                |token| token.len() <= 16384 && token.bytes().all(|b| b.is_ascii_graphic())
            ),
            "Invalid session token"
        );
        Ok(())
    }
}
