//! Per-test S3 and CloudFront protocol fixture; never a storage provider or IAM emulator.

/// Checksummed multipart lifecycle responses.
mod multipart;
/// Object reads, metadata, copies, and deletes.
mod objects;

use axum::{
    Router,
    body::{Body, Bytes},
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    response::Response,
};
use std::{
    collections::{BTreeMap, HashMap},
    sync::{Arc, Mutex},
};
use tokio::{sync::Notify, task::JoinHandle};

/// Running loopback fixture whose listener is bound before its endpoint becomes visible.
pub struct Fixture {
    /// URL used by the AWS SDK, never an actual AWS endpoint.
    pub endpoint: String,
    /// Server task cancelled when this fixture leaves scope.
    task: JoinHandle<()>,
    /// Shared state permits deterministic ordering of otherwise concurrent requests.
    store: Arc<Mutex<Store>>,
}
impl Fixture {
    /// Start independent protocol state without subprocesses or port-allocation races.
    pub async fn start() -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!("http://{}", listener.local_addr().unwrap());
        let store = Arc::new(Mutex::new(Store::default()));
        let router = Router::new().fallback(handle).with_state(store.clone());
        let task = tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap();
        });
        Self {
            endpoint,
            task,
            store,
        }
    }
}
impl Fixture {
    /// Pause one object PUT before it reaches the in-memory store.
    pub fn pause_put(&self) -> PausedPut {
        let pause = PausedPut {
            entered: Arc::new(Notify::new()),
            resume: Arc::new(Notify::new()),
        };
        self.store.lock().unwrap().pause = Some(pause.clone());
        pause
    }
}

/// One-shot protocol barrier for checking lifecycle races without network sleeps.
#[derive(Clone)]
pub struct PausedPut {
    /// Signals the exact point before object creation.
    entered: Arc<Notify>,
    /// Releases the pending write.
    resume: Arc<Notify>,
}
impl PausedPut {
    /// Wait until the targeted request has reached the fixture.
    pub async fn entered(&self) {
        tokio::time::timeout(std::time::Duration::from_secs(3), self.entered.notified())
            .await
            .unwrap();
    }
    /// Let the object PUT finish.
    pub fn release(&self) {
        self.resume.notify_one();
    }
}

impl Drop for Fixture {
    /// Release the listener and all fixture objects deterministically.
    fn drop(&mut self) {
        self.task.abort();
    }
}

/// Fully isolated state for this fixture only.
#[derive(Default)]
struct Store {
    /// Optional next-object-write barrier, removed before awaiting it.
    pause: Option<PausedPut>,
    /// Stored source and published copies, indexed by request path.
    objects: HashMap<String, Object>,
    /// Active multipart sessions.
    uploads: HashMap<String, Upload>,
    /// Number of progress polls per submitted invalidation.
    invalidations: HashMap<String, usize>,
}
/// Opaque object bytes and relevant response metadata.
#[derive(Clone)]
struct Object {
    /// Shared immutable fixture payload.
    bytes: Bytes,
    /// Media type assigned at upload or publication.
    mime: String,
    /// Cache policy assigned by the production operation.
    cache: String,
}
/// Ordered parts belonging to one source key.
struct Upload {
    /// Final S3 object path, including the bucket prefix.
    path: String,
    /// Source content type.
    mime: String,
    /// Confirmed part bytes.
    parts: BTreeMap<i32, Bytes>,
}

/// Dispatch a bounded protocol request without retaining a lock across an await.
async fn handle(State(store): State<Arc<Mutex<Store>>>, request: Request) -> Response {
    let (parts, body) = request.into_parts();
    let bytes = axum::body::to_bytes(body, 32 * 1024 * 1024).await.unwrap();
    let bytes = if parts
        .headers
        .get("content-encoding")
        .is_some_and(|v| v.as_bytes() == b"aws-chunked")
    {
        decode_chunks(&bytes)
    } else {
        bytes
    };
    let pause = if parts.method == "PUT"
        && parts
            .uri
            .query()
            .is_none_or(|query| !query.contains("uploadId="))
    {
        store.lock().unwrap().pause.take()
    } else {
        None
    };
    if let Some(pause) = pause {
        pause.entered.notify_one();
        pause.resume.notified().await;
    }
    let mut state = store.lock().unwrap();
    let path = parts.uri.path();
    let query: HashMap<String, String> =
        url::form_urlencoded::parse(parts.uri.query().unwrap_or_default().as_bytes())
            .into_owned()
            .collect();
    let method = parts.method.as_str();
    if path.starts_with("/2020-05-31/distribution/") {
        return invalidation(&mut state, method, path);
    }
    if query.contains_key("uploads") || query.contains_key("uploadId") {
        multipart::handle(&mut state, method, path, &query, &parts.headers, bytes)
    } else {
        objects::handle(&mut state, method, path, &parts.headers, bytes)
    }
}

/// Decode the AWS SDK's streaming HTTP chunk framing, ignoring trailer metadata.
fn decode_chunks(input: &[u8]) -> Bytes {
    let mut rest = input;
    let mut output = Vec::new();
    while !rest.is_empty() {
        let end = rest.windows(2).position(|v| v == b"\r\n").unwrap();
        let line = std::str::from_utf8(&rest[..end])
            .unwrap()
            .split(';')
            .next()
            .unwrap();
        let length = usize::from_str_radix(line, 16).unwrap();
        if length == 0 {
            break;
        }
        rest = &rest[end + 2..];
        output.extend_from_slice(&rest[..length]);
        rest = &rest[length + 2..];
    }
    output.into()
}

/// Emit an XML protocol response; opaque fixture identifiers need no XML escaping.
fn xml(status: StatusCode, body: String) -> Response {
    Response::builder()
        .status(status)
        .header("content-type", "application/xml")
        .body(Body::from(body))
        .unwrap()
}
/// Model asynchronous edge completion while preserving worker restart assertions.
fn invalidation(store: &mut Store, method: &str, path: &str) -> Response {
    let (id, progress) = if method == "POST" {
        let id = mori::app::random_id();
        store.invalidations.insert(id.clone(), 0);
        (id, 0)
    } else {
        let id = path.rsplit('/').next().unwrap().to_owned();
        let polls = store.invalidations.entry(id.clone()).or_default();
        *polls += 1;
        (id, *polls)
    };
    let status = if progress >= 2 {
        "Completed"
    } else {
        "InProgress"
    };
    xml(
        if method == "POST" {
            StatusCode::CREATED
        } else {
            StatusCode::OK
        },
        format!(
            "<Invalidation><Id>{id}</Id><Status>{status}</Status><CreateTime>2026-10-30T00:00:00Z</CreateTime><InvalidationBatch><CallerReference>fixture</CallerReference><Paths><Quantity>0</Quantity></Paths></InvalidationBatch></Invalidation>"
        ),
    )
}

/// Read an optional protocol header without returning credential-bearing debug output.
fn header<'a>(headers: &'a HeaderMap, name: &str) -> &'a str {
    headers
        .get(name)
        .map_or("", |value| value.to_str().unwrap())
}
