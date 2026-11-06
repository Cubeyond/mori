// SPDX-FileCopyrightText: 2026 Cubeyond
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Corresponding-source location without exposing operator configuration or data.

use anyhow::{Context, bail};
use std::io::ErrorKind;
use url::Url;

/// Resolve the operator override, immutable image reference, or development fallback.
pub(super) async fn load(override_url: Option<&str>) -> anyhow::Result<String> {
    if let Some(value) = override_url {
        return validate(value);
    }
    let value = match tokio::fs::read_to_string("/usr/share/licenses/mori/SOURCE").await {
        Ok(value) => value,
        Err(error) if error.kind() == ErrorKind::NotFound => {
            env!("CARGO_PKG_REPOSITORY").to_owned()
        }
        Err(error) => return Err(error).context("reading image source reference"),
    };
    validate(value.trim())
}

/// Accept only an explicit HTTP(S) source location without embedded credentials.
fn validate(value: &str) -> anyhow::Result<String> {
    let url = Url::parse(value).context("MORI_SOURCE_URL must be an absolute HTTP(S) URL")?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        bail!("MORI_SOURCE_URL must be an HTTP(S) URL without credentials");
    }
    Ok(url.into())
}
