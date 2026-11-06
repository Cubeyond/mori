// SPDX-FileCopyrightText: 2026 Cubeyond
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Minimal checksum-aware multipart fixture using the same SDK surface as production.

use super::{Bytes, HeaderMap, Object, Response, StatusCode, Store, Upload, header, xml};
use base64::{Engine, engine::general_purpose::STANDARD};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap};

/// Execute one multipart action against the isolated fixture store.
pub(super) fn handle(
    store: &mut Store,
    method: &str,
    path: &str,
    query: &HashMap<String, String>,
    headers: &HeaderMap,
    bytes: Bytes,
) -> Response {
    if query.contains_key("uploads") {
        let id = mori::app::random_id();
        store.uploads.insert(
            id.clone(),
            Upload {
                path: path.into(),
                mime: header(headers, "content-type").into(),
                parts: BTreeMap::new(),
            },
        );
        return xml(
            StatusCode::OK,
            format!(
                "<InitiateMultipartUploadResult><Bucket>test-bucket</Bucket><Key>{path}</Key><UploadId>{id}</UploadId></InitiateMultipartUploadResult>"
            ),
        );
    }
    let id = &query["uploadId"];
    let Some(upload) = store.uploads.get_mut(id) else {
        return xml(
            StatusCode::NOT_FOUND,
            "<Error><Code>NoSuchUpload</Code></Error>".into(),
        );
    };
    match method {
        "PUT" => {
            let checksum = STANDARD.encode(Sha256::digest(&bytes));
            let supplied = header(headers, "x-amz-checksum-sha256");
            let supplied = if supplied.is_empty() {
                query
                    .get("x-amz-checksum-sha256")
                    .map_or("", String::as_str)
            } else {
                supplied
            };
            if !supplied.is_empty() && checksum != supplied {
                return xml(
                    StatusCode::BAD_REQUEST,
                    "<Error><Code>BadDigest</Code></Error>".into(),
                );
            }
            let etag = etag(&bytes);
            upload
                .parts
                .insert(query["partNumber"].parse().unwrap(), bytes);
            let mut result = xml(StatusCode::OK, String::new());
            result.headers_mut().insert("etag", etag.parse().unwrap());
            result
                .headers_mut()
                .insert("x-amz-checksum-sha256", checksum.parse().unwrap());
            result
        }
        "GET" => parts(upload),
        "POST" => {
            let upload = store.uploads.remove(id).unwrap();
            let bytes = upload
                .parts
                .values()
                .flat_map(|part| part.iter().copied())
                .collect::<Vec<_>>()
                .into();
            store.objects.insert(
                upload.path,
                Object {
                    bytes,
                    mime: upload.mime,
                    cache: String::new(),
                },
            );
            xml(StatusCode::OK,"<CompleteMultipartUploadResult><Bucket>test-bucket</Bucket><Key>source</Key><ETag>\"complete\"</ETag></CompleteMultipartUploadResult>".into())
        }
        "DELETE" => {
            store.uploads.remove(id);
            xml(StatusCode::NO_CONTENT, String::new())
        }
        _ => xml(StatusCode::BAD_REQUEST, String::new()),
    }
}

/// Render verified part metadata; `ETags` remain opaque rather than promising a content MD5.
fn parts(upload: &Upload) -> Response {
    use std::fmt::Write;
    let mut rows = String::new();
    for (number, bytes) in &upload.parts {
        write!(&mut rows,"<Part><PartNumber>{number}</PartNumber><ETag>{}</ETag><Size>{}</Size><ChecksumSHA256>{}</ChecksumSHA256></Part>",etag(bytes),bytes.len(),STANDARD.encode(Sha256::digest(bytes))).unwrap();
    }
    xml(
        StatusCode::OK,
        format!(
            "<ListPartsResult><Bucket>test-bucket</Bucket><Key>{}</Key><UploadId>fixture</UploadId><IsTruncated>false</IsTruncated>{rows}</ListPartsResult>",
            upload.path
        ),
    )
}

/// Produce a stable opaque object validator, independent of multipart `ETag` conventions.
pub(super) fn etag(bytes: &[u8]) -> String {
    format!("\"{}\"", hex::encode(Sha256::digest(bytes)))
}
