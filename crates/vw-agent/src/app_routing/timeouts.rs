//! Deadlines for the authorized application upstream and its response body.

use super::{
    body_limits::MAX_JSON_BYTES,
    capacity::RequestPermit,
    protocol_errors::{Protocol, RouteError},
};
use axum::{
    body::{Body, BodyDataStream, Bytes},
    http::StatusCode,
};
use futures_util::{StreamExt, stream};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{convert::Infallible, future::Future, time::Duration};
use tokio::time::{Instant, sleep_until};

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
    permit: RequestPermit,
    protocol: Protocol,
) -> Result<Body, RouteError> {
    let mut stream = body.into_data_stream();
    let mut observer = OutputObserver {
        messages: matches!(protocol, Protocol::Messages).then(|| {
            crate::model_relay::stream_observer::StreamObserver::new(
                crate::model_relay::generation::GenerationProtocol::Messages,
            )
        }),
        ..Default::default()
    };
    let prelude = first_output(&mut stream, &mut observer, first_byte_deadline, protocol).await?;
    let state = TimedStream {
        prelude: Some(prelude.bytes.into()),
        upstream: (prelude.activity != Activity::End).then_some(stream),
        observer,
        protocol,
        idle,
        idle_deadline: (!idle.is_zero()).then(|| Instant::now() + idle),
        _permit: permit,
    };
    Ok(Body::from_stream(stream::unfold(state, TimedStream::next)))
}

/// Share the production event classifier with the failover retry owner, which
/// must report a stalled first event before releasing its circuit admission.
#[cfg(target_os = "macos")]
pub(super) async fn first_output_body(
    body: Body,
    deadline: Instant,
    protocol: Protocol,
) -> Result<Body, RouteError> {
    let mut stream = body.into_data_stream();
    let mut observer = OutputObserver {
        messages: matches!(protocol, Protocol::Messages).then(|| {
            crate::model_relay::stream_observer::StreamObserver::new(
                crate::model_relay::generation::GenerationProtocol::Messages,
            )
        }),
        ..Default::default()
    };
    let prelude = first_output(&mut stream, &mut observer, deadline, protocol).await?;
    let mut bytes = prelude.bytes;
    if prelude.activity != Activity::End {
        // The last read can also contain the beginning of the next frame.
        // Its decoder state is rebuilt downstream, so preserve these raw bytes.
        bytes.extend_from_slice(&observer.frame);
    }
    let bytes = Bytes::from(bytes);
    let first = stream::once(async move { Ok::<_, axum::Error>(bytes) });
    Ok(if prelude.activity == Activity::End {
        Body::from_stream(first)
    } else {
        Body::from_stream(first.chain(stream))
    })
}

async fn first_output(
    stream: &mut BodyDataStream,
    observer: &mut OutputObserver,
    first_byte_deadline: Instant,
    protocol: Protocol,
) -> Result<OutputBatch, RouteError> {
    let mut prelude = OutputBatch::default();
    let mut remaining = MAX_JSON_BYTES;
    before(first_byte_deadline, async {
        loop {
            let bytes = stream.next().await.and_then(Result::ok).ok_or(RouteError::StreamFailed)?;
            if bytes.len() > remaining {
                return Err(RouteError::StreamFailed);
            }
            remaining -= bytes.len();
            if let Err(error) = observer.observe(&bytes, &mut prelude) {
                if prelude.activity == Activity::Quiet {
                    return Err(error);
                }
                // Output and the error may arrive together in one TCP read.
                prelude.append_terminal(error, protocol);
            }
            if prelude.activity != Activity::Quiet {
                return Ok(());
            }
        }
    })
    .await??;
    Ok(prelude)
}

struct TimedStream {
    prelude: Option<Bytes>,
    upstream: Option<BodyDataStream>,
    observer: OutputObserver,
    protocol: Protocol,
    idle: Duration,
    idle_deadline: Option<Instant>,
    _permit: RequestPermit,
}

impl TimedStream {
    async fn next(mut self) -> Option<(Result<Bytes, Infallible>, Self)> {
        if let Some(bytes) = self.prelude.take() {
            return Some((Ok(bytes), self));
        }
        loop {
            let upstream = self.upstream.as_mut()?;
            let chunk = match self.idle_deadline {
                Some(deadline) => before(deadline, upstream.next()).await,
                None => Ok(upstream.next().await),
            };
            let mut batch = OutputBatch::default();
            let result = match chunk {
                Ok(Some(Ok(bytes))) => self.observer.observe(&bytes, &mut batch),
                Ok(_) => Err(RouteError::StreamFailed),
                Err(status) => Err(status.into()),
            };
            if let Err(error) = result {
                batch.append_terminal(error, self.protocol);
            }
            match batch.activity {
                Activity::Output if !self.idle.is_zero() => {
                    self.idle_deadline = Some(Instant::now() + self.idle);
                }
                Activity::End => self.upstream = None,
                _ => {}
            }
            if !batch.bytes.is_empty() {
                return Some((Ok(batch.bytes.into()), self));
            }
        }
    }
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum Activity {
    #[default]
    Quiet,
    Output,
    End,
}

#[derive(Default)]
struct OutputBatch {
    bytes: Vec<u8>,
    activity: Activity,
}

impl OutputBatch {
    fn append_terminal(&mut self, error: RouteError, protocol: Protocol) {
        self.bytes.extend_from_slice(protocol.terminal(error, 0).as_bytes());
        self.activity = Activity::End;
    }
}

/// Hold incomplete frames until they can be classified. Only complete success
/// and metadata frames are relayed verbatim, in their original order.
#[derive(Default)]
struct OutputObserver {
    messages: Option<crate::model_relay::stream_observer::StreamObserver>,
    frame: Vec<u8>,
    line_start: usize,
}

impl OutputObserver {
    fn observe(&mut self, bytes: &[u8], batch: &mut OutputBatch) -> Result<(), RouteError> {
        for fragment in bytes.split_inclusive(|byte| *byte == b'\n') {
            if fragment.len() > MAX_JSON_BYTES - self.frame.len() {
                return Err(RouteError::StreamFailed);
            }
            self.frame.extend_from_slice(fragment);
            if fragment.last() != Some(&b'\n') {
                continue;
            }
            if matches!(self.frame[self.line_start..], [b'\n'] | [b'\r', b'\n']) {
                let activity = match &mut self.messages {
                    Some(observer) => messages_activity(&self.frame, observer)?,
                    None => frame_activity(&self.frame)?,
                };
                batch.bytes.extend_from_slice(&self.frame);
                self.frame.clear();
                if activity != Activity::Quiet {
                    batch.activity = activity;
                }
                if activity == Activity::End {
                    return Ok(());
                }
            }
            self.line_start = self.frame.len();
        }
        Ok(())
    }
}

fn messages_activity(
    frame: &[u8],
    observer: &mut crate::model_relay::stream_observer::StreamObserver,
) -> Result<Activity, RouteError> {
    let update = observer.push(frame).map_err(|_| RouteError::StreamFailed)?;
    if update.completed {
        return Ok(Activity::End);
    }
    let frame = std::str::from_utf8(frame).map_err(|_| RouteError::StreamFailed)?;
    let payload =
        frame.lines().filter_map(|line| sse_field(line, "data")).collect::<Vec<_>>().join("\n");
    if payload.is_empty() {
        return Ok(Activity::Quiet);
    }
    let value: Value = serde_json::from_str(&payload).map_err(|_| RouteError::StreamFailed)?;
    let output = update.text_events > 0
        || match value["type"].as_str() {
            Some("content_block_start") => {
                value["content_block"]["name"].as_str().is_some_and(|name| !name.is_empty())
                    || value["content_block"]["text"].as_str().is_some_and(|text| !text.is_empty())
            }
            Some("content_block_delta") => value["delta"].as_object().is_some_and(|delta| {
                delta.iter().any(|(key, value)| {
                    key != "type" && value.as_str().is_some_and(|value| !value.is_empty())
                })
            }),
            Some("message_delta") => true,
            _ => false,
        };
    Ok(if output { Activity::Output } else { Activity::Quiet })
}

fn frame_activity(frame: &[u8]) -> Result<Activity, RouteError> {
    // The relay accepts LF and CRLF. Reject bare CR to avoid classifying a
    // frame differently from clients that also recognize CR as a delimiter.
    let frame = std::str::from_utf8(frame)
        .map_err(|_| RouteError::StreamFailed)?
        .trim_start_matches('\u{feff}');
    if frame.lines().any(|line| line.contains('\r')) {
        return Err(RouteError::StreamFailed);
    }
    let event = frame.lines().rev().find_map(|line| sse_field(line, "event"));
    if event.is_some_and(|event| is_error_event(event.trim_ascii())) {
        return Err(RouteError::StreamFailed);
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
    let value = serde_json::from_str::<Value>(payload).map_err(|_| RouteError::StreamFailed)?;
    if value["type"].as_str().is_some_and(is_error_event)
        || value.get("error").is_some_and(|error| match error {
            Value::Null | Value::Bool(false) => false,
            Value::String(message) => !message.is_empty(),
            Value::Object(fields) => !fields.is_empty(),
            _ => true,
        })
    {
        return Err(RouteError::StreamFailed);
    }
    if event.is_some_and(is_metadata) || value["type"].as_str().is_some_and(is_metadata) {
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

fn is_error_event(event: &str) -> bool {
    event.eq_ignore_ascii_case("error") || event == "response.failed"
}

fn is_metadata(event: &str) -> bool {
    matches!(
        event.trim_ascii(),
        "ping" | "heartbeat" | "keepalive" | "response.created" | "response.in_progress"
    )
}
