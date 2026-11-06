// SPDX-FileCopyrightText: 2026 Cubeyond
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Object lifecycle contracts preserve bytes across retries and historical layout upgrades.

use crate::support::{media::Media, wait_job};
use mori::{app::random_id, features::videos, infra::storage::Storage};
use serde_json::Value;

#[tokio::test]
/// An interrupted publish resumes from its verified destination after the source was released.
async fn relocation_recovers_after_source_deletion() {
    let media = Media::new(false).await;
    let video = media.ready().await;
    let storage = media.state.storage.get().unwrap();
    storage
        .copy_verified(
            video.private_source(),
            Storage::public(&video.token, &video.extension),
            "video/webm",
            Some(video.size),
        )
        .await
        .unwrap();
    storage.delete(video.private_source()).await.unwrap();
    let published = media.visibility(&video, "public").await;
    assert_eq!(published.state, "published");
    storage
        .copy_verified(
            Storage::public(&video.token, &video.extension),
            video.private_source(),
            "video/webm",
            Some(video.size),
        )
        .await
        .unwrap();
    storage
        .delete(Storage::public(&video.token, &video.extension))
        .await
        .unwrap();
    let private = media.visibility(&published, "private").await;
    assert_eq!(private.state, "ready");
    assert!(media.exists(private.private_source()).await);
}

#[tokio::test]
/// A destination with the wrong length never authorizes releasing a valid source.
async fn relocation_rejects_an_unverified_destination() {
    let media = Media::new(false).await;
    let video = media.ready().await;
    let storage = media.state.storage.get().unwrap();
    assert!(
        storage
            .copy_verified(
                video.private_source(),
                Storage::public(&video.token, &video.extension),
                "video/webm",
                Some(video.size + 1)
            )
            .await
            .is_err()
    );
    assert!(media.exists(video.private_source()).await);
}

#[tokio::test]
/// Legacy private sources and published duplicates are normalized without changing share tokens.
async fn legacy_object_layout_is_upgraded_without_losing_media() {
    let media = Media::new(false).await;
    let storage = media.state.storage.get().unwrap();
    for public in [false, true] {
        let video = media.ready().await;
        storage
            .copy(
                video.private_source(),
                Storage::source(&video.id),
                "video/webm",
            )
            .await
            .unwrap();
        storage
            .copy(
                video.private_poster(),
                Storage::poster(&video.id),
                "image/jpeg",
            )
            .await
            .unwrap();
        storage.delete(video.private_source()).await.unwrap();
        storage.delete(video.private_poster()).await.unwrap();
        if public {
            storage
                .copy(
                    Storage::source(&video.id),
                    Storage::public(&video.token, &video.extension),
                    "video/webm",
                )
                .await
                .unwrap();
            storage
                .copy(
                    Storage::poster(&video.id),
                    Storage::public_poster(&video.token),
                    "image/jpeg",
                )
                .await
                .unwrap();
        }
        sqlx::query(
            "UPDATE videos SET source_key=NULL,poster_key=NULL,state=?,visibility=? WHERE id=?",
        )
        .bind(if public { "published" } else { "ready" })
        .bind(if public { "public" } else { "private" })
        .bind(&video.id)
        .execute(&media.state.db)
        .await
        .unwrap();
        let job = random_id();
        sqlx::query(
            "INSERT INTO jobs(id,video_id,kind,revision,created_at) VALUES(?,?,'normalize',0,1)",
        )
        .bind(&job)
        .bind(&video.id)
        .execute(&media.state.db)
        .await
        .unwrap();
        media.state.job_wakeup.notify_one();
        assert_eq!(wait_job(&media.state, &job).await, "complete");
        let normalized = videos::get(&media.state, &video.id).await.unwrap();
        assert_eq!(
            normalized.source_key.as_ref(),
            Some(&video.private_source())
        );
        assert_eq!(normalized.token, video.token);
        assert_eq!(
            normalized.private_poster(),
            Storage::private_preview(&video.token)
        );
        assert_eq!(
            normalized.public_poster(),
            Storage::published_preview(&video.token)
        );
        assert_eq!(media.exists(normalized.private_poster()).await, !public);
        assert_eq!(media.exists(normalized.public_poster()).await, public);
        assert!(!media.exists(Storage::public_poster(&video.token)).await);
        assert!(!media.exists(Storage::source(&video.id)).await);
        assert!(!media.exists(Storage::poster(&video.id)).await);
        assert_eq!(media.exists(normalized.private_source()).await, !public);
        assert_eq!(
            media
                .exists(Storage::public(&video.token, &video.extension))
                .await,
            public
        );
        let page = media.request("GET", "/v1/admin/videos", Value::Null).await;
        assert!(
            page.body["videos"]
                .as_array()
                .unwrap()
                .iter()
                .any(|entry| entry["id"] == video.id)
        );
    }
}

#[tokio::test]
/// Moving an old image preview never removes an original with the same JPEG filename.
async fn old_image_preview_moves_without_deleting_jpeg_original() {
    let media = Media::new(false).await;
    let bytes = include_bytes!("../fixtures/image.jpg");
    let upload = media.begin("private", bytes.len(), "jpg").await;
    media.part(&upload, bytes).await;
    media.poster(upload["video_id"].as_str().unwrap()).await;
    let job = media.complete(&upload).await;
    assert_eq!(wait_job(&media.state, &job).await, "complete");
    let image = videos::get(&media.state, upload["video_id"].as_str().unwrap())
        .await
        .unwrap();
    let storage = media.state.storage.get().unwrap();
    let old = format!("private/posters/{}.jpg", image.token);
    storage
        .copy(image.private_poster(), old.clone(), "image/jpeg")
        .await
        .unwrap();
    storage.delete(image.private_poster()).await.unwrap();
    sqlx::query("UPDATE videos SET poster_key=? WHERE id=?")
        .bind(&old)
        .bind(&image.id)
        .execute(&media.state.db)
        .await
        .unwrap();
    let job = random_id();
    sqlx::query(
        "INSERT INTO jobs(id,video_id,kind,revision,created_at) VALUES(?,?,'normalize',0,1)",
    )
    .bind(&job)
    .bind(&image.id)
    .execute(&media.state.db)
    .await
    .unwrap();
    for _ in 0..2 {
        sqlx::query("UPDATE jobs SET state='pending' WHERE id=?")
            .bind(&job)
            .execute(&media.state.db)
            .await
            .unwrap();
        media.state.job_wakeup.notify_one();
        assert_eq!(wait_job(&media.state, &job).await, "complete");
        assert!(media.exists(image.private_source()).await);
        assert!(media.exists(Storage::private_preview(&image.token)).await);
        assert!(!media.exists(old.clone()).await);
    }
    let content = media
        .content(&format!("/v1/admin/videos/{}/content", image.id), None)
        .await;
    assert_eq!(
        http_body_util::BodyExt::collect(content.into_body())
            .await
            .unwrap()
            .to_bytes()
            .as_ref(),
        bytes
    );
}
