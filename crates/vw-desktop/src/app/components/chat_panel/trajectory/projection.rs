//! 原始步骤 JSON 到轨迹领域模型的纯投影。

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use serde_json::Value;
use vw_shared::session::ui_types::{ChatSessionStep, TokenUsage};

use super::model::{
    MetricAvailability, TrajectoryMetrics, TrajectoryProjection, TrajectoryRecord,
    TrajectoryRecordDetail, TrajectoryRecordKind, TrajectoryStep, TrajectorySummary,
    TrajectoryToolDefinition, TrajectoryTurn, TrajectoryUsage,
};

const PREVIEW_CHAR_LIMIT: usize = 160;

pub(crate) trait ProjectionRawKey {
    fn projection_key(&self) -> (String, u32);
}

impl ProjectionRawKey for u32 {
    fn projection_key(&self) -> (String, u32) {
        (format!("legacy-step-{self}"), *self)
    }
}

impl ProjectionRawKey for (String, String, u32) {
    fn projection_key(&self) -> (String, u32) {
        (self.1.clone(), self.2)
    }
}

#[derive(Debug, Clone)]
struct ToolCallData {
    id: Option<String>,
    name: Option<String>,
    arguments: Option<String>,
    raw: Value,
}

#[derive(Debug, Clone)]
struct ToolTimingData {
    name: String,
    started_ms: Option<u64>,
    finished_ms: Option<u64>,
}

#[derive(Debug, Clone)]
struct AssistantEcho {
    content: Option<String>,
}

#[derive(Debug, Clone, Default)]
struct RequestSnapshot {
    config: Option<Value>,
    tools: Vec<TrajectoryToolDefinition>,
}

#[derive(Default)]
struct ProjectionBuilder {
    records: Vec<TrajectoryRecord>,
    previous_messages: Vec<Value>,
    previous_message_sources: Vec<Value>,
    previous_system: Vec<Value>,
    seen_system: HashSet<String>,
    assistant_echoes: Vec<AssistantEcho>,
    tool_record_indices: HashMap<String, usize>,
    tool_timings: HashMap<String, ToolTimingData>,
    current_turn: Option<u32>,
    next_turn: u32,
}

impl ProjectionBuilder {
    fn new(steps: &[ChatSessionStep]) -> Self {
        let mut ordered_steps = steps.iter().collect::<Vec<_>>();
        ordered_steps.sort_by_key(|step| step.index);
        let mut tool_timings = HashMap::new();
        for step in ordered_steps {
            for timing in &step.tool_timings {
                tool_timings.entry(timing.call_id.clone()).or_insert_with(|| ToolTimingData {
                    name: timing.name.clone(),
                    started_ms: timing.started_ms,
                    finished_ms: timing.finished_ms,
                });
            }
        }
        Self { next_turn: 1, tool_timings, ..Self::default() }
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
            let request = parse_request_snapshot(raw.get("request"));
            self.project_system(
                step_index,
                raw.get("system"),
                &request.tools,
                &mut record_ids,
                &mut turn_indices,
            );
            self.project_messages(
                step_index,
                raw.get("messages"),
                raw.get("message_sources"),
                &request.tools,
                &mut record_ids,
                &mut turn_indices,
            );
            self.project_output(
                step_index,
                step,
                raw.get("output"),
                &request,
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
        request_tools: &[TrajectoryToolDefinition],
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
                    request_tools: request_tools.to_vec(),
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
        message_sources: Option<&Value>,
        request_tools: &[TrajectoryToolDefinition],
        record_ids: &mut Vec<String>,
        turn_indices: &mut Vec<u32>,
    ) {
        let Some(messages) = messages.and_then(Value::as_array) else {
            return;
        };
        let message_sources = aligned_message_sources(messages, message_sources);
        let prefix_len = common_message_prefix_len(
            &self.previous_messages,
            &self.previous_message_sources,
            messages,
            &message_sources,
        );
        for (offset, message) in messages[prefix_len..].iter().enumerate() {
            let message_index = prefix_len + offset;
            self.project_message(
                step_index,
                message_index,
                message,
                message_sources.get(message_index),
                request_tools,
                record_ids,
                turn_indices,
            );
        }
        self.previous_messages.clone_from(messages);
        self.previous_message_sources = message_sources;
    }

    fn project_message(
        &mut self,
        step_index: u32,
        message_index: usize,
        message: &Value,
        source: Option<&Value>,
        request_tools: &[TrajectoryToolDefinition],
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
                        request_tools: request_tools.to_vec(),
                        raw_fragments: vec![message.clone()],
                        ..TrajectoryRecordDetail::default()
                    },
                };
                self.push_record(record, record_ids, turn_indices);
            }
            "user" => {
                let source = source.filter(|source| !source.is_null()).cloned();
                let kind = if source
                    .as_ref()
                    .and_then(|source| source.get("kind"))
                    .and_then(Value::as_str)
                    == Some("user")
                    || source.is_none()
                {
                    TrajectoryRecordKind::User
                } else {
                    TrajectoryRecordKind::Context
                };
                let turn_index = if kind == TrajectoryRecordKind::User {
                    let turn = self.next_turn;
                    self.next_turn = self.next_turn.saturating_add(1);
                    self.current_turn = Some(turn);
                    Some(turn)
                } else {
                    None
                };
                let preview = content.as_deref().map(preview_text).filter(|text| !text.is_empty());
                let record = TrajectoryRecord {
                    id: format!("step:{step_index}:message:{message_index}:user"),
                    index: 0,
                    kind,
                    step_index,
                    turn_index,
                    preview: preview.unwrap_or_else(|| match kind {
                        TrajectoryRecordKind::Context => "Context message".to_string(),
                        _ => "User message".to_string(),
                    }),
                    started_ms: None,
                    finished_ms: None,
                    usage: MetricAvailability::Unavailable,
                    metrics: TrajectoryMetrics::default(),
                    detail: TrajectoryRecordDetail {
                        input: content,
                        source,
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
                    self.upsert_tool_call(
                        step_index,
                        ordinal,
                        call,
                        request_tools,
                        record_ids,
                        turn_indices,
                    );
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
        request: &RequestSnapshot,
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
                request_options: request.config.clone(),
                raw_fragments: vec![Value::Object(output_object.clone())],
                ..TrajectoryRecordDetail::default()
            },
        };
        self.push_record(record, record_ids, turn_indices);
        self.assistant_echoes.push(AssistantEcho { content });
        for (ordinal, call) in calls.into_iter().enumerate() {
            self.upsert_tool_call(
                step_index,
                ordinal,
                call,
                &request.tools,
                record_ids,
                turn_indices,
            );
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
        request_tools: &[TrajectoryToolDefinition],
        record_ids: &mut Vec<String>,
        turn_indices: &mut Vec<u32>,
    ) {
        if let Some(call_id) = call.id.as_deref()
            && let Some(&record_index) = self.tool_record_indices.get(call_id)
        {
            let timing = self.tool_timings.get(call_id).cloned();
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
            apply_tool_timing(record, timing.as_ref());
            return;
        }

        let timing = call.id.as_deref().and_then(|call_id| self.tool_timings.get(call_id)).cloned();
        let tool_name = call
            .name
            .or_else(|| timing.as_ref().map(|timing| timing.name.clone()).filter(non_empty));
        let tool_schema = tool_name.as_deref().and_then(|tool_name| {
            request_tools
                .iter()
                .find(|tool| tool.id == tool_name)
                .map(|tool| tool.input_schema.clone())
        });

        let id = call
            .id
            .as_deref()
            .map(|call_id| format!("tool:{call_id}"))
            .unwrap_or_else(|| format!("step:{step_index}:tool:{ordinal}"));
        let mut record = TrajectoryRecord {
            id,
            index: 0,
            kind: TrajectoryRecordKind::Tool,
            step_index,
            turn_index: self.current_turn,
            preview: tool_preview(tool_name.as_deref(), call.arguments.as_deref()),
            started_ms: None,
            finished_ms: None,
            usage: MetricAvailability::Unavailable,
            metrics: TrajectoryMetrics::default(),
            detail: TrajectoryRecordDetail {
                tool_name,
                call_id: call.id.clone(),
                arguments: call.arguments,
                tool_schema,
                raw_fragments: vec![call.raw],
                ..TrajectoryRecordDetail::default()
            },
        };
        apply_tool_timing(&mut record, timing.as_ref());
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
pub(crate) fn project_trajectory<K: ProjectionRawKey>(
    steps: &[ChatSessionStep],
    step_llm_raw: &HashMap<K, Value>,
) -> TrajectoryProjection {
    let mut steps_by_key = BTreeMap::new();
    for step in steps {
        let turn_id = step
            .turn_id
            .clone()
            .unwrap_or_else(|| format!("legacy-step-{}", step.index));
        steps_by_key.insert((turn_id, step.index), step);
    }
    let raw_by_key = step_llm_raw
        .iter()
        .map(|(key, raw)| (key.projection_key(), raw))
        .collect::<BTreeMap<_, _>>();
    let keys = steps_by_key.keys().chain(raw_by_key.keys()).cloned().collect::<BTreeSet<_>>();
    let mut ordered_keys = keys.into_iter().collect::<Vec<_>>();
    ordered_keys.sort_by_key(|key| {
        steps_by_key
            .get(key)
            .map_or((u64::MAX, key.1, key.0.clone()), |step| {
                (step.started_ms, step.index, key.0.clone())
            })
    });
    let duplicate_step_indices = ordered_keys
        .iter()
        .map(|(_, step_index)| *step_index)
        .filter(|step_index| {
            ordered_keys.iter().filter(|(_, candidate)| candidate == step_index).count() > 1
        })
        .collect::<BTreeSet<_>>();

    let mut builder = ProjectionBuilder::new(steps);
    let projected_steps = ordered_keys
        .into_iter()
        .map(|key| {
            let record_start = builder.records.len();
            let mut projected = builder.project_step(
                key.1,
                steps_by_key.get(&key).copied(),
                raw_by_key.get(&key).copied(),
            );
            if duplicate_step_indices.contains(&key.1) {
                let prefix = format!("turn:{}:", key.0);
                for record_id in &mut projected.record_ids {
                    record_id.insert_str(0, &prefix);
                }
                for record in &mut builder.records[record_start..] {
                    record.id.insert_str(0, &prefix);
                }
            }
            projected
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
            TrajectoryRecordKind::Context => summary.context_records += 1,
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

fn apply_tool_timing(record: &mut TrajectoryRecord, timing: Option<&ToolTimingData>) {
    let Some(timing) = timing else {
        return;
    };
    record.started_ms = timing.started_ms;
    record.finished_ms = timing.finished_ms;
    let duration_ms = duration(timing.started_ms, timing.finished_ms);
    record.metrics.duration_ms = duration_ms;
    record.metrics.tool_duration_ms = duration_ms;
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

fn aligned_message_sources(messages: &[Value], sources: Option<&Value>) -> Vec<Value> {
    let sources = sources.and_then(Value::as_array);
    messages
        .iter()
        .enumerate()
        .map(|(index, _)| {
            sources.and_then(|sources| sources.get(index)).cloned().unwrap_or(Value::Null)
        })
        .collect()
}

fn common_message_prefix_len(
    previous_messages: &[Value],
    previous_sources: &[Value],
    current_messages: &[Value],
    current_sources: &[Value],
) -> usize {
    previous_messages
        .iter()
        .zip(previous_sources)
        .zip(current_messages.iter().zip(current_sources))
        .take_while(|((previous_message, previous_source), (current_message, current_source))| {
            previous_message == current_message && previous_source == current_source
        })
        .count()
}

fn parse_request_snapshot(value: Option<&Value>) -> RequestSnapshot {
    let Some(request) = value.and_then(Value::as_object) else {
        return RequestSnapshot::default();
    };
    let config = request.get("config").filter(|value| value.is_object()).cloned();
    let tools = request
        .get("tools")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|tool| {
            let tool = tool.as_object()?;
            let id = tool.get("id")?.as_str()?.trim();
            if id.is_empty() {
                return None;
            }
            Some(TrajectoryToolDefinition {
                id: id.to_string(),
                description: tool
                    .get("description")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                input_schema: tool.get("input_schema").cloned().unwrap_or(Value::Null),
            })
        })
        .collect();
    RequestSnapshot { config, tools }
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
