//! Strict assembly of the sole public work tool. Arguments alone never start work.
use super::realtime::{id, voice_id};
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::HashMap;
use vw_api_types::voice::{VoiceCounter, VoiceErrorCode, VoiceId};
type Result<T> = std::result::Result<T, VoiceErrorCode>;

pub(crate) fn tools() -> Value {
    json!([{"type":"function","name":"shiju_handoff","description":"交给当前 ShiJu 会话执行用户工作，沿用权限审批。","parameters":{"type":"object","properties":{"instruction":{"type":"string"}},"required":["instruction"],"additionalProperties":false}}])
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct HandoffCall {
    pub call_id: VoiceId,
    pub item_id: VoiceId,
    pub response_id: VoiceId,
    pub generation: VoiceCounter,
    pub instruction: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Arguments {
    instruction: String,
}
fn instruction(arguments: &str) -> Result<String> {
    if arguments.len() > 8192 {
        return Err(VoiceErrorCode::PayloadTooLarge);
    }
    let args: Arguments =
        serde_json::from_str(arguments).map_err(|_| VoiceErrorCode::ProtocolError)?;
    if args.instruction.trim().is_empty() || args.instruction.chars().count() > 4000 {
        return Err(VoiceErrorCode::InvalidRequest);
    }
    Ok(args.instruction)
}
#[derive(Clone, PartialEq, Eq)]
struct ToolItem {
    response: String,
    call: String,
    generation: VoiceCounter,
    deltas: String,
    arguments: Option<String>,
    completed: Option<String>,
    emitted: bool,
}
#[derive(Default)]
pub(crate) struct HandoffItems {
    items: HashMap<String, ToolItem>,
    calls: HashMap<String, String>,
    responses: HashMap<String, String>,
    ready: Vec<HandoffCall>,
}
impl HandoffItems {
    pub(crate) fn event(&mut self, value: &Value, generation: VoiceCounter) -> Result<bool> {
        let kind = value["type"].as_str().ok_or(VoiceErrorCode::ProtocolError)?;
        match kind {
            "response.output_item.added" if value["item"]["type"] == "function_call" => {
                let item = id(&value["item"], "id")?;
                let response = id(value, "response_id")?;
                let call = id(&value["item"], "call_id")?;
                if value["item"]["name"] != "shiju_handoff" {
                    return Err(VoiceErrorCode::ProtocolError);
                }
                if let Some(existing) = self.items.get(&item) {
                    if existing.response != response
                        || existing.call != call
                        || existing.generation != generation
                    {
                        return Err(VoiceErrorCode::ProtocolError);
                    }
                    return Ok(true);
                }
                if self.items.len() >= 256 {
                    return Err(VoiceErrorCode::Backpressure);
                }
                if self.calls.contains_key(&call) {
                    return Err(VoiceErrorCode::ProtocolError);
                }
                self.calls.insert(call.clone(), item.clone());
                self.items.insert(
                    item,
                    ToolItem {
                        response,
                        call,
                        generation,
                        deltas: String::new(),
                        arguments: None,
                        completed: None,
                        emitted: false,
                    },
                );
                Ok(true)
            }
            "response.function_call_arguments.delta" | "response.function_call_arguments.done" => {
                let item = id(value, "item_id")?;
                let response = id(value, "response_id")?;
                let call = id(value, "call_id")?;
                let state = self.items.get_mut(&item).ok_or(VoiceErrorCode::ProtocolError)?;
                if state.response != response || state.call != call {
                    return Err(VoiceErrorCode::ProtocolError);
                }
                if state.generation != generation {
                    return Ok(true);
                }
                if kind.ends_with(".delta") {
                    let delta = value["delta"].as_str().ok_or(VoiceErrorCode::ProtocolError)?;
                    if state.arguments.is_some() || state.deltas.len() + delta.len() > 8192 {
                        return Err(VoiceErrorCode::ProtocolError);
                    }
                    state.deltas.push_str(delta);
                } else {
                    if value["name"] != "shiju_handoff" {
                        return Err(VoiceErrorCode::ProtocolError);
                    }
                    let args = value["arguments"].as_str().ok_or(VoiceErrorCode::ProtocolError)?;
                    instruction(args)?;
                    if state.arguments.as_deref().is_some_and(|old| old != args)
                        || (!state.deltas.is_empty() && state.deltas != args)
                    {
                        return Err(VoiceErrorCode::ProtocolError);
                    }
                    state.arguments = Some(args.to_owned());
                }
                self.release(&item, generation)?;
                Ok(true)
            }
            "response.output_item.done" if value["item"]["type"] == "function_call" => {
                let item = id(&value["item"], "id")?;
                let response = id(value, "response_id")?;
                let call = id(&value["item"], "call_id")?;
                let state = self.items.get_mut(&item).ok_or(VoiceErrorCode::ProtocolError)?;
                if state.response != response
                    || state.call != call
                    || value["item"]["name"] != "shiju_handoff"
                {
                    return Err(VoiceErrorCode::ProtocolError);
                }
                if state.generation != generation {
                    return Ok(true);
                }
                if value["item"]["status"] != "completed" {
                    return Err(VoiceErrorCode::ProtocolError);
                }
                let args =
                    value["item"]["arguments"].as_str().ok_or(VoiceErrorCode::ProtocolError)?;
                instruction(args)?;
                if state.completed.as_deref().is_some_and(|old| old != args) {
                    return Err(VoiceErrorCode::ProtocolError);
                }
                state.completed = Some(args.to_owned());
                self.release(&item, generation)?;
                Ok(true)
            }
            "response.done" => {
                let response = id(&value["response"], "id")?;
                let status =
                    value["response"]["status"].as_str().ok_or(VoiceErrorCode::ProtocolError)?;
                if let Some(old) = self.responses.get(&response) {
                    if old != status {
                        return Err(VoiceErrorCode::ProtocolError);
                    }
                    return Ok(true);
                }
                if self.responses.len() >= 256 {
                    return Err(VoiceErrorCode::Backpressure);
                }
                self.responses.insert(response.clone(), status.to_owned());
                let items: Vec<_> = self
                    .items
                    .iter()
                    .filter(|(_, s)| s.response == response)
                    .map(|(item, _)| item.clone())
                    .collect();
                for item in items {
                    self.release(&item, generation)?;
                }
                Ok(false) // normal response lifecycle still consumes this event
            }
            _ => Ok(false),
        }
    }
    fn release(&mut self, item: &str, generation: VoiceCounter) -> Result<()> {
        let state = self.items.get_mut(item).ok_or(VoiceErrorCode::ProtocolError)?;
        if state.emitted
            || state.generation != generation
            || self.responses.get(&state.response).map(String::as_str) != Some("completed")
        {
            return Ok(());
        }
        let (Some(arguments), Some(completed)) = (&state.arguments, &state.completed) else {
            return Ok(());
        };
        if arguments != completed {
            return Err(VoiceErrorCode::ProtocolError);
        }
        let call = HandoffCall {
            call_id: voice_id(state.call.clone())?,
            item_id: voice_id(item.to_owned())?,
            response_id: voice_id(state.response.clone())?,
            generation,
            instruction: instruction(arguments)?,
        };
        state.emitted = true;
        self.ready.push(call);
        Ok(())
    }
    pub(crate) fn take(&mut self) -> Vec<HandoffCall> {
        std::mem::take(&mut self.ready)
    }
    pub(crate) fn contains(&self, item: &str) -> bool {
        self.items.contains_key(item)
    }
    pub(crate) fn retire(&mut self) {
        self.ready.clear();
    }
}
#[cfg(test)]
#[path = "realtime_handoff_tests.rs"]
mod tests;
