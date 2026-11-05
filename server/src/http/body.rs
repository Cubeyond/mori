//! Bound control-body lifetime without interrupting durable storage mutations or media streams.

use axum::body::Body;
use futures_util::{StreamExt, stream};
use std::{io, time::Duration};
use tokio::time::{Instant, timeout_at};

/// Apply one fixed deadline to the complete incoming body, including trickled frames.
pub(super) fn bounded(body: Body, duration: Duration) -> Body {
    let incoming = body.into_data_stream();
    let deadline = Instant::now() + duration;
    Body::from_stream(stream::unfold(Some((incoming, deadline)), |state| async {
        let (mut incoming, deadline) = state?;
        match timeout_at(deadline, incoming.next()).await {
            Ok(Some(frame)) => Some((frame.map_err(io::Error::other), Some((incoming, deadline)))),
            Ok(None) => None,
            Err(_) => Some((
                Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "Request body deadline exceeded",
                )),
                None,
            )),
        }
    }))
}

#[cfg(test)]
/// Control bodies cannot keep a request slot occupied by sending occasional bytes.
mod tests {
    use super::{Body, Duration, StreamExt, bounded, stream};

    #[tokio::test]
    /// Even a continuously progressing body must finish within its total read deadline.
    async fn trickled_body_has_a_total_deadline() {
        let frames =
            stream::repeat_with(|| Ok::<_, std::io::Error>(axum::body::Bytes::from_static(b"x")))
                .then(|frame| async {
                    tokio::time::sleep(Duration::from_millis(2)).await;
                    frame
                });
        let body = bounded(Body::from_stream(frames), Duration::from_millis(20));
        let result =
            tokio::time::timeout(Duration::from_secs(1), axum::body::to_bytes(body, 1024)).await;
        assert!(matches!(result, Ok(Err(_))));
    }
}
