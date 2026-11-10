//! Deadlines for the authorized application upstream and its response body.

use super::body_limits::MAX_JSON_BYTES;
use axum::{
    body::{Body, BodyDataStream, Bytes},
    http::StatusCode,
};
use futures_util::{StreamExt, stream};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{future::Future, io, time::Duration};
use tokio::{
    sync::OwnedSemaphorePermit,
    time::{Instant, sleep_until},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct AppTimeouts {
    pub(crate) first_byte_timeout_secs: u16,
    pub(crate) idle_timeout_secs: u16,
    pub(crate) non_streaming_timeout_secs: u16,
}

impl AppTimeouts {
    #[cfg(target_os = "macos")]
    pub(crate) fn validate(&self) -> anyhow::Result<()> {
        anyhow::ensure!(
            (1..=120).contains(&self.first_byte_timeout_secs)
                && self.idle_timeout_secs <= 600
                && (60..=1200).contains(&self.non_streaming_timeout_secs),
            "invalid routing timeouts"
        );
        Ok(())
    }
}

impl Default for AppTimeouts {
    fn default() -> Self {
        Self {
            first_byte_timeout_secs: 60,
            idle_timeout_secs: 120,
            non_streaming_timeout_secs: 600,
        }
    }
}

pub(super) async fn before<T>(
    deadline: Instant,
    operation: impl Future<Output = T>,
) -> Result<T, StatusCode> {
    tokio::select! {
        biased;
        () = sleep_until(deadline) => Err(StatusCode::GATEWAY_TIMEOUT),
        result = operation => Ok(result),
    }
}

pub(super) async fn stream_body(
    body: Body,
    first_byte_deadline: Instant,
    idle: Duration,
    permit: OwnedSemaphorePermit,
) -> Result<Body, StatusCode> {
    let mut stream = body.into_data_stream();
    let mut observer = OutputObserver::default();
    let mut prelude = Vec::new();
    let activity = before(first_byte_deadline, async {
        loop {
            let bytes = stream
                .next()
                .await
                .ok_or(StatusCode::BAD_GATEWAY)?
                .map_err(|_| StatusCode::BAD_GATEWAY)?;
            if bytes.len() > MAX_JSON_BYTES - prelude.len() {
                return Err(StatusCode::BAD_GATEWAY);
            }
            prelude.extend_from_slice(&bytes);
            let activity = observer.observe(&bytes).map_err(|_| StatusCode::BAD_GATEWAY)?;
            if activity != Activity::Quiet {
                return Ok(activity);
            }
        }
    })
    .await??;
    let state = TimedStream {
        prelude: Some(prelude.into()),
        upstream: (activity != Activity::End).then_some(stream),
        observer,
        idle,
        idle_deadline: (!idle.is_zero()).then(|| Instant::now() + idle),
        _permit: permit,
    };
    Ok(Body::from_stream(stream::try_unfold(state, TimedStream::next)))
}

struct TimedStream {
    prelude: Option<Bytes>,
    upstream: Option<BodyDataStream>,
    observer: OutputObserver,
    idle: Duration,
    idle_deadline: Option<Instant>,
    _permit: OwnedSemaphorePermit,
}

impl TimedStream {
    async fn next(mut self) -> io::Result<Option<(Bytes, Self)>> {
        if let Some(bytes) = self.prelude.take() {
            return Ok(Some((bytes, self)));
        }
        let Some(upstream) = self.upstream.as_mut() else {
            return Ok(None);
        };
        let chunk = match self.idle_deadline {
            Some(deadline) => before(deadline, upstream.next()).await.map_err(|_| {
                io::Error::new(io::ErrorKind::TimedOut, "upstream stream idle timeout")
            })?,
            None => upstream.next().await,
        };
        let Some(chunk) = chunk else {
            return Ok(None);
        };
        let bytes = chunk.map_err(|_| io::Error::other("upstream stream failed"))?;
        match self.observer.observe(&bytes)? {
            Activity::Output if !self.idle.is_zero() => {
                self.idle_deadline = Some(Instant::now() + self.idle);
            }
            Activity::End => self.upstream = None,
            _ => {}
        }
        Ok(Some((bytes, self)))
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Activity {
    Quiet,
    Output,
    End,
}

/// Inspect complete SSE frames without rewriting any relayed bytes. The
/// unfinished frame is bounded separately from the prelude.
#[derive(Default)]
struct OutputObserver {
    frame: Vec<u8>,
    line_start: usize,
}

impl OutputObserver {
    fn observe(&mut self, bytes: &[u8]) -> io::Result<Activity> {
        let mut activity = Activity::Quiet;
        for fragment in bytes.split_inclusive(|byte| *byte == b'\n') {
            if fragment.len() > MAX_JSON_BYTES - self.frame.len() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "upstream SSE frame too large",
                ));
            }
            self.frame.extend_from_slice(fragment);
            if fragment.last() != Some(&b'\n') {
                continue;
            }
            if matches!(self.frame[self.line_start..], [b'\n'] | [b'\r', b'\n']) {
                match frame_activity(&self.frame)? {
                    Activity::Quiet => {}
                    Activity::Output => activity = Activity::Output,
                    Activity::End => return Ok(Activity::End),
                }
                self.frame.clear();
            }
            self.line_start = self.frame.len();
        }
        Ok(activity)
    }
}

fn frame_activity(frame: &[u8]) -> io::Result<Activity> {
    let frame = std::str::from_utf8(frame).map_err(|_| {
        io::Error::new(io::ErrorKind::InvalidData, "upstream SSE frame is not UTF-8")
    })?;
    let event = frame.lines().rev().find_map(|line| sse_field(line, "event"));
    if event.is_some_and(is_metadata) {
        return Ok(Activity::Quiet);
    }

    let mut data = frame.lines().filter_map(|line| sse_field(line, "data"));
    let Some(first) = data.next() else {
        return Ok(Activity::Quiet);
    };
    let mut multiline = String::new();
    for line in data {
        if multiline.is_empty() {
            multiline.push_str(first);
        }
        multiline.push('\n');
        multiline.push_str(line);
    }
    let payload = (if multiline.is_empty() { first } else { multiline.as_str() }).trim_ascii();
    if payload == "[DONE]" {
        return Ok(Activity::End);
    }
    let Ok(value) = serde_json::from_str::<Value>(payload) else {
        return Ok(Activity::Quiet);
    };
    if value["type"].as_str().is_some_and(is_metadata) {
        return Ok(Activity::Quiet);
    }
    // A role-only or usage-only Chat chunk can precede actual model output.
    let output = value["choices"].as_array().is_some_and(|choices| {
        choices.iter().any(|choice| {
            choice["finish_reason"].is_string()
                || choice["delta"].as_object().is_some_and(|delta| {
                    delta.iter().any(|(key, field)| {
                        key != "role"
                            && !field.is_null()
                            && field.as_str().is_none_or(|text| !text.is_empty())
                            && field.as_array().is_none_or(|items| !items.is_empty())
                    })
                })
        })
    });
    Ok(if output { Activity::Output } else { Activity::Quiet })
}

fn sse_field<'a>(line: &'a str, name: &str) -> Option<&'a str> {
    let (field, value) = line.split_once(':').unwrap_or((line, ""));
    (field == name).then(|| value.strip_prefix(' ').unwrap_or(value))
}

fn is_metadata(event: &str) -> bool {
    matches!(
        event.trim_ascii(),
        "ping" | "heartbeat" | "keepalive" | "response.created" | "response.in_progress"
    )
}
