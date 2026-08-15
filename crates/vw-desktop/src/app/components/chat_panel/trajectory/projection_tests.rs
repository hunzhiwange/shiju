use std::collections::HashMap;

use serde_json::{Value, json};
use vw_shared::session::ui_types::{ChatSessionStep, TokenUsage, ToolTiming};

use super::{MetricAvailability, TrajectoryRecordKind, TrajectoryUsage, project_trajectory};

fn step(index: u32, started_ms: u64, finished_ms: Option<u64>, usage: [i64; 4]) -> ChatSessionStep {
    ChatSessionStep {
        index,
        started_ms,
        finished_ms,
        start_snapshot_path: None,
        finish_snapshot_path: None,
        usage: TokenUsage {
            input_tokens: usage[0],
            output_tokens: usage[1],
            cached_tokens: usage[2],
            reasoning_tokens: usage[3],
        },
        cost_usd: None,
        finish_reason: None,
        model: Some("provider/model".to_string()),
        tool_timings: Vec::new(),
    }
}

fn raw_step(messages: Vec<Value>, text: &str, reasoning: &str, tool_calls: Vec<Value>) -> Value {
    json!({
        "system": ["You are helpful."],
        "messages": messages,
        "output": {
            "text": text,
            "reasoning_content": reasoning,
            "finish_reason": "stop",
            "tool_calls": tool_calls,
            "usage": {
                "input_tokens": 9,
                "output_tokens": 4,
                "cached_tokens": 2,
                "reasoning_tokens": 1
            }
        }
    })
}

fn tool_call(id: &str, name: &str, arguments: &str) -> Value {
    json!({
        "id": id,
        "type": "function",
        "function": {"name": name, "arguments": arguments}
    })
}

#[test]
fn trajectory_projection_deduplicates_cumulative_message_prefixes() {
    let steps =
        vec![step(1, 100, Some(500), [10, 4, 2, 1]), step(2, 600, Some(900), [20, 5, 3, 0])];
    let raw = HashMap::from([
        (
            1,
            raw_step(
                vec![json!({"role": "user", "content": "first question"})],
                "first answer",
                "first thought",
                Vec::new(),
            ),
        ),
        (
            2,
            raw_step(
                vec![
                    json!({"role": "user", "content": "first question"}),
                    json!({"role": "assistant", "content": "first answer"}),
                    json!({"role": "user", "content": "follow up"}),
                ],
                "second answer",
                "",
                Vec::new(),
            ),
        ),
    ]);

    let projection = project_trajectory(&steps, &raw);

    assert_eq!(
        projection.records.iter().map(|record| record.kind).collect::<Vec<_>>(),
        vec![
            TrajectoryRecordKind::System,
            TrajectoryRecordKind::User,
            TrajectoryRecordKind::Assistant,
            TrajectoryRecordKind::User,
            TrajectoryRecordKind::Assistant,
        ]
    );
    assert_eq!(projection.records[2].detail.reasoning.as_deref(), Some("first thought"));
    assert_eq!(projection.steps[0].record_ids.len(), 3);
    assert_eq!(projection.turns.len(), 2);
    assert!(projection.records[1].detail.source.is_none());
    assert_eq!(projection.summary.usage, TrajectoryUsage::new(30, 9, 5, 1));
    assert!(projection.records[4].matches_query("SECOND ANSWER"));
}

#[test]
fn trajectory_projection_pairs_tool_calls_with_later_results() {
    let call = tool_call("call-1", "weather", "{\"city\":\"Shanghai\"}");
    let steps = vec![step(1, 100, Some(300), [8, 1, 0, 0]), step(2, 400, Some(700), [12, 3, 0, 0])];
    let raw = HashMap::from([
        (
            1,
            raw_step(
                vec![json!({"role": "user", "content": "weather?"})],
                "",
                "",
                vec![call.clone()],
            ),
        ),
        (
            2,
            raw_step(
                vec![
                    json!({"role": "user", "content": "weather?"}),
                    json!({"role": "assistant", "content": null, "tool_calls": [call]}),
                    json!({"role": "tool", "tool_call_id": "call-1", "content": "sunny"}),
                ],
                "It is sunny.",
                "",
                Vec::new(),
            ),
        ),
    ]);

    let projection = project_trajectory(&steps, &raw);
    let tool = projection.record("tool:call-1").expect("paired tool record");

    assert_eq!(
        projection.records.iter().map(|record| record.kind).collect::<Vec<_>>(),
        vec![
            TrajectoryRecordKind::System,
            TrajectoryRecordKind::User,
            TrajectoryRecordKind::Assistant,
            TrajectoryRecordKind::Tool,
            TrajectoryRecordKind::Assistant,
        ]
    );
    assert_eq!(tool.detail.tool_name.as_deref(), Some("weather"));
    assert_eq!(tool.detail.call_id.as_deref(), Some("call-1"));
    assert_eq!(tool.detail.result.as_deref(), Some("sunny"));
    assert_eq!(tool.detail.result_step_index, Some(2));
    assert_eq!(tool.detail.raw_fragments.len(), 2);
}

#[test]
fn trajectory_projection_keeps_tool_only_assistant_records() {
    let steps = vec![step(7, 1_000, None, [4, 0, 0, 0])];
    let raw = HashMap::from([(
        7,
        raw_step(
            vec![json!({"role": "user", "content": "search"})],
            "",
            "",
            vec![tool_call("call-search", "search", "{\"q\":\"rust\"}")],
        ),
    )]);

    let projection = project_trajectory(&steps, &raw);
    let assistant = projection
        .records
        .iter()
        .find(|record| record.kind == TrajectoryRecordKind::Assistant)
        .expect("tool-only assistant record");

    assert_eq!(assistant.preview, "1 tool call");
    assert_eq!(assistant.detail.output.as_deref(), None);
    assert_eq!(projection.summary.assistant_records, 1);
    assert_eq!(projection.summary.tool_records, 1);
}

#[test]
fn trajectory_projection_degrades_missing_and_malformed_raw_without_panicking() {
    let steps = vec![
        step(1, 10, None, [0, 0, 0, 0]),
        step(2, 20, None, [0, 0, 0, 0]),
        step(3, 30, None, [0, 0, 0, 0]),
    ];
    let raw = HashMap::from([
        (2, json!("not an object")),
        (3, json!({"system": 7, "messages": "bad", "output": []})),
    ]);

    let projection = project_trajectory(&steps, &raw);

    assert!(projection.records.is_empty());
    assert_eq!(projection.steps.len(), 3);
    assert!(projection.turns.is_empty());
    assert_eq!(projection.summary.total_steps, 3);
    assert_eq!(projection.summary.total_records, 0);
}

#[test]
fn trajectory_projection_marks_unrecorded_metrics_unavailable() {
    let steps = vec![step(1, 1_000, Some(1_600), [9, 4, 2, 1])];
    let raw = HashMap::from([(
        1,
        raw_step(
            vec![json!({"role": "user", "content": "question"})],
            "answer",
            "thinking",
            Vec::new(),
        ),
    )]);

    let projection = project_trajectory(&steps, &raw);
    let assistant = projection
        .records
        .iter()
        .find(|record| record.kind == TrajectoryRecordKind::Assistant)
        .expect("assistant record");

    assert_eq!(assistant.metrics.duration_ms, MetricAvailability::Available(600));
    assert_eq!(assistant.metrics.ttft_ms, MetricAvailability::Unavailable);
    assert_eq!(assistant.metrics.tool_duration_ms, MetricAvailability::Unavailable);
    assert_eq!(assistant.metrics.generation_ms, MetricAvailability::Unavailable);
    assert_eq!(assistant.metrics.throughput_tokens_per_second, MetricAvailability::Unavailable);
}

#[test]
fn trajectory_projection_record_ids_are_stable_across_reload_and_input_order() {
    let steps = vec![step(2, 200, Some(300), [2, 1, 0, 0]), step(1, 100, Some(150), [1, 1, 0, 0])];
    let raw = HashMap::from([
        (
            2,
            raw_step(
                vec![
                    json!({"role": "user", "content": "one"}),
                    json!({"role": "assistant", "content": "first"}),
                    json!({"role": "user", "content": "two"}),
                ],
                "",
                "",
                vec![tool_call("stable-call", "read", "{}")],
            ),
        ),
        (1, raw_step(vec![json!({"role": "user", "content": "one"})], "first", "", Vec::new())),
    ]);

    let first = project_trajectory(&steps, &raw);
    let second = project_trajectory(&steps.iter().cloned().rev().collect::<Vec<_>>(), &raw);

    assert_eq!(
        first.records.iter().map(|record| record.id.as_str()).collect::<Vec<_>>(),
        second.records.iter().map(|record| record.id.as_str()).collect::<Vec<_>>()
    );
    assert!(first.record("step:1:assistant").is_some());
    assert!(first.record("tool:stable-call").is_some());
}

#[test]
fn projection_uses_persisted_tool_timing_by_call_id() {
    let mut persisted_step = step(1, 100, Some(500), [8, 1, 0, 0]);
    persisted_step.tool_timings = vec![
        ToolTiming {
            call_id: "call-b".to_string(),
            name: "write".to_string(),
            started_ms: Some(300),
            finished_ms: Some(450),
        },
        ToolTiming {
            call_id: "call-a".to_string(),
            name: "read".to_string(),
            started_ms: Some(180),
            finished_ms: Some(240),
        },
        ToolTiming {
            call_id: "call-partial".to_string(),
            name: "search".to_string(),
            started_ms: Some(250),
            finished_ms: None,
        },
        ToolTiming {
            call_id: "call-invalid".to_string(),
            name: "shell".to_string(),
            started_ms: Some(400),
            finished_ms: Some(350),
        },
    ];
    let raw = HashMap::from([(
        1,
        raw_step(
            vec![json!({"role": "user", "content": "run tools"})],
            "",
            "",
            vec![
                tool_call("call-a", "read", "{}"),
                tool_call("call-b", "write", "{}"),
                tool_call("call-partial", "search", "{}"),
                tool_call("call-invalid", "shell", "{}"),
                tool_call("call-missing", "fetch", "{}"),
            ],
        ),
    )]);

    let projection = project_trajectory(&[persisted_step], &raw);
    let call_a = projection.record("tool:call-a").expect("call-a tool record");
    assert_eq!(call_a.started_ms, Some(180));
    assert_eq!(call_a.finished_ms, Some(240));
    assert_eq!(call_a.metrics.duration_ms, MetricAvailability::Available(60));
    assert_eq!(call_a.metrics.tool_duration_ms, MetricAvailability::Available(60));
    assert_eq!(projection.record("tool:call-b").expect("call-b tool record").started_ms, Some(300));

    for call_id in ["call-partial", "call-invalid", "call-missing"] {
        let record = projection.record(&format!("tool:{call_id}")).expect("tool record");
        assert_eq!(record.metrics.duration_ms, MetricAvailability::Unavailable);
        assert_eq!(record.metrics.tool_duration_ms, MetricAvailability::Unavailable);
    }
}

#[test]
fn projection_classifies_user_and_context_from_message_source() {
    let mut raw = raw_step(
        vec![
            json!({"role": "user", "content": "skill catalog"}),
            json!({"role": "user", "content": "real question"}),
        ],
        "answer",
        "",
        Vec::new(),
    );
    raw["message_sources"] = json!([
        {"kind": "context", "form": "skill_catalog", "entries": ["rustcodegraph"]},
        {"kind": "user", "rpcId": "desktop-42", "clientTimeZone": "Asia/Shanghai"},
    ]);

    let projection =
        project_trajectory(&[step(1, 100, Some(200), [3, 2, 0, 0])], &HashMap::from([(1, raw)]));

    assert_eq!(
        projection.records.iter().map(|record| record.kind).collect::<Vec<_>>(),
        vec![
            TrajectoryRecordKind::System,
            TrajectoryRecordKind::Context,
            TrajectoryRecordKind::User,
            TrajectoryRecordKind::Assistant,
        ]
    );
    assert_eq!(projection.records[1].detail.source.as_ref().unwrap()["form"], "skill_catalog");
    assert_eq!(projection.records[2].detail.source.as_ref().unwrap()["rpcId"], "desktop-42");
    assert_eq!(projection.turns.len(), 1);
}

#[test]
fn projection_attaches_request_and_tool_inspection_facts() {
    let mut raw = raw_step(
        vec![json!({"role": "user", "content": "run shell"})],
        "",
        "",
        vec![tool_call("call-shell", "shell", "{\"command\":\"pwd\"}")],
    );
    raw["request"] = json!({
        "config": {"model": "provider/model", "temperature": 0.2},
        "tools": [{
            "id": "shell",
            "description": "Run a shell command",
            "input_schema": {
                "type": "object",
                "properties": {"command": {"type": "string"}},
                "required": ["command"],
            },
        }],
    });

    let projection =
        project_trajectory(&[step(1, 100, Some(200), [3, 2, 0, 0])], &HashMap::from([(1, raw)]));
    let system = projection
        .records
        .iter()
        .find(|record| record.kind == TrajectoryRecordKind::System)
        .expect("system record");
    let model = projection
        .records
        .iter()
        .find(|record| record.kind == TrajectoryRecordKind::Assistant)
        .expect("model record");
    let tool = projection.record("tool:call-shell").expect("tool record");

    assert_eq!(system.detail.request_tools[0].id, "shell");
    assert_eq!(system.detail.request_tools[0].description, "Run a shell command");
    assert_eq!(model.detail.request_options.as_ref().unwrap()["temperature"], 0.2);
    assert_eq!(tool.detail.tool_schema.as_ref().unwrap()["required"], json!(["command"]));
    assert_eq!(tool.detail.raw_fragments[0]["id"], "call-shell");
}

#[test]
fn detail_tabs_match_reference_for_every_record_kind() {
    let labels = |kind: TrajectoryRecordKind| {
        kind.detail_tabs().iter().map(|tab| tab.label()).collect::<Vec<_>>()
    };

    assert_eq!(labels(TrajectoryRecordKind::System), vec!["System Prompt", "Tools"]);
    assert_eq!(labels(TrajectoryRecordKind::User), vec!["Summary", "Preview", "Raw", "Source"]);
    assert_eq!(labels(TrajectoryRecordKind::Context), vec!["Summary", "Preview", "Raw", "Source"]);
    assert_eq!(
        labels(TrajectoryRecordKind::Assistant),
        vec!["Summary", "Options", "Usage", "Timing"]
    );
    assert_eq!(
        labels(TrajectoryRecordKind::Tool),
        vec!["Summary", "Payload", "Result", "Schema", "Timing"]
    );
}
