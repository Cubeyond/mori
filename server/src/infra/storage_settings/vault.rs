// SPDX-FileCopyrightText: 2026 Cubeyond
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Authenticated encryption for storage configuration, with an owner-only local key.

use anyhow::{Context, Result, ensure};
use chacha20poly1305::{
    XChaCha20Poly1305,
    aead::{Aead, AeadCore, KeyInit, Payload},
};
use std::{fs::OpenOptions, io::Write, path::Path};
use zeroize::Zeroizing;

/// In-memory cipher initialized from the metadata volume's independent encryption key.
pub struct Vault {
    /// Audited AEAD primitive; keys and nonces are never derived from the login password.
    cipher: XChaCha20Poly1305,
}
impl Vault {
    /// Open the existing key, creating it only for a configuration without encrypted data.
    ///
    /// # Errors
    /// Returns inaccessible/missing keys, invalid key length, or filesystem failures.
    pub fn open(path: &Path, existing: bool) -> Result<Self> {
        if !path.exists() {
            ensure!(
                !existing,
                "Storage encryption key is missing; restore storage.key from backup"
            );
            let key = XChaCha20Poly1305::generate_key(&mut rand::rngs::OsRng);
            let mut options = OpenOptions::new();
            options.create_new(true).write(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            let mut file = options
                .open(path)
                .context("creating storage encryption key")?;
            file.write_all(&key)?;
            file.sync_all()?;
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
        }
        let key = Zeroizing::new(std::fs::read(path).context("reading storage encryption key")?);
        let cipher = XChaCha20Poly1305::new_from_slice(&key)
            .map_err(|_| anyhow::anyhow!("Invalid storage encryption key"))?;
        Ok(Self { cipher })
    }
    /// Encrypt configuration bytes with an independent random nonce and versioned context.
    ///
    /// # Errors
    /// Returns an authenticated-encryption error without exposing the plaintext.
    pub fn seal(&self, plaintext: &[u8]) -> Result<Vec<u8>> {
        let nonce = XChaCha20Poly1305::generate_nonce(&mut rand::rngs::OsRng);
        let encrypted = self
            .cipher
            .encrypt(
                &nonce,
                Payload {
                    msg: plaintext,
                    aad: b"mori-storage-v1",
                },
            )
            .map_err(|_| anyhow::anyhow!("Could not encrypt storage settings"))?;
        let mut result = vec![1];
        result.extend_from_slice(&nonce);
        result.extend(encrypted);
        Ok(result)
    }
    /// Authenticate stored ciphertext before releasing any decoded configuration bytes.
    ///
    /// # Errors
    /// Returns invalid format, wrong key, or tampered ciphertext errors.
    pub fn open_bytes(&self, bytes: &[u8]) -> Result<Zeroizing<Vec<u8>>> {
        ensure!(
            bytes.len() > 25 && bytes[0] == 1,
            "Invalid encrypted storage settings"
        );
        let nonce: &[u8; 24] = bytes[1..25].try_into()?;
        let plaintext = self
            .cipher
            .decrypt(
                nonce.into(),
                Payload {
                    msg: &bytes[25..],
                    aad: b"mori-storage-v1",
                },
            )
            .map_err(|_| {
                anyhow::anyhow!(
                    "Could not decrypt storage settings; restore the correct storage.key"
                )
            })?;
        Ok(Zeroizing::new(plaintext))
    }
}
