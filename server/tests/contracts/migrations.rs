// SPDX-FileCopyrightText: 2026 Cubeyond
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Forward-only schema evolution preserves content and immutable historical checksums.

use sqlx::{SqlitePool, migrate::Migrator};
use std::path::Path;

/// Apply an authentic historical prefix using `SQLx`'s normal checksum tracking.
async fn database_at(version: u64) -> (SqlitePool, tempfile::TempDir) {
    let directory = tempfile::tempdir().unwrap();
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("migrations");
    for entry in std::fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name();
        let name = name.to_str().unwrap();
        let sequence: u64 = name.split('_').next().unwrap().parse().unwrap();
        if sequence <= version {
            std::fs::copy(entry.path(), directory.path().join(name)).unwrap();
        }
    }
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    Migrator::new(directory.path())
        .await
        .unwrap()
        .run(&pool)
        .await
        .unwrap();
    (pool, directory)
}

#[tokio::test]
/// Existing collections stay shared while newly created collections default to private.
async fn album_visibility_upgrade_preserves_existing_collections() {
    let (pool, _directory) = database_at(2).await;
    sqlx::query(
        "INSERT INTO albums(id,name,slug,created_at) VALUES('old','Existing','existing',1)",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::migrate!().run(&pool).await.unwrap();
    sqlx::query("INSERT INTO albums(id,name,slug,created_at) VALUES('new','New','new-album',2)")
        .execute(&pool)
        .await
        .unwrap();
    let visible: String = sqlx::query_scalar("SELECT visibility FROM albums WHERE id='old'")
        .fetch_one(&pool)
        .await
        .unwrap();
    let private: String = sqlx::query_scalar("SELECT visibility FROM albums WHERE id='new'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(visible, "public");
    assert_eq!(private, "private");
}

#[tokio::test]
/// Removed legacy collections release names and retain media and their access policy.
async fn deleted_album_upgrade_retains_content() {
    let (pool, _directory) = database_at(4).await;
    sqlx::query("INSERT INTO albums(id,name,slug,created_at,deleted_at,visibility) VALUES('old','Nature','nature',1,10,'public')").execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO videos(id,token,album_id,name,visibility,state,size,created_at,media_prefix) VALUES('video','opaque-token','old','Original','public','published',100,1,'nature')").execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO media_aliases(video_id,prefix) VALUES('video','nature')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::migrate!().run(&pool).await.unwrap();
    let video: (Option<String>, String, String, String) = sqlx::query_as(
        "SELECT album_id,media_prefix,visibility,token FROM videos WHERE id='video'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        video,
        (None, "file".into(), "public".into(), "opaque-token".into())
    );
    let aliases: Vec<String> =
        sqlx::query_scalar("SELECT prefix FROM media_aliases WHERE video_id='video'")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(aliases, ["file"]);
    sqlx::query("INSERT INTO albums(id,name,slug,created_at) VALUES('replacement','Replacement','nature',2)").execute(&pool).await.unwrap();
    let retired: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM pragma_table_info('albums') WHERE name='deleted_at'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(retired, 0);
}

#[tokio::test]
/// Current startup migrations are repeatable and support the actual collection query order.
async fn current_schema_is_repeatable_and_indexed() {
    let (pool, _directory) = database_at(0).await;
    sqlx::migrate!().run(&pool).await.unwrap();
    sqlx::migrate!().run(&pool).await.unwrap();
    for (query, index) in [
        (
            "EXPLAIN QUERY PLAN SELECT * FROM videos WHERE state!='deleted' AND state NOT IN ('failed','uploading','verifying') ORDER BY created_at DESC,id DESC LIMIT 31",
            "videos_library_order",
        ),
        (
            "EXPLAIN QUERY PLAN SELECT * FROM videos WHERE album_id='album' AND visibility='public' AND state='published' ORDER BY created_at DESC,id DESC LIMIT 31",
            "videos_public_album_order",
        ),
        (
            "EXPLAIN QUERY PLAN SELECT id FROM videos WHERE album_id='album' AND poster!=0 AND state IN ('ready','published') ORDER BY created_at DESC,id DESC LIMIT 3",
            "videos_album_previews",
        ),
    ] {
        let plan: Vec<(i64, i64, i64, String)> =
            sqlx::query_as(query).fetch_all(&pool).await.unwrap();
        assert!(
            plan.iter().any(|row| row.3.contains(index)),
            "Expected index {index}"
        );
        assert!(!plan.iter().any(|row| row.3.contains("TEMP B-TREE")));
    }
    let violations: Vec<(String, i64, String, i64)> = sqlx::query_as("PRAGMA foreign_key_check")
        .fetch_all(&pool)
        .await
        .unwrap();
    assert_eq!(violations, Vec::<(String, i64, String, i64)>::new());
}

#[tokio::test]
/// Adding optional descriptions keeps existing names, addresses and visibility unchanged.
async fn album_description_upgrade_preserves_metadata() {
    let (pool, _directory) = database_at(10).await;
    sqlx::query("INSERT INTO albums(id,name,slug,visibility,created_at) VALUES('album','Nature','nature','public',1)")
        .execute(&pool).await.unwrap();
    sqlx::migrate!().run(&pool).await.unwrap();
    let metadata: (String, String, String, String) =
        sqlx::query_as("SELECT name,slug,visibility,description FROM albums WHERE id='album'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        metadata,
        (
            "Nature".into(),
            "nature".into(),
            "public".into(),
            String::new()
        )
    );
}

#[test]
/// Applied migration bytes remain immutable; upgrades must append a new version.
fn historical_checksums_are_immutable() {
    let expected = [
        (
            1,
            "065dc7b3dc491ee19d36a131004a5ec07b65708504a9797fc9c40128ab58db7d42cab7e9459b8c71f3942f90ebd13e72",
        ),
        (
            2,
            "8852365a241d3fe2d7f534110050fc5a2096b3c6dc3246e37a23fcbeb0e78d15849dfab49f43ae49ed3745521bc3651e",
        ),
        (
            3,
            "46757ff7af72c97e97ac82c4735dbf43cef5f1498b415d5fdd8c67b6b3d0cba81c70ebadccd176289a0becf87329b5bc",
        ),
        (
            4,
            "3b4f325892dd1fde74dc31c7d1647043064e1aaf3ed6a306e94dbbea021f0efa0baaec72dcf6ed41839cb98e589d2e84",
        ),
        (
            5,
            "cdc32c9e7c0ff4d0f5f360f80f217bae9a49488f38d8541625045295baab424e19320b5a0eee8487c13f10afddb25f3a",
        ),
        (
            6,
            "8213393f1227213bd05822f77f96b3520debc1f5b1acfa0e33aff4d832b429d142c3dd934fabe17677e2c06b75c348aa",
        ),
        (
            7,
            "2a801e871f6b557eca05e9f575fa41f2bfde89458d42c49394e72153999e36de75be6b63fcc3565ec0797b9c451a03b2",
        ),
        (
            8,
            "150a7ce8fd15ebe709f294605a2a911ff0213af9d8f834b0d29cc02b8d913d310b9d144ebec0405613483543f2460e53",
        ),
        (
            9,
            "56cae57e7250887732bfe7984e59a009b27c697b4023ba3c830d19362e7577575626980a369aa16f4bc9164afed94fcf",
        ),
    ];
    let migrations = sqlx::migrate!();
    for (version, checksum) in expected {
        let actual = migrations
            .iter()
            .find(|migration| migration.version == version)
            .unwrap();
        assert_eq!(
            hex::encode(&actual.checksum),
            checksum,
            "Changed migration {version}"
        );
    }
}

#[tokio::test]
/// Upgrades retain old preview pointers and enqueue durable moves without touching active uploads.
async fn preview_layout_upgrade_keeps_readable_pointers() {
    let (pool, _directory) = database_at(9).await;
    for (id, state, pointer) in [
        ("old-video", "published", None),
        ("old-image", "ready", Some("private/posters/photo.jpg")),
        (
            "current",
            "ready",
            Some("private/files/posters/current.jpg"),
        ),
        ("upload", "uploading", None),
    ] {
        sqlx::query("INSERT INTO videos(id,token,name,state,visibility,size,created_at,source_key,poster_key) VALUES(?,?,?, ?,'private',100,1,'private/files/original.mp4',?)")
            .bind(id).bind(id).bind(id).bind(state).bind(pointer).execute(&pool).await.unwrap();
    }
    sqlx::migrate!().run(&pool).await.unwrap();
    let jobs: Vec<String> = sqlx::query_scalar(
        "SELECT video_id FROM jobs WHERE kind='normalize' AND state='pending' ORDER BY video_id",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(jobs, ["old-image", "old-video"]);
    let pointer: Option<String> =
        sqlx::query_scalar("SELECT poster_key FROM videos WHERE id='old-image'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(pointer.as_deref(), Some("private/posters/photo.jpg"));
}
