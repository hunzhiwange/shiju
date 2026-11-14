//! Resident-only Chat stream differences; framing and lifecycle stay in the shared decoder.
use super::responses::ChatTools;
use axum::http::StatusCode;
use serde_json::{Value, json};
use std::collections::BTreeMap;

#[derive(Default)]
pub(super) struct State {
    order: BTreeMap<String, usize>,
    calls: BTreeMap<String, (String, String, usize)>,
    legacy: Option<bool>,
}

impl State {
    pub(super) fn normalize(&mut self, chunk: &mut Value) -> Result<(), StatusCode> {
        let Some(choices) = chunk["choices"].as_array_mut() else {
            return Err(StatusCode::BAD_GATEWAY);
        };
        for choice in choices {
            let delta = choice["delta"].as_object_mut().ok_or(StatusCode::BAD_GATEWAY)?;
            let legacy = delta.remove("function_call").filter(|v| !v.is_null());
            let modern = delta.get("tool_calls").is_some_and(|v| !v.is_null());
            if legacy.is_some() && modern {
                return Err(StatusCode::BAD_GATEWAY);
            }
            if legacy.is_some() || modern {
                let mode = legacy.is_some();
                if self.legacy.is_some_and(|previous| previous != mode) {
                    return Err(StatusCode::BAD_GATEWAY);
                }
                let first = self.legacy.is_none();
                self.legacy = Some(mode);
                if let Some(function) = legacy {
                    super::messages::object_keys(&function, &["name", "arguments"])?;
                    let mut call = json!({"index":0,"type":"function","function":function});
                    if first {
                        call["id"] = "call_legacy_0".into();
                    }
                    delta.insert("tool_calls".into(), json!([call]));
                }
            }
            if choice["finish_reason"] == "function_call" {
                choice["finish_reason"] = "tool_calls".into();
            }
        }
        Ok(())
    }

    pub(super) fn deltas(
        &mut self,
        calls: &BTreeMap<u64, Value>,
        tools: &ChatTools,
        response_id: &str,
    ) -> Result<Vec<Value>, StatusCode> {
        let mut events = Vec::new();
        for (index, call) in calls {
            let id = call["id"].as_str().ok_or(StatusCode::BAD_GATEWAY)?;
            let name = call["function"]["name"].as_str().ok_or(StatusCode::BAD_GATEWAY)?;
            let args = call["function"]["arguments"].as_str().ok_or(StatusCode::BAD_GATEWAY)?;
            if id.is_empty() || name.is_empty() || args.is_empty() {
                continue;
            }
            let item_id = format!("{response_id}_stream_fc_{index}");
            // Custom tool inputs are encoded JSON: expose them only after full decoding.
            let block = json!({"id":id,"name":name,"input":{}});
            let item = tools.output(&block, call, item_id.clone(), "in_progress");
            let Ok(mut item) = item else { continue };
            if item["type"] != "function_call" {
                continue;
            }
            let first = !self.calls.contains_key(id);
            if first {
                if self.calls.values().any(|(existing, _, _)| existing == &item_id) {
                    return Err(StatusCode::BAD_GATEWAY);
                }
                self.calls.insert(id.into(), (item_id.clone(), name.into(), 0));
                item["arguments"] = "".into();
                events.push(json!({"type":"response.output_item.added","item":item}));
            }
            let (original_id, original_name, sent) =
                self.calls.get_mut(id).ok_or(StatusCode::BAD_GATEWAY)?;
            if original_id.as_str() != item_id.as_str()
                || original_name.as_str() != name
                || *sent > args.len()
            {
                return Err(StatusCode::BAD_GATEWAY);
            }
            if *sent < args.len() {
                events.push(json!({"type":"response.function_call_arguments.delta","item_id":item_id,"delta":&args[*sent..]}));
                *sent = args.len();
            }
        }
        Ok(events)
    }

    pub(super) fn streamed(&self, item: &Value) -> bool {
        item["call_id"].as_str().is_some_and(|id| self.calls.contains_key(id))
    }

    fn item(&self, item: &mut Value) {
        if let Some((id, _, _)) = item["call_id"].as_str().and_then(|id| self.calls.get(id)) {
            item["id"] = id.clone().into();
        }
    }

    pub(super) fn decorate(&mut self, event: &mut Value) {
        if let Some(item) = event.get_mut("item") {
            self.item(item);
        }
        let id =
            event["item"]["id"].as_str().or_else(|| event["item_id"].as_str()).map(str::to_owned);
        if let Some(id) = id {
            let next = self.order.len();
            let index = *self.order.entry(id).or_insert(next);
            event["output_index"] = index.into();
        }
        if let Some(items) = event
            .get_mut("response")
            .and_then(|response| response.get_mut("output"))
            .and_then(Value::as_array_mut)
        {
            for item in items.iter_mut() {
                self.item(item);
            }
            items.sort_by_key(|item| {
                self.order
                    .get(item["id"].as_str().unwrap_or_default())
                    .copied()
                    .unwrap_or(usize::MAX)
            });
        }
    }

    pub(super) fn final_item(&self, item: &mut Value) {
        self.item(item);
    }
}
