//! 原始轨迹补载的候选筛选、稳定键与重试规则。
use std::collections::{BTreeSet, HashMap, HashSet};
use std::time::Duration;
use session_ui_types::ChatSessionStep;
use serde_json::Value;
pub type SessionStepRawKey = (String, String, u32);
pub const MAX_RAW_LOAD_ATTEMPTS: u8 = 3;
const SECOND_RAW_LOAD_DELAY: Duration = Duration::from_millis(300);
const THIRD_RAW_LOAD_DELAY: Duration = Duration::from_millis(900);

pub fn retry_delay(attempt: u8) -> Option<Duration> {
    match attempt { 2 => Some(SECOND_RAW_LOAD_DELAY), 3 => Some(THIRD_RAW_LOAD_DELAY), _ => None }
}

/// 仅借用一次补载筛选实际需要的状态；不含视图或网关能力。
pub struct RawLoadState<'a> {
    pub steps: &'a [ChatSessionStep],
    pub indices: &'a HashSet<SessionStepRawKey>,
    pub raw: &'a HashMap<SessionStepRawKey, Value>,
    pub loading: &'a mut HashSet<SessionStepRawKey>,
    pub attempts: &'a mut HashMap<SessionStepRawKey, u8>,
    pub errors: &'a mut HashMap<SessionStepRawKey, String>,
}
impl RawLoadState<'_> {
    pub fn missing(&mut self, session_id: &str, only_step_index: Option<u32>) -> Vec<SessionStepRawKey> {
        self.loading
            .retain(|(loading_session_id, _, _)| loading_session_id == session_id);
        self.attempts
            .retain(|(loading_session_id, _, _), _| loading_session_id == session_id);
        self.errors.retain(|(error_session_id, _, _), _| error_session_id == session_id);

        let mut candidate_keys = self.steps
            .iter()
            .filter(|step| step.finished_ms.is_some())
            .filter(|step| only_step_index.is_none_or(|index| index == step.index))
            .map(|step| (session_id.to_string(), trajectory_turn_id(step), step.index))
            .collect::<BTreeSet<_>>();
        candidate_keys.extend(
            self.indices
                .iter()
                .filter(|(stored_session_id, _, step_index)| {
                    stored_session_id == session_id
                        && only_step_index.is_none_or(|index| index == *step_index)
                })
                .cloned(),
        );
        let missing = candidate_keys
            .into_iter()
            .filter(|key| {
                !self.raw.contains_key(key)
                    && !self.loading.contains(key)
            })
            .collect::<Vec<_>>();

        for key in &missing {
            self.errors.remove(key);
            self.loading.insert(key.clone());
            self.attempts.insert(key.clone(), 1);
        }
        missing
    }
    pub fn clear(&mut self, key: &SessionStepRawKey) {
        self.loading.remove(key);
        self.attempts.remove(key);
        self.errors.remove(key);
    }
}
pub fn is_current_completed_missing_raw(
    active_session_id: Option<&str>,
    steps: &[ChatSessionStep],
    indices: &HashSet<SessionStepRawKey>,
    raw: &HashMap<SessionStepRawKey, Value>,
    session_id: &str,
    turn_id: &str,
    step_index: u32,
) -> bool {
    let key = (session_id.to_string(), turn_id.to_string(), step_index);
    active_session_id == Some(session_id)
        && !raw.contains_key(&key)
        && (indices.contains(&key)
            || steps.iter().any(|step| {
                step.index == step_index
                    && trajectory_turn_id(step) == turn_id
                    && step.finished_ms.is_some()
            }))
}

fn trajectory_turn_id(step: &ChatSessionStep) -> String {
    step.turn_id
        .clone()
        .filter(|turn_id| !turn_id.trim().is_empty())
        .unwrap_or_else(|| format!("legacy-step-{}", step.index))
}

/// 网关结果应用只返回实际需要的载荷或延时重试效果。
pub enum RawResultEffect {
    None,
    Insert(Value),
    Retry { attempt: u8, delay: Duration },
}

impl RawLoadState<'_> {
    pub fn accepts(&mut self, active_session_id: Option<&str>, key: &SessionStepRawKey, attempt: u8) -> bool {
        if active_session_id != Some(key.0.as_str()) || !(1..=MAX_RAW_LOAD_ATTEMPTS).contains(&attempt) {
            self.clear(key);
            return false;
        }
        // 较早的回包不能清除较新一次重试的状态。
        if self.attempts.get(key).copied() != Some(attempt) {
            return false;
        }
        if !is_current_completed_missing_raw(active_session_id, self.steps, self.indices, self.raw, &key.0, &key.1, key.2) {
            self.clear(key);
            return false;
        }
        true
    }

    pub fn apply_result(&mut self, active_session_id: Option<&str>, key: &SessionStepRawKey, attempt: u8, result: Result<Option<Value>, String>) -> RawResultEffect {
        if !self.accepts(active_session_id, key, attempt) {
            return RawResultEffect::None;
        }
        match result {
            Ok(Some(raw)) => RawResultEffect::Insert(raw),
            Ok(None) | Err(_) if attempt < MAX_RAW_LOAD_ATTEMPTS => {
                let next = attempt + 1;
                let Some(delay) = retry_delay(next) else { return RawResultEffect::None; };
                self.loading.insert(key.clone());
                self.attempts.insert(key.clone(), next);
                self.errors.remove(key);
                RawResultEffect::Retry { attempt: next, delay }
            }
            result => {
                self.clear(key);
                let error = match result {
                    Err(error) => error,
                    _ => "原始数据自动重试 3 次后仍未就绪，可稍后重试".to_string(),
                };
                self.errors.insert(key.clone(), error);
                RawResultEffect::None
            }
        }
    }
}

pub fn accepted_snapshot(
    trajectory_mode: bool,
    active_session_id: Option<&str>,
    session_id: &str,
    result: Result<Option<gateway_session::SessionUiLoadResult>, String>,
) -> Option<gateway_session::SessionUiLoadResult> {
    if !trajectory_mode || active_session_id != Some(session_id) {
        return None;
    }
    let loaded = result.ok()??;
    (loaded.session.id == session_id).then_some(loaded)
}

/// 按快照原始稳定键去重；规范化仍由实际插入步骤执行。
pub fn missing_snapshot_steps(existing: &[ChatSessionStep], incoming: Vec<ChatSessionStep>) -> Vec<ChatSessionStep> {
    let mut known = existing.iter().map(|step| (step.index, step.turn_id.clone())).collect::<Vec<_>>();
    let mut missing = Vec::new();
    for step in incoming {
        if !known.iter().any(|old| old.0 == step.index && old.1.as_deref() == step.turn_id.as_deref()) {
            let turn_id = step.turn_id.clone().filter(|id| !id.trim().is_empty());
            known.push((step.index, turn_id));
            missing.push(step);
        }
    }
    missing
}
