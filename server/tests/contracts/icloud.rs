// SPDX-FileCopyrightText: 2026 Cubeyond
// SPDX-License-Identifier: AGPL-3.0-or-later

//! iCloud control and additive import through the real HTTP and S3 upload contracts.

use crate::support::{Client, media::Media};
use axum::http::StatusCode;
use mori::{app::valid_token, features::icloud, http};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::UnixListener,
};

/// A read-only source distinct from the destination's real S3 protocol fixture.
async fn provider(listener: UnixListener, root: PathBuf, downloads: Arc<AtomicUsize>) {
    let bytes = include_bytes!("../fixtures/image.jpg");
    let digest = hex::encode(Sha256::digest(bytes));
    loop {
        let (mut socket, _) = listener.accept().await.unwrap();
        let length = usize::try_from(socket.read_u32().await.unwrap()).unwrap();
        let mut buffer = vec![0; length];
        socket.read_exact(&mut buffer).await.unwrap();
        let request: Value = serde_json::from_slice(&buffer).unwrap();
        let response = match request["op"].as_str().unwrap() {
            "start" => {
                assert_eq!(request["password"], "apple-password-only-for-test");
                json!({"state":"challenge","id":request["challenge"],"kind":"code","phones":[],"retry":false})
            }
            "answer" => {
                assert_eq!(request["answer"], "123456");
                json!({"state":"connected","account":"owner@example.com","session":{"cookies":"test-cookies","trust_token":"test-token"}})
            }
            "albums" => {
                json!({"value":[{"path":"PrimarySync/Trips","name":"Trips"}],"session":null})
            }
            "files" => {
                json!({"value":{"assets":[{"path":"IMG_1234.jpg","size":bytes.len(),"modified":"2026-10-31T00:00:00Z"}],"more":false},"session":null})
            }
            "prepare" => {
                downloads.fetch_add(1, Ordering::SeqCst);
                assert_eq!(request["optimization"], "quality");
                let path = root.join("stage").join(request["stage"].as_str().unwrap());
                tokio::fs::create_dir_all(&path).await.unwrap();
                tokio::fs::write(path.join("source"), bytes).await.unwrap();
                json!({"value":{"file":"source","source_digest":digest,"payload_digest":digest,"extension":"jpg","size":bytes.len(),"width":32,"height":24,"duration":0,"poster":false},"session":null})
            }
            "remove" => {
                let path = root.join("stage").join(request["stage"].as_str().unwrap());
                tokio::fs::remove_dir_all(path).await.unwrap();
                json!({})
            }
            "next" => {
                if request["frequency"] == "weekly" {
                    assert_eq!(request["weekday"], "saturday");
                    assert_eq!(request["time"], "03:00");
                    assert_eq!(request["timezone"], "Asia/Shanghai");
                }
                json!(mori::app::now() + 3600)
            }
            "forget" => json!({}),
            _ => panic!("unexpected worker request"),
        };
        let reply = serde_json::to_vec(&json!({"result":response})).unwrap();
        socket
            .write_u32(u32::try_from(reply.len()).unwrap())
            .await
            .unwrap();
        socket.write_all(&reply).await.unwrap();
    }
}

/// Wait for the same persisted status seen by the settings page.
async fn completed(admin: &Client<'_>, top: &str) -> Value {
    for _ in 0..120 {
        let state = admin.request("GET", top, Value::Null).await.body;
        if state["activity"] == "idle" && state["last_sync"].is_number() {
            return state;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("iCloud sync did not settle");
}

/// Import into an existing album, verify S3 naming, then rerun with an empty spool.
#[tokio::test]
async fn icloud_import_uses_existing_album_and_durable_deduplication() {
    let media = Media::new(false).await;
    let folder = tempfile::tempdir().unwrap();
    let mut state = media.state.clone();
    Arc::make_mut(&mut state.config).icloud_dir = Some(folder.path().to_path_buf());
    let listener = UnixListener::bind(folder.path().join("worker.sock")).unwrap();
    let downloads = Arc::new(AtomicUsize::new(0));
    let provider = tokio::spawn(provider(
        listener,
        folder.path().to_path_buf(),
        downloads.clone(),
    ));
    let worker = tokio::spawn(icloud::run(state.clone()));
    let router = http::router(state.clone());
    let top = "/v1/admin/icloud";
    let connect = "/v1/admin/icloud/connect";
    let albums = "/v1/admin/icloud/albums";
    let sync = "/v1/admin/icloud/sync";
    let admin = bind_account(&state, &router, top, connect, albums).await;
    let album = admin
        .request(
            "POST",
            "/v1/admin/albums",
            json!({
                "name":"Destination in Mori","slug":"destination","visibility":"private"
            }),
        )
        .await;
    assert_eq!(album.status, StatusCode::CREATED);
    let id = album.body["id"].as_str().unwrap();
    assert_eq!(
        admin
            .request(
                "PUT",
                albums,
                json!({"albums":[{"source":"PrimarySync/Trips","album_id":id}]})
            )
            .await
            .status,
        StatusCode::NO_CONTENT
    );

    let first = completed(&admin, top).await;
    assert_eq!(first["imported"], 1, "{first}");
    let (name, key, album_id): (String, String, String) =
        sqlx::query_as("SELECT name,source_key,album_id FROM videos WHERE state='ready'")
            .fetch_one(&state.db)
            .await
            .unwrap();
    assert_eq!(album_id, id);
    assert!(valid_token(&name));
    assert!(key.starts_with(&format!("private/imports/icloud/{id}/")));
    assert!(!key.contains("IMG_1234"));
    assert_eq!(
        std::fs::read_dir(Path::new(folder.path()).join("stage"))
            .unwrap()
            .count(),
        0
    );
    assert_eq!(
        admin.request("POST", sync, Value::Null).await.status,
        StatusCode::ACCEPTED
    );
    assert_eq!(completed(&admin, top).await["skipped"], 1);
    worker.abort();
    let _ = worker.await;
    verify_schedule(&admin, &state, top).await;
    assert_eq!(downloads.load(Ordering::SeqCst), 1);

    assert_eq!(
        admin.request("DELETE", connect, Value::Null).await.status,
        StatusCode::NO_CONTENT
    );
    assert!(admin.request("GET", top, Value::Null).await.body["account"].is_null());
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM videos WHERE state='ready'")
        .fetch_one(&state.db)
        .await
        .unwrap();
    assert_eq!(count, 1);
    provider.abort();
}

/// A due persisted schedule is consumed by the worker without a manual sync request.
async fn verify_schedule(admin: &Client<'_>, state: &mori::app::AppState, top: &str) {
    let settings = format!("{top}/schedule");
    let mut rule = json!({"automatic":true,"frequency":"weekly","local_time":"03:00","timezone":"Asia/Shanghai","optimization":null});
    assert_eq!(
        admin.request("PUT", &settings, rule.clone()).await.status,
        StatusCode::BAD_REQUEST
    );
    rule["weekday"] = json!("saturday");
    assert_eq!(
        admin.request("PUT", &settings, rule.clone()).await.status,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        admin.request("GET", top, Value::Null).await.body["schedule"],
        rule
    );
    let idle_at = mori::app::now();
    sqlx::query("UPDATE icloud_schedule SET next_sync=?,queued_at=?,last_sync=NULL WHERE id=1")
        .bind(idle_at)
        .bind(idle_at + 300)
        .execute(&state.db)
        .await
        .unwrap();
    let scheduler = tokio::spawn(icloud::run(state.clone()));
    let result = completed(admin, top).await;
    scheduler.abort();
    assert_eq!(result["skipped"], 1);
    assert!(result["next_sync"].as_i64().unwrap() > idle_at);
}

/// Exercise authorization and exchange a fake Apple challenge through the public routes.
async fn bind_account<'a>(
    state: &mori::app::AppState,
    router: &'a axum::Router,
    top: &str,
    connect: &str,
    albums: &str,
) -> Client<'a> {
    assert_eq!(
        Client::visitor(router)
            .request("GET", top, Value::Null)
            .await
            .status,
        StatusCode::UNAUTHORIZED
    );
    let admin = Client::admin(router).await;
    let initial = admin.request("GET", top, Value::Null).await.body;
    assert_eq!(initial["schedule"]["automatic"], false);
    assert_eq!(initial["schedule"]["frequency"], "weekly");
    assert_eq!(initial["schedule"]["weekday"], "sunday");
    let challenge = admin.request("POST", connect, json!({
        "action":"start","account":"owner@example.com","password":"apple-password-only-for-test","upload_optimization":"quality"
    })).await;
    assert_eq!(challenge.status, StatusCode::OK);
    assert_eq!(challenge.body["state"], "challenge");
    assert!(challenge.body.get("session").is_none());
    let connected = admin
        .request(
            "POST",
            connect,
            json!({
                "action":"answer","challenge":challenge.body["id"],"answer":"123456"
            }),
        )
        .await;
    assert_eq!(connected.body, json!({"state":"connected"}));
    let credentials: Vec<u8> = sqlx::query_scalar("SELECT credentials FROM icloud_connection")
        .fetch_one(&state.db)
        .await
        .unwrap();
    assert!(!String::from_utf8_lossy(&credentials).contains("test-token"));
    assert_eq!(
        admin.request("GET", albums, Value::Null).await.body,
        json!([{"path":"PrimarySync/Trips","name":"Trips"}])
    );

    admin
}
