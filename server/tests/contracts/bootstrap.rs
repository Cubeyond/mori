//! Bootstrap HTTP contracts with isolated state.

use crate::support::{lifecycle_collection, login, setup};
use axum::body::Body;
use axum::http::Request;
use http_body_util::BodyExt;
use serde_json::Value;
use tower::ServiceExt;

#[tokio::test]
/// The initial document contains authorized metadata synchronously without leaking it to visitors.
async fn document_seed_privacy_contract() {
    let (state, router, _directory) = setup().await;
    lifecycle_collection(&state).await;
    let name = "Nature </script><img src=x>";
    sqlx::query("UPDATE albums SET name=? WHERE id='collection'")
        .bind(name)
        .execute(&state.db)
        .await
        .unwrap();
    let (cookie, csrf) = login(&router).await;
    for (path, authenticated, count, visibility) in [
        ("/albums/collection", true, Some(2), "public"),
        ("/albums/collection", false, None, "public"),
        ("/nature", true, Some(2), "public"),
        ("/nature", false, Some(1), "public"),
        ("/nature", true, Some(2), "private"),
        ("/nature", false, None, "private"),
    ] {
        sqlx::query("UPDATE albums SET visibility=? WHERE id='collection'")
            .bind(visibility)
            .execute(&state.db)
            .await
            .unwrap();
        let mut request = Request::builder().uri(path);
        if authenticated {
            request = request.header("cookie", &cookie);
        }
        let response = router
            .clone()
            .oneshot(request.body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.headers()["cache-control"], "private,no-store");
        assert_eq!(response.headers()["vary"], "Cookie");
        let html = String::from_utf8(
            response
                .into_body()
                .collect()
                .await
                .unwrap()
                .to_bytes()
                .to_vec(),
        )
        .unwrap();
        assert!(!html.contains(name));
        let data = html
            .split("type=\"application/json\">")
            .nth(1)
            .unwrap()
            .split("</script>")
            .next()
            .unwrap();
        let data: Value = serde_json::from_str(data).unwrap();
        assert_eq!(data["session"]["authenticated"], authenticated);
        if authenticated {
            assert_eq!(data["session"]["csrf"], csrf);
        }
        if let Some(count) = count {
            assert_eq!(data["album_page"]["album"]["name"], name);
            assert_eq!(
                data["album_page"]["videos"].as_array().unwrap().len(),
                count
            );
        } else {
            assert!(data["album_page"].is_null());
        }
        if !authenticated {
            assert!(data["albums"].is_null());
        }
    }
}
