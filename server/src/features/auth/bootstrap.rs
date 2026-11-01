//! Environment-driven single-administrator provisioning without storing plaintext passwords.

use super::hash_password;
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use sqlx::SqlitePool;

/// Apply configured credentials, retaining sessions when credentials are unchanged.
///
/// # Errors
/// Returns password-worker, hashing, or database errors.
pub async fn configure(
    pool: &SqlitePool,
    username: &str,
    password: Option<String>,
) -> anyhow::Result<()> {
    let stored: Option<(String, String)> =
        sqlx::query_as("SELECT username,password_hash FROM admin WHERE id=1")
            .fetch_optional(pool)
            .await?;
    let current = stored.as_ref().map(|record| record.1.clone());
    let replacement = if let Some(password) = password {
        tokio::task::spawn_blocking(move || -> anyhow::Result<Option<String>> {
            let unchanged = current.as_ref().is_some_and(|value| {
                PasswordHash::new(value).is_ok_and(|hash| {
                    Argon2::default()
                        .verify_password(password.as_bytes(), &hash)
                        .is_ok()
                })
            });
            if unchanged {
                Ok(None)
            } else {
                Ok(Some(hash_password(&password)?))
            }
        })
        .await??
    } else {
        None
    };
    let renamed = stored.as_ref().is_some_and(|record| record.0 != username);
    if replacement.is_none() && !renamed {
        return Ok(());
    }
    let mut transaction = pool.begin().await?;
    if let Some(hash) = replacement {
        sqlx::query("INSERT INTO admin(id,username,password_hash) VALUES(1,?,?) ON CONFLICT(id) DO UPDATE SET username=excluded.username,password_hash=excluded.password_hash")
            .bind(username).bind(hash).execute(&mut *transaction).await?;
    } else {
        sqlx::query("UPDATE admin SET username=? WHERE id=1")
            .bind(username)
            .execute(&mut *transaction)
            .await?;
    }
    sqlx::query("DELETE FROM sessions")
        .execute(&mut *transaction)
        .await?;
    transaction.commit().await?;
    Ok(())
}
