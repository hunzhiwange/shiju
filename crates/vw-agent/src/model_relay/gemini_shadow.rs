//! Opaque Gemini receipts owned by an authorized route, never by client headers.
use crate::app_routing::reference_scope::ReferenceScope;
use axum::http::StatusCode;
use serde_json::Value;
use std::{sync::Arc, time::{Duration, Instant}};

const TTL: Duration = Duration::from_secs(1800);
const CAPACITY: usize = 1024;
const MAX_PART: usize = 64 * 1024;

struct Receipt {
    scope: ReferenceScope,
    session: String,
    id: String,
    part: Value,
    created: Instant,
}
#[derive(Default)]
pub(crate) struct Store(parking_lot::Mutex<Vec<Receipt>>);
impl Store {
    pub(crate) fn clear(&self) { self.0.lock().clear(); }
}

#[derive(Clone)]
pub(crate) struct Context {
    pub(crate) store: Arc<Store>,
    pub(crate) scope: ReferenceScope,
    pub(crate) session: String,
    pub(crate) cancelled: tokio_util::sync::CancellationToken,
}
impl Context {
    /// Each client-visible random call id is a session receipt, never an authority.
    /// Exact call contents must match as well; names alone cannot select a receipt.
    pub(crate) fn replay(&self, body: &mut Value) -> Result<(), StatusCode> {
        let mut entries = self.store.0.lock();
        if self.cancelled.is_cancelled() { entries.clear(); return Err(StatusCode::FORBIDDEN); }
        entries.retain(|r| r.created.elapsed() < TTL);
        let Some(contents) = body["contents"].as_array_mut() else { return Ok(()) };
        for content in contents {
            let model = content["role"] == "model";
            let Some(parts) = content["parts"].as_array_mut() else { continue };
            for part in parts {
                let Some(call) = part.get("functionCall") else { continue };
                let Some(id) = call["id"].as_str() else { continue };
                if let Some(r) = entries.iter().find(|r| r.scope == self.scope && r.session == self.session && r.id == id) {
                    if !model || call["name"] != r.part["functionCall"]["name"]
                        || call["args"] != r.part["functionCall"]["args"] {
                        return Err(StatusCode::FORBIDDEN);
                    }
                    *part = r.part.clone();
                } else if id.starts_with("shiju_gemini_") {
                    return Err(StatusCode::FORBIDDEN);
                }
            }
        }
        Ok(())
    }

    /// Strip opaque metadata only on this resident conversion path. Native and
    /// legacy relay validation keep their existing strict field policy.
    pub(crate) fn capture(&self, body: &mut Value) -> Result<(), StatusCode> {
        let Some(parts) = body.pointer_mut("/candidates/0/content/parts").and_then(Value::as_array_mut) else { return Ok(()) };
        let mut pending = Vec::new();
        for part in parts {
            for key in ["thoughtSignature", "thought_signature"] {
                if part.get(key).is_some_and(|v| !v.is_null() && !v.is_string()) {
                    return Err(StatusCode::BAD_GATEWAY);
                }
            }
            if part.get("functionCall").is_some() {
                if part.to_string().len() > MAX_PART { return Err(StatusCode::BAD_GATEWAY); }
                if part["functionCall"].get("args").is_none() { part["functionCall"]["args"] = serde_json::json!({}); }
                let id = format!("shiju_gemini_{}", uuid::Uuid::new_v4().simple());
                pending.push(Receipt { scope: self.scope.clone(), id: id.clone(), part: part.clone(), session: self.session.clone(), created: Instant::now() });
                part["functionCall"]["id"] = id.into();
            }
            let object = part.as_object_mut().ok_or(StatusCode::BAD_GATEWAY)?;
            object.remove("thoughtSignature");
            object.remove("thought_signature");
        }
        let mut entries = self.store.0.lock();
        if self.cancelled.is_cancelled() { entries.clear(); return Err(StatusCode::FORBIDDEN); }
        entries.retain(|r| r.created.elapsed() < TTL);
        entries.extend(pending);
        let excess = entries.len().saturating_sub(CAPACITY);
        entries.drain(..excess);
        Ok(())
    }
}
