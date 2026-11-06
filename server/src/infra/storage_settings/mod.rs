// SPDX-FileCopyrightText: 2026 Cubeyond
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Persistent encrypted storage settings and atomic runtime client replacement.

/// Internal secret configuration and target validation.
pub mod config;
/// Owner-only encryption key and authenticated configuration envelopes.
pub mod vault;

use crate::{app::Config, infra::storage::Storage};
use anyhow::Result;
use arc_swap::ArcSwapOption;
use config::StorageSettings;
use sqlx::{SqlitePool, sqlite::SqliteConnectOptions};
use std::{str::FromStr, sync::Arc};
use tokio::sync::{OwnedRwLockReadGuard, OwnedRwLockWriteGuard, RwLock};
use vault::Vault;
use zeroize::Zeroizing;

/// A fresh installation can serve administration before S3 is configured.
#[derive(Debug)]
pub struct Unconfigured;
impl std::fmt::Display for Unconfigured {
    /// Render a safe setup instruction without storage credentials.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Configure Storage before uploading or playing private media")
    }
}
impl std::error::Error for Unconfigured {}

/// Shared client snapshot and mutation gate; media operations never hold a lock per byte.
#[derive(Clone)]
pub struct StorageManager {
    /// One current validated SDK client, absent until setup is complete.
    current: Arc<ArcSwapOption<Storage>>,
    /// Coordinate configuration commits with media control operations and durable jobs.
    gate: Arc<RwLock<()>>,
    /// Independent encryption key retained only on the server.
    vault: Arc<Vault>,
}
impl StorageManager {
    /// Assemble the manager from an existing client or an unconfigured installation.
    #[must_use]
    pub fn new(client: Option<Storage>, vault: Vault) -> Self {
        Self {
            current: Arc::new(ArcSwapOption::from(client.map(Arc::new))),
            gate: Arc::new(RwLock::new(())),
            vault: Arc::new(vault),
        }
    }
    /// Load the saved encrypted configuration, or start unconfigured.
    ///
    /// # Errors
    /// Returns database, encryption-key, credential-file, or invalid persisted-configuration errors.
    pub async fn load(pool: &SqlitePool, config: &Config) -> Result<Self> {
        let bytes: Option<Vec<u8>> =
            sqlx::query_scalar("SELECT ciphertext FROM storage_settings WHERE id=1")
                .fetch_optional(pool)
                .await?;
        let options = SqliteConnectOptions::from_str(&config.database_url)?;
        let parent = options
            .get_filename()
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."));
        let vault = Vault::open(&parent.join("storage.key"), bytes.is_some())?;
        let settings = if let Some(bytes) = bytes {
            Some(serde_json::from_slice::<StorageSettings>(
                &vault.open_bytes(&bytes)?,
            )?)
        } else {
            None
        };
        let client = settings
            .as_ref()
            .map(|settings| Storage::new(settings, config))
            .transpose()?;
        Ok(Self::new(client, vault))
    }
    /// Return the current cheap client snapshot without exposing any secret configuration.
    #[must_use]
    pub fn current(&self) -> Option<Arc<Storage>> {
        self.current.load_full()
    }
    /// Require a configured client for an actual storage operation.
    ///
    /// # Errors
    /// Returns an explicit setup requirement on a fresh installation.
    pub fn get(&self) -> std::result::Result<Arc<Storage>, Unconfigured> {
        self.current().ok_or(Unconfigured)
    }
    /// Hold a shared gate across one media control operation or background task.
    pub async fn operation(&self) -> OwnedRwLockReadGuard<()> {
        self.gate.clone().read_owned().await
    }
    /// Serialize settings replacement after existing media control requests finish.
    pub async fn exclusive(&self) -> OwnedRwLockWriteGuard<()> {
        self.gate.clone().write_owned().await
    }
    /// Activate a fully validated client synchronously after durable persistence succeeds.
    pub fn install(&self, storage: Storage) {
        self.current.store(Some(Arc::new(storage)));
    }
    /// Authenticate and decode persisted credentials only inside storage administration.
    ///
    /// # Errors
    /// Returns missing-key, authentication, malformed data, or database errors.
    pub async fn stored(&self, pool: &SqlitePool) -> Result<Option<StorageSettings>> {
        let bytes: Option<Vec<u8>> =
            sqlx::query_scalar("SELECT ciphertext FROM storage_settings WHERE id=1")
                .fetch_optional(pool)
                .await?;
        bytes
            .map(|bytes| Ok(serde_json::from_slice(&self.vault.open_bytes(&bytes)?)?))
            .transpose()
    }
    /// Persist authenticated ciphertext without writing credentials to logs or plaintext SQLite.
    ///
    /// # Errors
    /// Returns serialization, encryption, or database errors; the active client stays unchanged.
    pub async fn persist(&self, pool: &SqlitePool, settings: &StorageSettings) -> Result<()> {
        let plaintext = Zeroizing::new(serde_json::to_vec(settings)?);
        let bytes = self.vault.seal(&plaintext)?;
        sqlx::query("INSERT INTO storage_settings(id,ciphertext,updated_at) VALUES(1,?,?) ON CONFLICT(id) DO UPDATE SET ciphertext=excluded.ciphertext,updated_at=excluded.updated_at")
            .bind(bytes).bind(crate::app::now()).execute(pool).await?;
        Ok(())
    }
}
