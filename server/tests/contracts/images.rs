// SPDX-FileCopyrightText: 2026 Cubeyond
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Mixed media shares one upload, authorization, publication, and deletion lifecycle.

use crate::support::{Client, media::Media, wait_job};
use axum::http::StatusCode;
use http_body_util::BodyExt;
use mori::features::videos::{self, MediaKind};
use serde_json::{Value, json};

#[tokio::test]
/// A JPEG original and its JPEG preview never share an object key in either visibility.
async fn jpeg_original_survives_preview_publication_and_withdrawal() {
    let media = Media::new(false).await;
    let bytes = include_bytes!("../fixtures/image.jpg");
    let upload = media.begin("private", bytes.len(), "jpg").await;
    media.part(&upload, bytes).await;
    media.poster(upload["video_id"].as_str().unwrap()).await;
    let job = media.complete(&upload).await;
    assert_eq!(wait_job(&media.state, &job).await, "complete");
    let mut image = videos::get(&media.state, upload["video_id"].as_str().unwrap())
        .await
        .unwrap();
    assert_eq!(image.kind, MediaKind::Image);
    assert_ne!(image.private_source(), image.private_poster());
    for visibility in ["public", "private", "public"] {
        image = media.visibility(&image, visibility).await;
        let content = media
            .content(&format!("/v1/admin/videos/{}/content", image.id), None)
            .await;
        assert_eq!(content.headers()["content-type"], "image/jpeg");
        assert!(
            content
                .headers()
                .get_all("content-security-policy")
                .iter()
                .any(|value| value.to_str().unwrap().starts_with("sandbox;"))
        );
        assert_eq!(
            content
                .into_body()
                .collect()
                .await
                .unwrap()
                .to_bytes()
                .as_ref(),
            bytes
        );
        if visibility == "public" {
            // Existing least-privilege IAM and OAC grants cover only public/files/*.
            assert!(image.public_poster().starts_with("public/files/"));
            assert_ne!(
                image.public_poster(),
                mori::infra::storage::Storage::public(&image.token, "jpg")
            );
            assert!(!media.exists(image.private_source()).await);
            let visitor = Client::visitor(&media.router);
            let preview = visitor
                .request(
                    "HEAD",
                    &format!("/posters/{}.jpg", image.token),
                    Value::Null,
                )
                .await;
            assert_eq!(preview.status, StatusCode::OK);
            assert!(preview.headers.get("location").is_none());
        } else {
            assert!(!media.exists(image.public_poster()).await);
        }
    }
}

#[tokio::test]
/// Unsupported image decoders do not reject original files, and album queries retain both categories.
async fn albums_mix_image_and_video_without_transcoding() {
    let media = Media::new(false).await;
    let album = media
        .request(
            "POST",
            "/v1/admin/albums",
            json!({"name":"Mixed","slug":"mixed","visibility":"public"}),
        )
        .await
        .body;
    let video = media.ready().await;
    let bytes = include_bytes!("../fixtures/image.jpg");
    let upload = media.begin("private", bytes.len(), "dng").await;
    media.part(&upload, bytes).await;
    let job = media.complete(&upload).await;
    assert_eq!(wait_job(&media.state, &job).await, "complete");
    let image = videos::get(&media.state, upload["video_id"].as_str().unwrap())
        .await
        .unwrap();
    for item in [video, image] {
        let response = media
            .request(
                "PATCH",
                &format!("/v1/admin/videos/{}", item.id),
                json!({"name":item.name,"album_id":album["id"],"visibility":"public"}),
            )
            .await;
        assert_eq!(
            wait_job(&media.state, response.body["job_id"].as_str().unwrap()).await,
            "complete"
        );
    }
    let page = Client::visitor(&media.router)
        .request("GET", "/v1/albums/mixed", Value::Null)
        .await;
    let kinds: std::collections::BTreeSet<&str> = page.body["videos"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["kind"].as_str().unwrap())
        .collect();
    assert_eq!(kinds, std::collections::BTreeSet::from(["image", "video"]));
}

#[tokio::test]
/// Opening an authenticated SVG directly cannot execute scripts with the application's origin.
async fn private_svg_has_an_independent_sandbox_policy() {
    let media = Media::new(false).await;
    let bytes = b"<svg xmlns='http://www.w3.org/2000/svg'><script>alert(1)</script></svg>";
    let upload = media.begin("private", bytes.len(), "svg").await;
    media.part(&upload, bytes).await;
    let job = media.complete(&upload).await;
    assert_eq!(wait_job(&media.state, &job).await, "complete");
    let response = media
        .content(
            &format!(
                "/v1/admin/videos/{}/content",
                upload["video_id"].as_str().unwrap()
            ),
            None,
        )
        .await;
    assert_eq!(response.headers()["content-type"], "image/svg+xml");
    assert!(
        response.headers()["content-disposition"]
            .to_str()
            .unwrap()
            .starts_with("attachment;")
    );
    assert!(
        response
            .headers()
            .get_all("content-security-policy")
            .iter()
            .any(|value| value.to_str().unwrap().starts_with("sandbox;"))
    );
}
