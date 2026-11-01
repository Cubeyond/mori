//! Verify the destination before callers release any source object.

use super::Storage;
use anyhow::{Result, ensure};

impl Storage {
    /// Copy once, or recover a verified destination after an interrupted relocation.
    ///
    /// # Errors
    /// Returns transport, permission, missing-source, or destination-integrity errors.
    pub async fn copy_verified(
        &self,
        source: String,
        destination: String,
        mime: &str,
        size: Option<i64>,
    ) -> Result<()> {
        let head = self
            .s3
            .head_object()
            .bucket(&self.bucket)
            .key(&destination)
            .send()
            .await;
        if let Ok(object) = head {
            ensure!(
                size.is_none_or(|size| object.content_length == Some(size)),
                "Relocation destination has an unexpected length"
            );
            return Ok(());
        }
        // Without unrestricted ListBucket, S3 returns 403 for missing keys. Copying is safe;
        // the mandatory successful HEAD below still prevents deleting an unverified source.
        self.copy(source, destination.clone(), mime).await?;
        let stored = self
            .s3
            .head_object()
            .bucket(&self.bucket)
            .key(destination)
            .send()
            .await?;
        ensure!(
            size.is_none_or(|size| stored.content_length == Some(size)),
            "Relocation destination has an unexpected length"
        );
        Ok(())
    }
}
