//! 原始步骤 JSON 到轨迹领域模型的纯投影。

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use serde_json::Value;
use vw_shared::session::ui_types::{ChatSessionStep, TokenUsage};

use super::model::{
    MetricAvailability, TrajectoryMetrics, TrajectoryProjection, TrajectoryRecord,
    TrajectoryRecordDetail, TrajectoryRecordKind, TrajectoryStep, TrajectorySummary,
    TrajectoryTurn, TrajectoryUsage,
};

const PREVIEW_CHAR_LIMIT: usize = 160;

#[derive(Debug, Clone)]
struct ToolCallData {
    id: Option<String>,
    name: Option<String>,
    arguments: Option<String>,
    raw: Value,
}

#[derive(Debug, Clone)]
struct AssistantEcho {
    content: Option<String>,
}

#[derive(Default)]
struct ProjectionBuilder {
    records: Vec<TrajectoryRecord>,
    previous_messages: Vec<Value>,
    previous_system: Vec<Value>,
    seen_system: HashSet<String>,
    assistant_echoes: Vec<AssistantEcho>,
    tool_record_indices: HashMap<String, usize>,
    current_turn: Option<u32>,
    next_turn: u32,
}

impl ProjectionBuilder {
    fn new() -> Self {
        Self { next_turn: 1, ..Self::default() }
    }

    fn project_step(
        &mut self,
        step_index: u32,
        step: Option<&ChatSessionStep>,
        raw: Option<&Value>,
    ) -> TrajectoryStep {
        let mut record_ids = Vec::new();
        let mut turn_indices = Vec::new();
        let usage = usage_for(step, raw);
        let started_ms = step.map(|step| step.started_ms);
        let finished_ms = step.and_then(|step| step.finished_ms);
        let duration_ms = duration(started_ms, finished_ms);

        if let Some(raw) = raw.and_then(Value::as_object) {
            self.project_system(step_index, raw.get("system"), &mut record_ids, &mut turn_indices);
            self.project_messages(
                step_index,
                raw.get("messages"),
                &mut record_ids,
                &mut turn_indices,
            );
            self.project_output(
                step_index,
                step,
                raw.get("output"),
                usage,
                &mut record_ids,
                &mut turn_indices,
            );
        }

        TrajectoryStep {
            index: step_index,
            turn_indices,
            record_ids,
            started_ms,
            finished_ms,
            usage,
            duration_ms,
        }
    }

    fn project_system(
        &mut self,
        step_index: u32,
        system: Option<&Value>,
        record_ids: &mut Vec<String>,
        turn_indices: &mut Vec<u32>,
    ) {
        let Some(system) = system.and_then(Value::as_array) else {
            return;
        };
        let prefix_len = common_prefix_len(&self.previous_system, system);
        for (offset, raw) in system[prefix_len..].iter().enumerate() {
            let Some(content) = value_text(raw).filter(|content| !content.trim().is_empty()) else {
                continue;
            };
            if !self.seen_system.insert(content.clone()) {
                continue;
            }
            let absolute_index = prefix_len + offset;
            let id = format!("step:{step_index}:system:{absolute_index}");
            let record = TrajectoryRecord {
                id,
                index: 0,
                kind: TrajectoryRecordKind::System,
                step_index,
                turn_index: None,
                preview: preview_text(&content),
                started_ms: None,
                finished_ms: None,
                usage: MetricAvailability::Unavailable,
                metrics: TrajectoryMetrics::default(),
                detail: TrajectoryRecordDetail {
                    input: Some(content),
                    raw_fragments: vec![raw.clone()],
                    ..TrajectoryRecordDetail::default()
                },
            };
            self.push_record(record, record_ids, turn_indices);
        }
        self.previous_system.clone_from(system);
    }

    fn project_messages(
        &mut self,
        step_index: u32,
        messages: Option<&Value>,
        record_ids: &mut Vec<String>,
        turn_indices: &mut Vec<u32>,
    ) {
        let Some(messages) = messages.and_then(Value::as_array) else {
            return;
        };
        let prefix_len = common_prefix_len(&self.previous_messages, messages);
        for (offset, message) in messages[prefix_len..].iter().enumerate() {
            self.project_message(
                step_index,
                prefix_len + offset,
                message,
                record_ids,
                turn_indices,
            );
        }
        self.previous_messages.clone_from(messages);
    }

    fn project_message(
        &mut self,
        step_index: u32,
        message_index: usize,
        message: &Value,
        record_ids: &mut Vec<String>,
        turn_indices: &mut Vec<u32>,
    ) {
        let Some(message_object) = message.as_object() else {
            return;
        };
        let Some(role) = message_object.get("role").and_then(Value::as_str) else {
            return;
        };
        let content = message_object.get("content").and_then(value_text);
        match role.to_ascii_lowercase().as_str() {
            "system" => {
                let Some(content) = content.filter(|content| !content.trim().is_empty()) else {
                    return;
                };
                if !self.seen_system.insert(content.clone()) {
                    return;
                }
                let record = TrajectoryRecord {
                    id: format!("step:{step_index}:message:{message_index}:system"),
                    index: 0,
                    kind: TrajectoryRecordKind::System,
                    step_index,
                    turn_index: None,
                    preview: preview_text(&content),
                    started_ms: None,
                    finished_ms: None,
                    usage: MetricAvailability::Unavailable,
                    metrics: TrajectoryMetrics::default(),
                    detail: TrajectoryRecordDetail {
                        input: Some(content),
                        raw_fragments: vec![message.clone()],
                        ..TrajectoryRecordDetail::default()
                    },
                };
                self.push_record(record, record_ids, turn_indices);
            }
            "user" => {
                let turn = self.next_turn;
                self.next_turn = self.next_turn.saturating_add(1);
                self.current_turn = Some(turn);
                let preview = content.as_deref().map(preview_text).filter(|text| !text.is_empty());
                let record = TrajectoryRecord {
                    id: format!("step:{step_index}:message:{message_index}:user"),
                    index: 0,
                    kind: TrajectoryRecordKind::User,
                    step_index,
                    turn_index: Some(turn),
                    preview: preview.unwrap_or_else(|| "User message".to_string()),
                    started_ms: None,
                    finished_ms: None,
                    usage: MetricAvailability::Unavailable,
                    metrics: TrajectoryMetrics::default(),
                    detail: TrajectoryRecordDetail {
                        input: content,
                        raw_fragments: vec![message.clone()],
                        ..TrajectoryRecordDetail::default()
                    },
                };
                self.push_record(record, record_ids, turn_indices);
            }
            "assistant" => {
                let calls = parse_tool_calls(message_object.get("tool_calls"));
                if self.is_assistant_echo(content.as_deref(), &calls) {
                    return;
                }
                let preview = assistant_preview(content.as_deref(), None, calls.len());
                let record = TrajectoryRecord {
                    id: format!("step:{step_index}:message:{message_index}:assistant"),
                    index: 0,
                    kind: TrajectoryRecordKind::Assistant,
                    step_index,
                    turn_index: self.current_turn,
                    preview,
                    started_ms: None,
                    finished_ms: None,
                    usage: MetricAvailability::Unavailable,
                    metrics: TrajectoryMetrics::default(),
                    detail: TrajectoryRecordDetail {
                        output: content,
                        raw_fragments: vec![message.clone()],
                        ..TrajectoryRecordDetail::default()
                    },
                };
                self.push_record(record, record_ids, turn_indices);
                for (ordinal, call) in calls.into_iter().enumerate() {
                    self.upsert_tool_call(step_index, ordinal, call, record_ids, turn_indices);
                }
            }
            "tool" => {
                let call_id = string_alias(
                    message,
                    &["tool_call_id", "toolCallId", "call_id", "callId", "id"],
                );
                let name = string_alias(message, &["name", "tool_name", "toolName"]);
                self.attach_tool_result(
                    step_index,
                    message_index,
                    call_id,
                    name,
                    content,
                    message.clone(),
                    record_ids,
                    turn_indices,
                );
            }
            _ => {}
        }
    }

    fn project_output(
        &mut self,
        step_index: u32,
        step: Option<&ChatSessionStep>,
        output: Option<&Value>,
        usage: MetricAvailability<TrajectoryUsage>,
        record_ids: &mut Vec<String>,
        turn_indices: &mut Vec<u32>,
    ) {
        let Some(output_object) = output.and_then(Value::as_object) else {
            return;
        };
        let content = output_object.get("text").and_then(value_text).filter(non_empty);
        let reasoning =
            output_object.get("reasoning_content").and_then(value_text).filter(non_empty);
        let calls = parse_tool_calls(output_object.get("tool_calls"));
        if content.is_none() && reasoning.is_none() && calls.is_empty() {
            return;
        }
        let started_ms = step.map(|step| step.started_ms);
        let finished_ms = step.and_then(|step| step.finished_ms);
        let record = TrajectoryRecord {
            id: format!("step:{step_index}:assistant"),
            index: 0,
            kind: TrajectoryRecordKind::Assistant,
            step_index,
            turn_index: self.current_turn,
            preview: assistant_preview(content.as_deref(), reasoning.as_deref(), calls.len()),
            started_ms,
            finished_ms,
            usage,
            metrics: TrajectoryMetrics {
                duration_ms: duration(started_ms, finished_ms),
                ..TrajectoryMetrics::default()
            },
            detail: TrajectoryRecordDetail {
                output: content.clone(),
                reasoning,
                raw_fragments: vec![Value::Object(output_object.clone())],
                ..TrajectoryRecordDetail::default()
            },
        };
        self.push_record(record, record_ids, turn_indices);
        self.assistant_echoes.push(AssistantEcho { content });
        for (ordinal, call) in calls.into_iter().enumerate() {
            self.upsert_tool_call(step_index, ordinal, call, record_ids, turn_indices);
        }
    }

    fn is_assistant_echo(&self, content: Option<&str>, calls: &[ToolCallData]) -> bool {
        let content_matches =
            content.filter(|content| !content.trim().is_empty()).is_some_and(|content| {
                self.assistant_echoes.iter().any(|echo| echo.content.as_deref() == Some(content))
            });
        let call_ids = calls.iter().filter_map(|call| call.id.as_deref()).collect::<Vec<_>>();
        let calls_match = !calls.is_empty()
            && call_ids.len() == calls.len()
            && call_ids.iter().all(|id| self.tool_record_indices.contains_key(*id));
        content_matches || calls_match
    }

    fn upsert_tool_call(
        &mut self,
        step_index: u32,
        ordinal: usize,
        call: ToolCallData,
        record_ids: &mut Vec<String>,
        turn_indices: &mut Vec<u32>,
    ) {
        if let Some(call_id) = call.id.as_deref()
            && let Some(&record_index) = self.tool_record_indices.get(call_id)
        {
            let record = &mut self.records[record_index];
            if record.detail.tool_name.is_none() {
                record.detail.tool_name.clone_from(&call.name);
            }
            if record.detail.arguments.is_none() {
                record.detail.arguments.clone_from(&call.arguments);
            }
            record.detail.raw_fragments.push(call.raw);
            record.preview = tool_preview(
                record.detail.tool_name.as_deref(),
                record.detail.arguments.as_deref(),
            );
            return;
        }

        let id = call
            .id
            .as_deref()
            .map(|call_id| format!("tool:{call_id}"))
            .unwrap_or_else(|| format!("step:{step_index}:tool:{ordinal}"));
        let record = TrajectoryRecord {
            id,
            index: 0,
            kind: TrajectoryRecordKind::Tool,
            step_index,
            turn_index: self.current_turn,
            preview: tool_preview(call.name.as_deref(), call.arguments.as_deref()),
            started_ms: None,
            finished_ms: None,
            usage: MetricAvailability::Unavailable,
            metrics: TrajectoryMetrics::default(),
            detail: TrajectoryRecordDetail {
                tool_name: call.name,
                call_id: call.id.clone(),
                arguments: call.arguments,
                raw_fragments: vec![call.raw],
                ..TrajectoryRecordDetail::default()
            },
        };
        let record_index = self.push_record(record, record_ids, turn_indices);
        if let Some(call_id) = call.id {
            self.tool_record_indices.insert(call_id, record_index);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn attach_tool_result(
        &mut self,
        step_index: u32,
        message_index: usize,
        call_id: Option<String>,
        name: Option<String>,
        result: Option<String>,
        raw: Value,
        record_ids: &mut Vec<String>,
        turn_indices: &mut Vec<u32>,
    ) {
        if let Some(call_id) = call_id.as_deref()
            && let Some(&record_index) = self.tool_record_indices.get(call_id)
        {
            let record = &mut self.records[record_index];
            record.detail.result = result;
            record.detail.result_step_index = Some(step_index);
            record.detail.raw_fragments.push(raw);
            if record.detail.tool_name.is_none() {
                record.detail.tool_name = name;
            }
            return;
        }

        let id = call_id
            .as_deref()
            .map(|call_id| format!("tool:{call_id}"))
            .unwrap_or_else(|| format!("step:{step_index}:tool-result:{message_index}"));
        let preview = result
            .as_deref()
            .map(preview_text)
            .filter(|preview| !preview.is_empty())
            .unwrap_or_else(|| "Tool result".to_string());
        let record = TrajectoryRecord {
            id,
            index: 0,
            kind: TrajectoryRecordKind::Tool,
            step_index,
            turn_index: self.current_turn,
            preview,
            started_ms: None,
            finished_ms: None,
            usage: MetricAvailability::Unavailable,
            metrics: TrajectoryMetrics::default(),
            detail: TrajectoryRecordDetail {
                tool_name: name,
                call_id: call_id.clone(),
                result,
                result_step_index: Some(step_index),
                raw_fragments: vec![raw],
                ..TrajectoryRecordDetail::default()
            },
        };
        let record_index = self.push_record(record, record_ids, turn_indices);
        if let Some(call_id) = call_id {
            self.tool_record_indices.insert(call_id, record_index);
        }
    }

    fn push_record(
        &mut self,
        mut record: TrajectoryRecord,
        record_ids: &mut Vec<String>,
        turn_indices: &mut Vec<u32>,
    ) -> usize {
        record.index = self.records.len() + 1;
        if let Some(turn_index) = record.turn_index
            && !turn_indices.contains(&turn_index)
        {
            turn_indices.push(turn_index);
        }
        record_ids.push(record.id.clone());
        let index = self.records.len();
        self.records.push(record);
        index
    }
}

/// 将当前会话的步骤与原始载荷转换为稳定的只读轨迹模型。
pub(crate) fn project_trajectory(
    steps: &[ChatSessionStep],
    step_llm_raw: &HashMap<u32, Value>,
) -> TrajectoryProjection {
    let mut steps_by_index = BTreeMap::new();
    let mut indices = BTreeSet::new();
    for step in steps {
        indices.insert(step.index);
        steps_by_index.insert(step.index, step);
    }
    indices.extend(step_llm_raw.keys().copied());

    let mut builder = ProjectionBuilder::new();
    let projected_steps = indices
        .into_iter()
        .map(|index| {
            builder.project_step(
                index,
                steps_by_index.get(&index).copied(),
                step_llm_raw.get(&index),
            )
        })
        .collect::<Vec<_>>();
    let turns = build_turns(&builder.records, &projected_steps);
    let summary = build_summary(&builder.records, &projected_steps, &turns);
    TrajectoryProjection { records: builder.records, steps: projected_steps, turns, summary }
}

fn build_turns(records: &[TrajectoryRecord], steps: &[TrajectoryStep]) -> Vec<TrajectoryTurn> {
    let mut turns = BTreeMap::<u32, TrajectoryTurn>::new();
    for record in records {
        let Some(turn_index) = record.turn_index else {
            continue;
        };
        let turn = turns.entry(turn_index).or_insert_with(|| TrajectoryTurn {
            index: turn_index,
            user_record_id: None,
            step_indices: Vec::new(),
            record_ids: Vec::new(),
        });
        if record.kind == TrajectoryRecordKind::User && turn.user_record_id.is_none() {
            turn.user_record_id = Some(record.id.clone());
        }
        turn.record_ids.push(record.id.clone());
    }
    for step in steps {
        for turn_index in &step.turn_indices {
            let turn = turns.entry(*turn_index).or_insert_with(|| TrajectoryTurn {
                index: *turn_index,
                user_record_id: None,
                step_indices: Vec::new(),
                record_ids: Vec::new(),
            });
            if !turn.step_indices.contains(&step.index) {
                turn.step_indices.push(step.index);
            }
        }
    }
    turns.into_values().collect()
}

fn build_summary(
    records: &[TrajectoryRecord],
    steps: &[TrajectoryStep],
    turns: &[TrajectoryTurn],
) -> TrajectorySummary {
    let mut summary = TrajectorySummary {
        total_steps: steps.len(),
        total_turns: turns.len(),
        total_records: records.len(),
        ..TrajectorySummary::default()
    };
    for record in records {
        match record.kind {
            TrajectoryRecordKind::System => summary.system_records += 1,
            TrajectoryRecordKind::User => summary.user_records += 1,
            TrajectoryRecordKind::Assistant => summary.assistant_records += 1,
            TrajectoryRecordKind::Tool => summary.tool_records += 1,
        }
    }
    for step in steps {
        if let MetricAvailability::Available(usage) = step.usage {
            summary.usage.add_assign(usage);
        }
    }
    summary.elapsed_ms = elapsed_duration(steps);
    summary
}

fn elapsed_duration(steps: &[TrajectoryStep]) -> MetricAvailability<u64> {
    if steps.is_empty() {
        return MetricAvailability::Unavailable;
    }
    let mut earliest = u64::MAX;
    let mut latest = 0;
    for step in steps {
        let (Some(started_ms), Some(finished_ms)) = (step.started_ms, step.finished_ms) else {
            return MetricAvailability::Unavailable;
        };
        if finished_ms < started_ms {
            return MetricAvailability::Unavailable;
        }
        earliest = earliest.min(started_ms);
        latest = latest.max(finished_ms);
    }
    MetricAvailability::Available(latest.saturating_sub(earliest))
}

fn duration(started_ms: Option<u64>, finished_ms: Option<u64>) -> MetricAvailability<u64> {
    match (started_ms, finished_ms) {
        (Some(started_ms), Some(finished_ms)) if finished_ms >= started_ms => {
            MetricAvailability::Available(finished_ms - started_ms)
        }
        _ => MetricAvailability::Unavailable,
    }
}

fn usage_for(
    step: Option<&ChatSessionStep>,
    raw: Option<&Value>,
) -> MetricAvailability<TrajectoryUsage> {
    if let Some(step) = step {
        return MetricAvailability::Available(usage_from_step(&step.usage));
    }
    let Some(usage) = raw
        .and_then(|raw| raw.get("output"))
        .and_then(|output| output.get("usage"))
        .and_then(Value::as_object)
    else {
        return MetricAvailability::Unavailable;
    };
    let has_usage = ["input_tokens", "output_tokens", "cached_tokens", "reasoning_tokens"]
        .iter()
        .any(|key| usage.get(*key).and_then(nonnegative_integer).is_some());
    if !has_usage {
        return MetricAvailability::Unavailable;
    }
    MetricAvailability::Available(TrajectoryUsage::new(
        usage.get("input_tokens").and_then(nonnegative_integer).unwrap_or(0),
        usage.get("output_tokens").and_then(nonnegative_integer).unwrap_or(0),
        usage.get("cached_tokens").and_then(nonnegative_integer).unwrap_or(0),
        usage.get("reasoning_tokens").and_then(nonnegative_integer).unwrap_or(0),
    ))
}

fn usage_from_step(usage: &TokenUsage) -> TrajectoryUsage {
    TrajectoryUsage::new(
        usage.input_tokens.max(0) as u64,
        usage.output_tokens.max(0) as u64,
        usage.cached_tokens.max(0) as u64,
        usage.reasoning_tokens.max(0) as u64,
    )
}

fn nonnegative_integer(value: &Value) -> Option<u64> {
    value.as_u64().or_else(|| value.as_i64().and_then(|value| u64::try_from(value).ok()))
}

fn common_prefix_len(previous: &[Value], current: &[Value]) -> usize {
    previous.iter().zip(current).take_while(|(left, right)| left == right).count()
}

fn parse_tool_calls(value: Option<&Value>) -> Vec<ToolCallData> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|raw| {
            let object = raw.as_object()?;
            let function = object.get("function").and_then(Value::as_object);
            let id = string_alias(raw, &["id", "call_id", "callId"]);
            let name = function
                .and_then(|function| function.get("name"))
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
                .or_else(|| string_alias(raw, &["name", "tool_name", "toolName"]));
            let arguments = function
                .and_then(|function| function.get("arguments"))
                .or_else(|| object.get("arguments"))
                .and_then(value_as_detail_text);
            (id.is_some() || name.is_some()).then(|| ToolCallData {
                id,
                name,
                arguments,
                raw: raw.clone(),
            })
        })
        .collect()
}

fn string_alias(value: &Value, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|key| value.get(*key).and_then(Value::as_str))
        .filter(|value| !value.trim().is_empty())
        .map(ToOwned::to_owned)
}

fn value_as_detail_text(value: &Value) -> Option<String> {
    match value {
        Value::String(value) => Some(value.clone()),
        Value::Null => None,
        other => serde_json::to_string(other).ok(),
    }
}

fn value_text(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.clone()),
        Value::Array(blocks) => {
            let text = blocks
                .iter()
                .filter_map(|block| {
                    block.as_str().map(ToOwned::to_owned).or_else(|| {
                        block
                            .get("text")
                            .or_else(|| block.get("content"))
                            .and_then(Value::as_str)
                            .map(ToOwned::to_owned)
                    })
                })
                .collect::<Vec<_>>()
                .join("\n");
            (!text.is_empty()).then_some(text)
        }
        _ => None,
    }
}

fn assistant_preview(content: Option<&str>, reasoning: Option<&str>, tool_calls: usize) -> String {
    content
        .filter(|content| !content.trim().is_empty())
        .map(preview_text)
        .or_else(|| reasoning.filter(|reasoning| !reasoning.trim().is_empty()).map(preview_text))
        .unwrap_or_else(|| {
            if tool_calls == 1 {
                "1 tool call".to_string()
            } else if tool_calls > 1 {
                format!("{tool_calls} tool calls")
            } else {
                "Assistant response".to_string()
            }
        })
}

fn tool_preview(name: Option<&str>, arguments: Option<&str>) -> String {
    let name = name.filter(|name| !name.trim().is_empty()).unwrap_or("Tool call");
    arguments
        .filter(|arguments| !arguments.trim().is_empty())
        .map(|arguments| preview_text(&format!("{name} {arguments}")))
        .unwrap_or_else(|| name.to_string())
}

fn preview_text(text: &str) -> String {
    let compact = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let count = compact.chars().count();
    if count <= PREVIEW_CHAR_LIMIT {
        return compact;
    }
    let mut preview =
        compact.chars().take(PREVIEW_CHAR_LIMIT.saturating_sub(1)).collect::<String>();
    preview.push('…');
    preview
}

fn non_empty(value: &String) -> bool {
    !value.trim().is_empty()
}
