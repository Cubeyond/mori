// SPDX-FileCopyrightText: 2026 Cubeyond
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Immutable object storage and bounded byte ranges for protocol acceptance tests.

use super::{
    Body, Bytes, HeaderMap, Object, Response, StatusCode, Store, header, multipart::etag, xml,
};

/// Handle object puts, internal copies, deletion, metadata, and byte reads.
pub(super) fn handle(
    store: &mut Store,
    method: &str,
    path: &str,
    headers: &HeaderMap,
    bytes: Bytes,
) -> Response {
    match method {
        "PUT" => {
            let source = header(headers, "x-amz-copy-source");
            let copied = !source.is_empty();
            let bytes = if copied {
                let source = format!("/{}", source.trim_start_matches('/').replace("%2F", "/"));
                let Some(object) = store.objects.get(&source) else {
                    return xml(StatusCode::NOT_FOUND, String::new());
                };
                object.bytes.clone()
            } else {
                bytes
            };
            let tag = etag(&bytes);
            store.objects.insert(
                path.into(),
                Object {
                    bytes,
                    mime: header(headers, "content-type").into(),
                    cache: header(headers, "cache-control").into(),
                },
            );
            let mut response = xml(
                StatusCode::OK,
                if copied {
                    format!("<CopyObjectResult><ETag>{tag}</ETag></CopyObjectResult>")
                } else {
                    String::new()
                },
            );
            response.headers_mut().insert("etag", tag.parse().unwrap());
            response
        }
        "DELETE" => {
            store.objects.remove(path);
            xml(StatusCode::NO_CONTENT, String::new())
        }
        "GET" | "HEAD" => read(store, method, path, headers),
        _ => xml(StatusCode::BAD_REQUEST, String::new()),
    }
}

/// Preserve metadata and HTTP Range semantics without using the production range parser.
fn read(store: &Store, method: &str, path: &str, headers: &HeaderMap) -> Response {
    let Some(object) = store.objects.get(path) else {
        return xml(
            if method == "HEAD" {
                StatusCode::FORBIDDEN
            } else {
                StatusCode::NOT_FOUND
            },
            "<Error><Code>NoSuchKey</Code></Error>".into(),
        );
    };
    let length = object.bytes.len();
    let mut response = Response::builder()
        .header("content-type", &object.mime)
        .header("etag", etag(&object.bytes))
        .header("accept-ranges", "bytes")
        .header("cache-control", &object.cache);
    let mut data = object.bytes.clone();
    if let Some(range) = headers.get("range") {
        let (start, end) = range
            .to_str()
            .unwrap()
            .strip_prefix("bytes=")
            .unwrap()
            .split_once('-')
            .unwrap();
        let from = if start.is_empty() {
            length.saturating_sub(end.parse().unwrap())
        } else {
            start.parse().unwrap()
        };
        let to = if end.is_empty() || start.is_empty() {
            length - 1
        } else {
            end.parse::<usize>().unwrap().min(length - 1)
        };
        if from >= length || from > to {
            return response
                .status(StatusCode::RANGE_NOT_SATISFIABLE)
                .header("content-range", format!("bytes */{length}"))
                .body(Body::empty())
                .unwrap();
        }
        response = response
            .status(StatusCode::PARTIAL_CONTENT)
            .header("content-range", format!("bytes {from}-{to}/{length}"));
        data = data.slice(from..=to);
    }
    response
        .header("content-length", data.len())
        .body(if method == "HEAD" {
            Body::empty()
        } else {
            Body::from(data)
        })
        .unwrap()
}
