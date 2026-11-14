//! One observer per authorized HTTP request, including response-body lifetime.
use axum::{body::Body, response::Response};
use futures_util::StreamExt;
use parking_lot::Mutex;
use std::{collections::VecDeque, sync::Arc, time::Instant};
use vw_api_types::{
    agent_apps::AppId,
    app_routing::{RoutingRequestState as State, RoutingRequestStatus, RoutingRequestsStatus},
    id::ProjectScope,
};

#[derive(Default)]
pub(crate) struct Requests(Mutex<VecDeque<Row>>);
struct Row {
    project: ProjectScope,
    host: String,
    app: AppId,
    started: Instant,
    status: RoutingRequestStatus,
}
impl Requests {
    pub(super) fn begin(
        self: &Arc<Self>,
        project: ProjectScope,
        host: String,
        app: AppId,
    ) -> Option<Guard> {
        let mut rows = self.0.lock();
        if rows.iter().filter(|row| row.status.state == State::Active).count() >= 1024 {
            return None;
        }
        let id = uuid::Uuid::new_v4().to_string();
        rows.push_back(Row {
            project,
            host,
            app,
            started: Instant::now(),
            status: RoutingRequestStatus {
                request_id: id.clone(),
                state: State::Active,
                first_token_ms: None,
                elapsed_ms: 0,
            },
        });
        Some(Guard { requests: Arc::clone(self), id })
    }
    pub(crate) fn snapshot(
        &self,
        project: &ProjectScope,
        host: &str,
        app: AppId,
    ) -> RoutingRequestsStatus {
        let requests: Vec<_> = self
            .0
            .lock()
            .iter()
            .filter(|row| &row.project == project && row.host == host && row.app == app)
            .map(|row| {
                let mut status = row.status.clone();
                if status.state == State::Active {
                    status.elapsed_ms = millis(row.started);
                }
                status
            })
            .collect();
        RoutingRequestsStatus {
            active_count: requests.iter().filter(|row| row.state == State::Active).count(),
            requests,
        }
    }
}
fn millis(started: Instant) -> u64 {
    started.elapsed().as_millis().min(u64::MAX as u128) as u64
}
pub(super) struct Guard {
    requests: Arc<Requests>,
    id: String,
}
impl Guard {
    fn output(&self) {
        if let Some(row) =
            self.requests.0.lock().iter_mut().find(|row| row.status.request_id == self.id)
        {
            if row.status.first_token_ms.is_none() {
                row.status.first_token_ms = Some(millis(row.started));
            }
        }
    }
    fn finish(&self, state: State) {
        let mut rows = self.requests.0.lock();
        if let Some(row) = rows.iter_mut().find(|row| row.status.request_id == self.id) {
            if row.status.state != State::Active {
                return;
            }
            row.status.state = state;
            row.status.elapsed_ms = millis(row.started);
        }
        while rows.iter().filter(|row| row.status.state != State::Active).count() > 256 {
            if let Some(index) = rows.iter().position(|row| row.status.state != State::Active) {
                rows.remove(index);
            }
        }
    }
    pub(super) fn response(self, response: Response) -> Response {
        let success = response.status().is_success();
        let sse =
            response.headers().get("content-type").and_then(|v| v.to_str().ok()).is_some_and(|v| {
                v.split(';').next().is_some_and(|v| v.trim() == "text/event-stream")
            });
        let (parts, body) = response.into_parts();
        let stream = futures_util::stream::unfold(
            (body.into_data_stream(), self, Vec::new(), false),
            move |(mut stream, guard, mut pending, mut observed)| async move {
                match stream.next().await {
                    Some(Ok(bytes)) => {
                        if sse && success {
                            for byte in bytes.iter() {
                                // Only retain a bounded current line, never request content.
                                if *byte == b'\n' {
                                    if failure_line(&pending) {
                                        guard.finish(State::Failed);
                                    }
                                    if !observed && output_line(&pending) {
                                        guard.output();
                                        observed = true;
                                    }
                                    pending.clear();
                                } else if pending.len() < 65536 {
                                    pending.push(*byte);
                                } else {
                                    observed = true;
                                    pending.clear();
                                }
                            }
                        }
                        Some((Ok::<_, axum::Error>(bytes), (stream, guard, pending, observed)))
                    }
                    Some(Err(error)) => {
                        guard.finish(State::Failed);
                        Some((Err(error), (stream, guard, pending, observed)))
                    }
                    None => {
                        guard.finish(if success { State::Completed } else { State::Failed });
                        None
                    }
                }
            },
        );
        Response::from_parts(parts, Body::from_stream(stream))
    }
}
impl Drop for Guard {
    fn drop(&mut self) {
        self.finish(State::Cancelled);
    }
}
fn output_line(line: &[u8]) -> bool {
    let Some(data) = line.strip_prefix(b"data:") else {
        return false;
    };
    let Ok(v) = serde_json::from_slice::<serde_json::Value>(data) else {
        return false;
    };
    let nonempty = |v: &serde_json::Value| v.as_str().is_some_and(|s| !s.is_empty());
    match v.get("type").and_then(|v| v.as_str()) {
        Some("content_block_delta") => {
            ["text", "thinking", "partial_json"].iter().any(|key| nonempty(&v["delta"][*key]))
        }
        Some(
            "response.output_text.delta"
            | "response.reasoning_text.delta"
            | "response.reasoning_summary_text.delta"
            | "response.function_call_arguments.delta",
        ) => nonempty(&v["delta"]),
        _ => {
            v["choices"].as_array().is_some_and(|choices| {
                choices.iter().any(|c| {
                    nonempty(&c["delta"]["content"])
                        || nonempty(&c["delta"]["reasoning_content"])
                        || c["delta"]["tool_calls"].as_array().is_some_and(|t| !t.is_empty())
                })
            }) || v["candidates"].as_array().is_some_and(|candidates| {
                candidates.iter().any(|c| {
                    c["content"]["parts"].as_array().is_some_and(|parts| {
                        parts
                            .iter()
                            .any(|p| nonempty(&p["text"]) || p.get("functionCall").is_some())
                    })
                })
            })
        }
    }
}

fn failure_line(line: &[u8]) -> bool {
    let Some(data) = line.strip_prefix(b"data:") else {
        return false;
    };
    let Ok(v) = serde_json::from_slice::<serde_json::Value>(data) else {
        return false;
    };
    v.get("error").is_some()
        || matches!(
            v.get("type").and_then(|v| v.as_str()),
            Some("error" | "response.failed" | "response.incomplete")
        )
}
