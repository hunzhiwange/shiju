//! Preserve raw constraints and keep user answers out of the SDK's debug/trace payloads.
use super::mcp_elicitation::MAX_PENDING;
use anyhow::{Result, ensure};
use rmcp::model::RequestId;
use serde_json::{Value, json};
use std::{
    collections::{HashMap, hash_map::Entry},
    sync::Mutex,
};
use tokio_util::sync::CancellationToken;

pub(super) struct FormWireAnswer {
    pub(super) result: Value,
    pub(super) request_closed: CancellationToken,
    pub(super) operation_closed: CancellationToken,
    pub(super) connection_closed: CancellationToken,
    pub(super) ui_closed: CancellationToken,
    pub(super) deadline: tokio::time::Instant,
}
impl FormWireAnswer {
    fn current(&mut self) -> &Value {
        if self.request_closed.is_cancelled()
            || self.operation_closed.is_cancelled()
            || self.connection_closed.is_cancelled()
            || self.ui_closed.is_cancelled()
            || tokio::time::Instant::now() >= self.deadline
        {
            self.result = json!({"action":"cancel"});
        }
        &self.result
    }
}
#[derive(Default)]
pub(crate) struct FormWireRequests {
    requests: Mutex<HashMap<String, WireRequest>>,
    answers: Mutex<HashMap<String, FormWireAnswer>>,
}
struct WireRequest {
    params: Option<Value>,
    cancelled: CancellationToken,
}
impl FormWireRequests {
    pub(crate) fn prepare(&self, message: &mut Value) {
        // SDK RequestContext.ct is also cancelled on ordinary response completion.
        // Track the peer's actual cancellation independently through the final send.
        if message.get("method").and_then(Value::as_str) == Some("notifications/cancelled") {
            if let Some(key) = message
                .get("params")
                .and_then(|p| p.get("requestId"))
                .and_then(|id| serde_json::to_string(id).ok())
            {
                if let Some(request) = self
                    .requests
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .get(&key)
                {
                    request.cancelled.cancel();
                }
            }
            return;
        }
        if message.get("method").and_then(Value::as_str) != Some("elicitation/create") {
            return;
        }
        if let Some(id) = message
            .get("id")
            .and_then(|id| serde_json::to_string(id).ok())
            .filter(|id| id.len() <= 256)
        {
            let mut requests =
                self.requests.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            let room = requests.len() < MAX_PENDING;
            match requests.entry(id) {
                Entry::Occupied(mut entry) => {
                    entry.get().cancelled.cancel();
                    entry.get_mut().params = None;
                }
                Entry::Vacant(entry) if room => {
                    let params = message
                        .get("params")
                        .filter(|params| {
                            serde_json::to_vec(params).is_ok_and(|bytes| bytes.len() <= 65_536)
                        })
                        .cloned();
                    entry.insert(WireRequest { params, cancelled: CancellationToken::new() });
                }
                _ => {}
            }
        }
        // Only the bounded raw params are validated. Unknown constraints must not be
        // discarded by rmcp's permissive enum deserializer before reaching that check.
        message["params"] =
            json!({"message":"", "requestedSchema":{"type":"object","properties":{}}});
    }
    pub(crate) fn take(&self, id: &RequestId) -> Option<(Value, CancellationToken)> {
        let key = serde_json::to_string(id).ok()?;
        let mut requests = self.requests.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let request = requests.get_mut(&key)?;
        Some((request.params.take()?, request.cancelled.clone()))
    }
    pub(super) fn store_answer(&self, id: &RequestId, answer: FormWireAnswer) -> Result<()> {
        let mut answers = self.answers.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        ensure!(answers.len() < MAX_PENDING, "MCP form response queue is full");
        let key = serde_json::to_string(id)?;
        ensure!(!answers.contains_key(&key), "MCP form response is already queued");
        answers.insert(key, answer);
        Ok(())
    }
    pub(crate) fn prepare_answer(&self, message: &mut Value) -> Option<String> {
        if message.get("result").is_none() && message.get("error").is_none() {
            return None;
        }
        let id = serde_json::to_string(message.get("id")?).ok()?;
        let mut answers = self.answers.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(answer) = answers.get_mut(&id) {
            message["result"] = answer.current().clone();
            return Some(id);
        }
        // An already acknowledged or abandoned masked response can never become an
        // empty acceptance on a transport retry.
        if message["result"]["action"] == "accept" && message["result"].get("content").is_none() {
            message["result"] = json!({"action":"cancel"});
        }
        self.requests
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .contains_key(&id)
            .then_some(id)
    }
    pub(crate) fn complete_answer(&self, id: Option<String>) {
        if let Some(id) = id {
            self.answers.lock().unwrap_or_else(std::sync::PoisonError::into_inner).remove(&id);
            if let Some(request) =
                self.requests.lock().unwrap_or_else(std::sync::PoisonError::into_inner).remove(&id)
            {
                request.cancelled.cancel();
            }
        }
    }
}
