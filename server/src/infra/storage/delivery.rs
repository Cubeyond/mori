//! S3-only delivery grants access to published copies without exposing private sources.

use super::Storage;
use anyhow::Result;
use aws_sdk_s3::presigning::PresigningConfig;
use std::time::Duration;

/// Validity of an individual playback session; stable Mori links issue fresh signatures.
const PLAYBACK_LIFETIME: Duration = Duration::from_hours(1);

impl Storage {
    /// Sign the requested HTTP method for a published object, never a canonical private source.
    ///
    /// Range headers remain unsigned so subsequent seeks can use the same GET signature.
    /// The caller must authorize public visibility and completed publication first.
    ///
    /// # Errors
    /// Returns signature construction errors without making an S3 network request.
    pub async fn public_url(&self, token: &str, extension: &str, head: bool) -> Result<String> {
        let key = Self::public(token, extension);
        self.published_url(key, head).await
    }

    /// Sign an already-authorized published original or derivative, never an input bucket key.
    ///
    /// # Errors
    /// Returns signature construction errors without an S3 network request.
    pub(crate) async fn published_url(&self, key: String, head: bool) -> Result<String> {
        let expires = PresigningConfig::expires_in(PLAYBACK_LIFETIME)?;
        let signed = if head {
            self.s3
                .head_object()
                .bucket(&self.bucket)
                .key(key)
                .presigned(expires)
                .await?
        } else {
            self.s3
                .get_object()
                .bucket(&self.bucket)
                .key(key)
                .response_cache_control("no-store")
                .response_content_disposition("inline")
                .presigned(expires)
                .await?
        };
        Ok(signed.uri().into())
    }

    /// Exact AWS origin that may receive public playback and poster redirects.
    ///
    /// Dotted bucket names use a regional path-style endpoint to preserve TLS validation.
    #[must_use]
    pub fn browser_sources(&self) -> String {
        let suffix = if self.region.starts_with("cn-") {
            "amazonaws.com.cn"
        } else {
            "amazonaws.com"
        };
        if self.bucket.contains('.') {
            format!("https://s3.{}.{suffix}", self.region)
        } else {
            format!("https://{}.s3.{}.{suffix}", self.bucket, self.region)
        }
    }
}
