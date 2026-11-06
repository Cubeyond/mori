// SPDX-FileCopyrightText: 2026 Cubeyond
// SPDX-License-Identifier: AGPL-3.0-or-later

//! S3 multipart operations, publication copies, and optional CloudFront clients.

/// Verified, restart-safe object relocation within the configured bucket.
mod relocation;

use crate::{app::Config, infra::storage_settings::config::StorageSettings};
use anyhow::{Context, Result, ensure};
use aws_sdk_s3::operation::abort_multipart_upload::AbortMultipartUploadError;
use aws_sdk_s3::{
    Client,
    config::{BehaviorVersion, Credentials, Region},
    presigning::PresigningConfig,
    types::{CompletedMultipartUpload, CompletedPart},
};
use std::time::Duration;

/// Permanent multipart integrity failure that must not enter transport retry loops.
#[derive(Debug)]
pub struct InvalidUpload(
    /// Permanent validation failure, distinguished from retryable transport errors.
    pub &'static str,
);
impl std::fmt::Display for InvalidUpload {
    /// Render this validation failure without leaking credentials or signed URLs.
    ///
    /// # Errors
    /// Returns a formatting error if the destination writer rejects the message.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}
impl std::error::Error for InvalidUpload {}

/// Reusable AWS clients and the canonical media-key conventions.
#[derive(Clone)]
pub struct Storage {
    /// Reusable S3 SDK client with bounded request timeouts.
    pub s3: Client,
    /// Reusable `CloudFront` SDK client for invalidation coordination.
    pub cf: Option<aws_sdk_cloudfront::Client>,
    /// Private S3 bucket containing canonical and published objects.
    pub bucket: String,
    /// AWS region used for browser upload origins and the active connection.
    pub region: String,
    /// Optional CDN distribution coordinating confirmed edge withdrawal.
    pub distribution_id: Option<String>,
    /// Non-secret suffix identifying the saved access key in administration.
    pub key_hint: String,
    /// Whether the encrypted configuration contains a temporary session token.
    pub has_session_token: bool,
}
impl Storage {
    /// Create reusable AWS clients with explicit credentials and bounded timeouts.
    ///
    /// # Errors
    /// Returns invalid bucket, region, or credential values before building any client.
    pub fn new(settings: &StorageSettings, config: &Config) -> Result<Self> {
        settings.validate()?;
        let credentials = Credentials::new(
            settings.access_key_id.clone(),
            settings.secret_access_key.clone(),
            settings.session_token.clone(),
            None,
            "mori-settings",
        );
        let mut builder = aws_sdk_s3::config::Builder::new()
            .behavior_version(BehaviorVersion::latest())
            .region(Region::new(settings.region.clone()))
            .credentials_provider(credentials.clone())
            .timeout_config(
                aws_sdk_s3::config::timeout::TimeoutConfig::builder()
                    .operation_timeout(Duration::from_secs(90))
                    .operation_attempt_timeout(Duration::from_secs(60))
                    .connect_timeout(Duration::from_secs(5))
                    .read_timeout(Duration::from_secs(30))
                    .build(),
            );
        if let Some(endpoint) = &config.s3_endpoint {
            builder = builder.endpoint_url(endpoint).force_path_style(true);
        }
        let cf = settings.distribution_id.as_ref().map(|_| {
            let mut cf_builder = aws_sdk_cloudfront::config::Builder::new()
                .behavior_version(BehaviorVersion::latest())
                .region(Region::new("us-east-1"))
                .credentials_provider(credentials)
                .timeout_config(
                    aws_sdk_cloudfront::config::timeout::TimeoutConfig::builder()
                        .operation_timeout(Duration::from_secs(30))
                        .connect_timeout(Duration::from_secs(5))
                        .build(),
                );
            if let Some(endpoint) = &config.cloudfront_endpoint {
                cf_builder = cf_builder.endpoint_url(endpoint);
            }
            aws_sdk_cloudfront::Client::from_conf(cf_builder.build())
        });
        Ok(Self {
            s3: Client::from_conf(builder.build()),
            cf,
            bucket: settings.bucket.clone(),
            region: settings.region.clone(),
            distribution_id: settings.distribution_id.clone(),
            key_hint: settings
                .access_key_id
                .chars()
                .rev()
                .take(4)
                .collect::<String>()
                .chars()
                .rev()
                .collect(),
            has_session_token: settings.session_token.is_some(),
        })
    }
    /// Verify required object write/read/delete permissions using a dedicated random zero-byte object.
    ///
    /// # Errors
    /// Returns connection or permission failure; cleanup is attempted after a successful write.
    pub async fn probe(&self) -> Result<()> {
        let key = format!("private/setup-check/{}", crate::app::random_id());
        self.s3
            .put_object()
            .bucket(&self.bucket)
            .key(&key)
            .body(aws_sdk_s3::primitives::ByteStream::from_static(b""))
            .send()
            .await?;
        let checked = self
            .s3
            .head_object()
            .bucket(&self.bucket)
            .key(&key)
            .send()
            .await;
        let cleaned = self.delete(key).await;
        checked?;
        cleaned?;
        Ok(())
    }
    /// Retain the legacy source key while old records and multipart sessions are upgraded.
    #[must_use]
    pub fn source(id: &str) -> String {
        format!("private/{id}/source.mp4")
    }
    /// Retain the legacy poster key until the matching source has been upgraded.
    #[must_use]
    pub fn poster(id: &str) -> String {
        format!("private/{id}/poster.jpg")
    }
    /// Build a private video key with the same immutable filename as its public location.
    #[must_use]
    pub fn private(token: &str, extension: &str) -> String {
        format!("private/files/{token}.{extension}")
    }
    /// Build a private poster key with the same immutable filename as its public location.
    #[must_use]
    pub fn private_poster(token: &str) -> String {
        format!("private/files/{token}.jpg")
    }
    /// Store derivatives separately so a JPEG original can never overwrite its own preview.
    #[must_use]
    pub fn private_preview(token: &str) -> String {
        format!("private/files/posters/{token}.jpg")
    }
    /// Keep derivatives distinct from originals within the existing public IAM/OAC prefix.
    #[must_use]
    pub fn published_preview(token: &str) -> String {
        format!("public/files/posters/{token}.jpg")
    }
    /// Build the published media key from the immutable sharing token.
    #[must_use]
    pub fn public(token: &str, extension: &str) -> String {
        format!("public/files/{token}.{extension}")
    }
    /// Build the public poster key from an immutable sharing token.
    #[must_use]
    pub fn public_poster(token: &str) -> String {
        format!("public/files/{token}.jpg")
    }
    /// Initiate a private checksummed S3 multipart operation.
    ///
    /// # Errors
    /// Returns an SDK, transport, or object-integrity error; partial operations remain retryable.
    pub async fn begin(&self, key: &str) -> Result<String> {
        self.s3
            .create_multipart_upload()
            .bucket(&self.bucket)
            .key(key)
            .content_type("application/octet-stream")
            .checksum_algorithm(aws_sdk_s3::types::ChecksumAlgorithm::Sha256)
            .send()
            .await?
            .upload_id
            .context("missing upload id")
    }
    /// Issue a short-lived signature for one checksummed multipart part.
    ///
    /// # Errors
    /// Returns an SDK, transport, or object-integrity error; partial operations remain retryable.
    pub async fn part(&self, key: &str, upload: &str, part: i32, checksum: &str) -> Result<String> {
        Ok(self
            .s3
            .upload_part()
            .bucket(&self.bucket)
            .key(key)
            .upload_id(upload)
            .part_number(part)
            .checksum_sha256(checksum)
            .presigned(PresigningConfig::expires_in(Duration::from_mins(15))?)
            .await?
            .uri()
            .to_owned())
    }
    /// Abort an incomplete S3 upload, treating an absent upload as already aborted.
    ///
    /// # Errors
    /// Returns an SDK, transport, or object-integrity error; partial operations remain retryable.
    pub async fn abort(&self, key: &str, upload: &str) -> Result<()> {
        match self
            .s3
            .abort_multipart_upload()
            .bucket(&self.bucket)
            .key(key)
            .upload_id(upload)
            .send()
            .await
        {
            Ok(_) => Ok(()),
            Err(e)
                if e.as_service_error()
                    .is_some_and(AbortMultipartUploadError::is_no_such_upload) =>
            {
                Ok(())
            }
            Err(e) => Err(e.into()),
        }
    }
    /// Verify consecutive part sizes and total length before completing the S3 object.
    ///
    /// # Errors
    /// Returns an SDK, transport, or object-integrity error; partial operations remain retryable.
    pub async fn complete(
        &self,
        key: &str,
        upload: &str,
        expected: i64,
        part_size: i64,
    ) -> Result<()> {
        let mut marker = None;
        let mut parts = Vec::new();
        let mut total = 0;
        loop {
            let response = self
                .s3
                .list_parts()
                .bucket(&self.bucket)
                .key(key)
                .upload_id(upload)
                .set_part_number_marker(marker)
                .send()
                .await?;
            for part in response.parts() {
                let number = part.part_number.context("missing part number")?;
                ensure!(
                    number == i32::try_from(parts.len())? + 1,
                    InvalidUpload("parts must be consecutive")
                );
                let size = part.size.unwrap_or(0);
                total += size;
                let final_part = total == expected;
                ensure!(
                    total <= expected && size > 0 && (size == part_size || final_part),
                    InvalidUpload("incorrect part size")
                );
                parts.push(
                    CompletedPart::builder()
                        .part_number(number)
                        .set_e_tag(part.e_tag.clone())
                        .set_checksum_sha256(part.checksum_sha256.clone())
                        .build(),
                );
            }
            if !response.is_truncated.unwrap_or(false) {
                break;
            }
            marker = response.next_part_number_marker;
            ensure!(marker.is_some(), "missing pagination marker");
        }
        ensure!(
            total == expected && !parts.is_empty(),
            InvalidUpload("upload size mismatch")
        );
        self.s3
            .complete_multipart_upload()
            .bucket(&self.bucket)
            .key(key)
            .upload_id(upload)
            .multipart_upload(
                CompletedMultipartUpload::builder()
                    .set_parts(Some(parts))
                    .build(),
            )
            .send()
            .await?;
        let object = self
            .s3
            .head_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await?;
        ensure!(
            object.content_length == Some(expected),
            InvalidUpload("completed size mismatch")
        );
        Ok(())
    }
    /// Idempotently remove one object using its canonical bucket key.
    ///
    /// # Errors
    /// Returns an SDK, transport, or object-integrity error; partial operations remain retryable.
    pub async fn delete(&self, key: String) -> Result<()> {
        self.s3
            .delete_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await?;
        Ok(())
    }
    /// Copy a canonical object inside S3 with explicit delivery metadata.
    ///
    /// # Errors
    /// Returns an SDK, transport, or object-integrity error; partial operations remain retryable.
    pub async fn copy(&self, source: String, dest: String, mime: &str) -> Result<()> {
        self.s3
            .copy_object()
            .bucket(&self.bucket)
            .key(dest)
            .copy_source(format!("{}/{}", self.bucket, source))
            .metadata_directive(aws_sdk_s3::types::MetadataDirective::Replace)
            .content_type(mime)
            .cache_control(if self.distribution_id.is_some() {
                "public,max-age=0,s-maxage=60,must-revalidate"
            } else {
                "no-store"
            })
            .send()
            .await?;
        Ok(())
    }
}
