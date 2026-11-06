// SPDX-FileCopyrightText: 2026 Cubeyond
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Storage HTTP contracts with isolated state.

use crate::support::{Client, call, login, s3_fixture, setup};
use axum::Router;
use axum::http::StatusCode;
use mori::app::AppState;
use mori::http;
use mori::infra::storage_settings::StorageManager;
use mori::infra::storage_settings::vault::Vault;
use serde_json::Value;
use serde_json::json;
use std::sync::Arc;

#[test]
/// Reject authenticated-encryption tampering, wrong keys, and missing recovery keys.
fn storage_vault_authenticates_and_protects_key_files() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("storage.key");
    let vault = Vault::open(&path, false).unwrap();
    let message = b"storage-credentials-not-plaintext";
    let first = vault.seal(message).unwrap();
    let second = vault.seal(message).unwrap();
    assert_ne!(first, second);
    assert_eq!(vault.open_bytes(&first).unwrap().as_slice(), message);
    let mut damaged = first.clone();
    damaged[26] ^= 1;
    assert!(vault.open_bytes(&damaged).is_err());
    let other = Vault::open(&dir.path().join("other.key"), false).unwrap();
    assert!(other.open_bytes(&first).is_err());
    assert!(Vault::open(&dir.path().join("missing.key"), true).is_err());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
}

#[tokio::test]
/// Start without any AWS configuration and reject media operations before creating records.
async fn unconfigured_storage_keeps_administration_available() {
    let (initial, _, _dir) = setup().await;
    let config = (*initial.config).clone();
    let state = AppState::new(config).await.unwrap();
    assert!(state.storage.current().is_none());
    let router = http::router(state.clone());
    let (cookie, csrf) = login(&router).await;
    let (_, _, view) = call(
        &router,
        "GET",
        "/v1/admin/storage",
        Some(&cookie),
        None,
        Value::Null,
    )
    .await;
    assert_eq!(view["configured"], false);
    assert_eq!(view["mode"], "unconfigured");
    assert_eq!(
        call(&router, "GET", "/v1/admin/storage", None, None, Value::Null)
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        call(
            &router,
            "POST",
            "/v1/admin/uploads",
            Some(&cookie),
            Some(&csrf),
            json!({"size": 100, "visibility":"private"})
        )
        .await
        .0,
        StatusCode::SERVICE_UNAVAILABLE
    );
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM videos")
        .fetch_one(&state.db)
        .await
        .unwrap();
    assert_eq!(count, 0);
    assert_eq!(
        call(
            &router,
            "GET",
            "/v1/admin/albums",
            Some(&cookie),
            None,
            Value::Null
        )
        .await
        .0,
        StatusCode::OK
    );
}

/// Arrange a fresh unconfigured client against the independent protocol fixture.
async fn storage_setup() -> (AppState, Router, tempfile::TempDir, crate::support::Fixture) {
    let (initial, _, directory) = setup().await;
    let (fixture, endpoint) = s3_fixture().await;
    let mut config = (*initial.config).clone();
    config.s3_endpoint = Some(endpoint.clone());
    config.cloudfront_endpoint = Some(endpoint);
    let state = AppState::new(config).await.unwrap();
    let router = http::router(state.clone());
    (state, router, directory, fixture)
}
/// Fake credentials exercise encrypted persistence without contacting AWS.
fn settings_input() -> Value {
    json!({"bucket":"test-bucket","region":"us-east-1","distribution_id":"TESTDIST","access_key_id":"AWSKEY1234","secret_access_key":"very-private-secret","session_token":"very-private-session"})
}

#[tokio::test]
/// Tests require CSRF, never persist candidates, and reject arbitrary endpoint inputs.
async fn storage_probe_does_not_save_or_allow_endpoint_injection() {
    let (state, router, _directory, _fixture) = storage_setup().await;
    let admin = Client::admin(&router).await;
    let (cookie, _) = login(&router).await;
    assert_eq!(
        call(
            &router,
            "PUT",
            "/v1/admin/storage",
            Some(&cookie),
            None,
            settings_input()
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    let checked = admin
        .request("POST", "/v1/admin/storage/test", settings_input())
        .await;
    assert_eq!(checked.status, StatusCode::OK);
    assert_eq!(checked.body["verified"], true);
    assert_eq!(checked.body["mode"], "test");
    assert!(state.storage.current().is_none());
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM storage_settings")
        .fetch_one(&state.db)
        .await
        .unwrap();
    assert_eq!(count, 0);
    let mut injected = settings_input();
    injected["endpoint"] = json!("http://169.254.169.254/");
    assert_eq!(
        admin
            .request("PUT", "/v1/admin/storage", injected)
            .await
            .status,
        StatusCode::UNPROCESSABLE_ENTITY
    );
}

#[tokio::test]
/// Saving redacts response secrets and restores encrypted credentials after reloading.
async fn storage_credentials_are_encrypted_and_reloadable() {
    let (state, router, _directory, _fixture) = storage_setup().await;
    let admin = Client::admin(&router).await;
    let saved = admin
        .request("PUT", "/v1/admin/storage", settings_input())
        .await;
    assert_eq!(saved.status, StatusCode::OK);
    assert_eq!(saved.body["access_key_hint"], "1234");
    assert_eq!(saved.body["has_session_token"], true);
    for secret in ["AWSKEY1234", "very-private-secret", "very-private-session"] {
        assert!(!saved.body.to_string().contains(secret));
    }
    let encrypted: Vec<u8> =
        sqlx::query_scalar("SELECT ciphertext FROM storage_settings WHERE id=1")
            .fetch_one(&state.db)
            .await
            .unwrap();
    assert!(!String::from_utf8_lossy(&encrypted).contains("very-private-secret"));
    let reloaded = StorageManager::load(&state.db, &state.config)
        .await
        .unwrap();
    assert_eq!(
        reloaded.get().unwrap().distribution_id.as_deref(),
        Some("TESTDIST")
    );
    let stored = reloaded.stored(&state.db).await.unwrap().unwrap();
    assert_eq!(stored.secret_access_key, "very-private-secret");
    assert_eq!(
        stored.session_token.as_deref(),
        Some("very-private-session")
    );
}

#[tokio::test]
/// Configuration changes serialize with media operations, while credentials can rotate under a location lock.
async fn storage_gate_and_location_lock_preserve_rotation() {
    let (state, router, _directory, _fixture) = storage_setup().await;
    let admin = Client::admin(&router).await;
    assert_eq!(
        admin
            .request("PUT", "/v1/admin/storage", settings_input())
            .await
            .status,
        StatusCode::OK
    );
    let retained =
        json!({"bucket":"test-bucket","region":"us-east-1","distribution_id":"TESTDIST"});
    let guard = state.storage.operation().await;
    let mut saving = Box::pin(admin.request("PUT", "/v1/admin/storage", retained.clone()));
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(100), &mut saving)
            .await
            .is_err()
    );
    drop(guard);
    assert_eq!(saving.await.status, StatusCode::OK);
    sqlx::query("INSERT INTO videos(id,token,name,visibility,state,size,created_at) VALUES('locked','abcdef','Sample','private','ready',100,1)").execute(&state.db).await.unwrap();
    let mut moved = retained.clone();
    moved["bucket"] = json!("different-bucket");
    assert_eq!(
        admin
            .request("PUT", "/v1/admin/storage", moved)
            .await
            .status,
        StatusCode::CONFLICT
    );
    let mut rotated = retained;
    rotated["access_key_id"] = json!("AWSKEY5678");
    rotated["secret_access_key"] = json!("rotated-private-secret");
    rotated["session_token"] = json!("");
    let saved = admin.request("PUT", "/v1/admin/storage", rotated).await;
    assert_eq!(saved.status, StatusCode::OK);
    assert_eq!(saved.body["location_locked"], true);
    assert_eq!(saved.body["access_key_hint"], "5678");
    assert_eq!(saved.body["has_session_token"], false);
    let reloaded = StorageManager::load(&state.db, &state.config)
        .await
        .unwrap();
    assert_eq!(
        reloaded
            .stored(&state.db)
            .await
            .unwrap()
            .unwrap()
            .secret_access_key,
        "rotated-private-secret"
    );
}

#[tokio::test]
/// Restoring encrypted settings requires their matching recovery key.
async fn storage_reload_rejects_a_missing_key() {
    let (state, router, directory, _fixture) = storage_setup().await;
    let admin = Client::admin(&router).await;
    assert_eq!(
        admin
            .request("PUT", "/v1/admin/storage", settings_input())
            .await
            .status,
        StatusCode::OK
    );
    std::fs::rename(
        directory.path().join("storage.key"),
        directory.path().join("storage.key.backup"),
    )
    .unwrap();
    assert!(
        StorageManager::load(&state.db, &state.config)
            .await
            .is_err()
    );
}

#[tokio::test]
/// Preserve the failed S3 operation in setup feedback without exposing provider messages or credentials.
async fn storage_probe_permission_feedback_is_actionable_and_redacted() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let service = Router::new().fallback(|| async {
        (
            StatusCode::FORBIDDEN,
            [("content-type", "application/xml")],
            "<Error><Code>AccessDenied</Code><Message>PRIVATE_PROVIDER_DETAILS</Message></Error>",
        )
    });
    let fixture = tokio::spawn(async move { axum::serve(listener, service).await.unwrap() });
    let (mut state, _, _dir) = setup().await;
    let mut config = (*state.config).clone();
    config.s3_endpoint = Some(format!("http://{address}"));
    state.config = Arc::new(config);
    let router = http::router(state);
    let (cookie, csrf) = login(&router).await;
    let (status, _, body) = call(
        &router, "POST", "/v1/admin/storage/test", Some(&cookie), Some(&csrf),
        json!({"bucket":"test-bucket","region":"us-east-1","access_key_id":"TESTKEY1234","secret_access_key":"PRIVATE_INPUT_SECRET"}),
    ).await;
    fixture.abort();
    assert_eq!(status, StatusCode::BAD_GATEWAY);
    let message = body["error"].as_str().unwrap();
    assert!(
        message.contains("s3:PutObject"),
        "Missing failed operation: {message}"
    );
    assert!(!message.contains("PRIVATE_PROVIDER_DETAILS"));
    assert!(!message.contains("PRIVATE_INPUT_SECRET"));
    assert!(!message.contains("TESTKEY1234"));
}
