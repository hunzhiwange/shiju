//! Session usage expansion, raw payload requests and modal state.
use std::collections::{HashMap, HashSet};
use crate::view_types::{UsageModelInfo, UsageStepRawModal};
pub type Key = (String, String, u32);
pub type PayloadResult = Result<Option<serde_json::Value>, String>;
#[derive(Debug, Clone)]
pub enum Message {
    UsageModelInfoLoaded(Option<UsageModelInfo>),
    UsageStepToggled(u32), UsageTurnStepToggled(String, u32),
    UsageStepRawLoaded(String, u32, PayloadResult),
    UsageTurnStepRawLoaded(String, String, u32, PayloadResult),
    UsageStepRawOpen(u32), UsageTurnStepRawOpen(String, u32), UsageStepRawClose,
}
#[derive(Default)]
pub struct State {
    pub usage_model_info: Option<UsageModelInfo>,
    pub usage_step_expanded: HashSet<u32>,
    pub usage_step_raw_payloads: HashMap<Key, PayloadResult>,
    pub usage_step_raw_loading: HashSet<Key>,
    pub usage_step_raw_modal: Option<UsageStepRawModal>,
}
pub enum Effect { InsertRaw(Key, serde_json::Value), LoadRaw(Key) }
fn raw_modal(step_index: u32, value: &serde_json::Value) -> UsageStepRawModal {
    let content = serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string());
    UsageStepRawModal { step_index, content }
}
fn open_raw_modal(app: &mut State, raw: &HashMap<Key, serde_json::Value>, key: &Key) {
    if let Some(value) = raw.get(key) { app.usage_step_raw_modal = Some(raw_modal(key.2, value)); }
}
pub fn turn_id_for_step(steps: &[session_ui_types::ChatSessionStep], step_index: u32) -> String {
    steps.iter().find(|step| step.index == step_index)
        .and_then(|step| step.turn_id.clone())
        .filter(|turn_id| !turn_id.trim().is_empty())
        .unwrap_or_else(|| format!("legacy-step-{step_index}"))
}
pub fn update(app: &mut State, active_session_id: Option<&str>, steps: &[session_ui_types::ChatSessionStep], raw: &HashMap<Key, serde_json::Value>, message: Message) -> Option<Effect> {
    let message = match message {
        Message::UsageStepToggled(step_index) => {
            let turn_id = turn_id_for_step(steps, step_index);
            Message::UsageTurnStepToggled(turn_id, step_index)
        }
        Message::UsageStepRawLoaded(session_id, step_index, result) => {
            let turn_id = turn_id_for_step(steps, step_index);
            Message::UsageTurnStepRawLoaded(session_id, turn_id, step_index, result)
        }
        Message::UsageStepRawOpen(step_index) => {
            let turn_id = turn_id_for_step(steps, step_index);
            Message::UsageTurnStepRawOpen(turn_id, step_index)
        }
        message => message,
    };
    match message {
        Message::UsageModelInfoLoaded(info) => {
            app.usage_model_info = info;
            None
        }
        Message::UsageTurnStepToggled(_turn_id, step_index) => {
            if app.usage_step_expanded.contains(&step_index) {
                app.usage_step_expanded.remove(&step_index);
                if app
                    .usage_step_raw_modal
                    .as_ref()
                    .is_some_and(|modal| modal.step_index == step_index)
                {
                    app.usage_step_raw_modal = None;
                }
                return None;
            }
            app.usage_step_expanded.insert(step_index);
            None
        }
        Message::UsageTurnStepRawLoaded(session_id, turn_id, step_index, result) => {
            let key = (session_id.clone(), turn_id, step_index);
            app.usage_step_raw_loading.remove(&key);
            if active_session_id == Some(session_id.as_str()) {
                match result {
                    Ok(Some(raw)) => {

                        app.usage_step_raw_payloads.remove(&key);
                        if app.usage_step_expanded.contains(&step_index) {
                            app.usage_step_raw_modal = Some(raw_modal(step_index, &raw));
                        }
                        return Some(Effect::InsertRaw(key, raw));
                    }
                    result => {
                        app.usage_step_raw_payloads.insert(key, result);
                    }
                }
            }
            None
        }
        Message::UsageTurnStepRawOpen(turn_id, step_index) => {
            let Some(session_id) = active_session_id.map(str::to_owned) else {
                return None;
            };
            let key = (session_id.clone(), turn_id.clone(), step_index);
            if raw.contains_key(&key) {
                open_raw_modal(app, raw, &key);
                return None;
            }
            if app.usage_step_raw_loading.contains(&key)
                || matches!(app.usage_step_raw_payloads.get(&key), Some(Ok(None)))
            {
                return None;
            }
            app.usage_step_raw_payloads.remove(&key);
            app.usage_step_raw_loading.insert(key);
            Some(Effect::LoadRaw((session_id, turn_id, step_index)))
        }
        Message::UsageStepRawClose => {
            app.usage_step_raw_modal = None;
            None
        }
        _ => None,
    }
}
