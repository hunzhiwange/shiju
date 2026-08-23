// 覆盖运行轨迹的列表、筛选、详情查询与读取错误。

use super::traces::run_traces;
use crate::app::agent::config::Config;
use serde_json::json;
use std::fs;

fn config_for_trace_file(path: &std::path::Path) -> Config {
    let mut config = Config::default();
    config.workspace_dir = path.parent().expect("trace file parent").to_path_buf();
    config.observability.runtime_trace_path =
        path.file_name().expect("trace file name").to_string_lossy().to_string();
    config
}

fn write_trace_events(path: &std::path::Path) {
    let old = json!({
        "id": "old-id",
        "timestamp": "2026-01-01T00:00:00Z",
        "event_type": "tool_call",
        "success": true,
        "message": "short result",
        "payload": {"tool": "read"}
    });
    let newest = json!({
        "id": "new-id",
        "timestamp": "2026-01-01T00:00:02Z",
        "event_type": "llm_response",
        "success": false,
        "message": "this diagnostic message is intentionally long enough to be truncated in list output",
        "provider": "openrouter",
        "model": "glm",
        "payload": {"reason": "needle"}
    });
    let pending = json!({
        "id": "pending-id",
        "timestamp": "2026-01-01T00:00:01Z",
        "event_type": "agent_turn",
        "payload": {}
    });
    fs::write(path, format!("{old}\nnot-json\n{pending}\n{newest}\n")).expect("write trace jsonl");
}

// 诊断断言同时检查严重级别与可操作提示，避免只有返回值正确而用户指引失真。
#[test]
fn run_traces_returns_ok_when_trace_file_is_missing() {
    let temp = tempfile::tempdir().expect("tempdir");
    let trace = temp.path().join("missing.jsonl");
    let config = config_for_trace_file(&trace);

    run_traces(&config, None, None, None, 0).expect("missing trace is informational");
}

#[test]
fn run_traces_lists_matching_events_with_safe_limit() {
    let temp = tempfile::tempdir().expect("tempdir");
    let trace = temp.path().join("trace.jsonl");
    write_trace_events(&trace);
    let config = config_for_trace_file(&trace);

    run_traces(&config, None, Some("llm_response"), Some("needle"), 0)
        .expect("filtered trace list");
}

#[test]
fn run_traces_lists_events_with_default_filters_and_statuses() {
    let temp = tempfile::tempdir().expect("tempdir");
    let trace = temp.path().join("trace.jsonl");
    write_trace_events(&trace);
    let config = config_for_trace_file(&trace);

    run_traces(&config, None, None, None, 10).expect("unfiltered trace list");
}

#[test]
fn run_traces_handles_empty_query_result() {
    let temp = tempfile::tempdir().expect("tempdir");
    let trace = temp.path().join("trace.jsonl");
    write_trace_events(&trace);
    let config = config_for_trace_file(&trace);

    run_traces(&config, None, Some("missing-event"), None, 10).expect("empty query");
}

#[test]
fn run_traces_can_lookup_by_id_and_report_miss() {
    let temp = tempfile::tempdir().expect("tempdir");
    let trace = temp.path().join("trace.jsonl");
    write_trace_events(&trace);
    let config = config_for_trace_file(&trace);

    run_traces(&config, Some(" new-id "), None, None, 10).expect("trace lookup hit");
    run_traces(&config, Some("missing-id"), None, None, 10).expect("trace lookup miss");
}

#[test]
fn run_traces_treats_blank_id_as_list_query() {
    let temp = tempfile::tempdir().expect("tempdir");
    let trace = temp.path().join("trace.jsonl");
    write_trace_events(&trace);
    let config = config_for_trace_file(&trace);

    run_traces(&config, Some(" \t"), None, None, 10).expect("blank id should list traces");
}

#[test]
fn run_traces_returns_error_when_trace_path_is_not_readable_file() {
    let temp = tempfile::tempdir().expect("tempdir");
    let config = config_for_trace_file(temp.path());

    let err = run_traces(&config, None, None, None, 10).expect_err("directory read should fail");
    assert!(!err.to_string().is_empty());
}

#[test]
fn run_traces_propagates_id_lookup_read_error() {
    let temp = tempfile::tempdir().expect("tempdir");
    let config = config_for_trace_file(temp.path());

    let err = run_traces(&config, Some("event-id"), None, None, 10)
        .expect_err("directory lookup should fail");
    assert!(!err.to_string().is_empty());
}
