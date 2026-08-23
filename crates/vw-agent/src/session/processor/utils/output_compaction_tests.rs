//! 验证工具输入脱敏、输出压缩、预算裁剪与界面摘要规则。
//! 脱敏与裁剪用 UTF-8、敏感字段和长输出夹具共同约束，防止日志泄密或截断乱码。

use serde_json::json;

#[test]
fn tool_fingerprint_depends_on_name_input_length_and_head() {
    let base = super::tool_fingerprint("read", "{}", "session message");

    assert_eq!(base, super::tool_fingerprint("read", "{}", "session message"));
    assert_ne!(base, super::tool_fingerprint("write", "{}", "session message"));
    assert_ne!(base, super::tool_fingerprint("read", r#"{"path":"a"}"#, "session message"));
    assert_ne!(base, super::tool_fingerprint("read", "{}", "different message"));
}

#[test]
fn streaming_tool_detection_matches_known_streaming_tools() {
    for name in ["bash", "shell", "grep", "read", "file_read", "pdf_read", "glob", "ls"] {
        assert!(super::is_streaming_tool(name), "{name} should stream");
    }
    assert!(!super::is_streaming_tool("todowrite"));
    assert!(!super::is_streaming_tool("batch"));
}

// 输入脱敏组同时约束日志视图和界面视图，二者允许保留的字段集合不同。
#[test]
fn sanitize_tool_input_handles_empty_raw_invalid_and_truncated_inputs() {
    assert_eq!(super::sanitize_tool_input("bash", "   "), "");
    assert_eq!(super::sanitize_tool_input("bash", "not json"), "not json");

    let long = "a".repeat(2100);
    let sanitized = super::sanitize_tool_input("bash", &long);
    assert!(sanitized.ends_with("…<truncated>"));
    assert!(sanitized.len() < long.len());

    let invalid_json = format!("{{\"command\":\"{}\"", "b".repeat(2100));
    assert!(super::sanitize_tool_input("bash", &invalid_json).ends_with("…<truncated>"));
}

#[test]
fn sanitize_tool_input_redacts_sensitive_log_fields_but_keeps_todo_content() {
    let input = json!({
        "path": "demo.md",
        "content": "hello",
        "prompt": "secret",
        "nested": { "body": "large" }
    })
    .to_string();

    let sanitized = super::sanitize_tool_input("file_write", &input);
    let parsed: serde_json::Value = serde_json::from_str(&sanitized).expect("json");

    assert_eq!(parsed["path"], "demo.md");
    assert_eq!(parsed["content"], "<omitted 5 chars>");
    assert_eq!(parsed["prompt"], "<omitted 6 chars>");
    assert_eq!(parsed["nested"]["body"], "<omitted 5 chars>");

    let todo = super::sanitize_tool_input("todowrite", r#"{"content":"keep me"}"#);
    let parsed_todo: serde_json::Value = serde_json::from_str(&todo).expect("todo json");
    assert_eq!(parsed_todo["content"], "keep me");
}

#[test]
fn sanitize_tool_input_for_ui_preserves_edit_fields_and_marks_remaining_array_items() {
    let input = json!({
        "oldString": "x".repeat(500),
        "newString": "y".repeat(500),
        "messages": (0..25).collect::<Vec<_>>(),
        "short": "ok"
    })
    .to_string();

    let sanitized = super::sanitize_tool_input_for_ui("file_edit", &input);
    let parsed: serde_json::Value = serde_json::from_str(&sanitized).expect("json");

    assert_eq!(parsed["oldString"].as_str().map(str::len), Some(500));
    assert_eq!(parsed["newString"].as_str().map(str::len), Some(500));
    assert_eq!(parsed["short"], "ok");
    assert_eq!(parsed["messages"].as_array().expect("array").len(), 21);
    assert_eq!(parsed["messages"][20]["_remaining_items"], 5);

    let patch = "*** Begin Patch\n*** End Patch";
    assert_eq!(super::sanitize_tool_input_for_ui("apply_patch", patch), patch);
}

#[test]
fn sanitize_tool_input_truncates_long_json_strings_on_char_boundaries() {
    let input = json!({ "value": "汉".repeat(300) }).to_string();
    let sanitized = super::sanitize_tool_input("bash", &input);
    let parsed: serde_json::Value = serde_json::from_str(&sanitized).expect("json");

    let value = parsed["value"].as_str().expect("value");
    assert!(value.contains("…<truncated 300 chars>"));
}

#[test]
fn truncate_string_respects_byte_limit_and_utf8_boundaries() {
    assert_eq!(super::truncate_string("short", 10), "short");

    let truncated = super::truncate_string("éééé", 5);
    assert!(truncated.ends_with("…<truncated>"));
    assert!(truncated.starts_with("éé"));
}

#[test]
fn compact_tool_output_compacts_file_links_and_uses_tail_for_bash() {
    let with_link = concat!(
        "<file_link>\n",
        "path: docs/readme.md\n",
        "open: file:///tmp/readme.md\n",
        "size_bytes: 10\n",
        "</file_link>\n",
        "visible body"
    );
    let compacted = super::compact_tool_output("read", with_link);
    assert!(compacted.starts_with("path: docs/readme.md"));
    assert!(!compacted.contains("open:"));

    let long = format!("{}TAIL", "a".repeat(60 * 1024));
    let bash = super::compact_tool_output("bash", &long);
    assert!(bash.starts_with("…<truncated>"));
    assert!(bash.ends_with("TAIL"));

    let other = super::compact_tool_output("grep", &"b".repeat(4 * 1024));
    assert!(other.ends_with("…<truncated>"));
}

#[test]
fn compact_tool_output_keeps_short_bash_output_unchanged() {
    let output = "command completed\nshort output";

    assert_eq!(super::compact_tool_output("bash", output), output);
}

// 代理工具结果采用独立预算策略，研究报告与多任务摘要都要保留可识别的首尾结构。
#[test]
fn agent_tool_result_budget_preserves_single_research_report_head_and_tail() {
    let report = format!(
        "Conclusion: stale lifecycle state.\nSource: https://example.test/source\n{}\nUncertainty: reconnect needs validation.",
        "evidence ".repeat(4_000)
    );
    let output = json!({
        "action": "run",
        "state": "succeeded",
        "child_session_id": "research-child",
        "output": report,
    })
    .to_string();

    let compacted = super::compact_tool_output("AgentTool", &output);
    let parsed: serde_json::Value = serde_json::from_str(&compacted).expect("valid compacted json");
    let report = parsed["output"].as_str().expect("report");
    assert!(report.starts_with("Conclusion:"));
    assert!(report.contains("https://example.test/source"));
    assert!(report.contains("<middle truncated>"));
    assert!(report.ends_with("Uncertainty: reconnect needs validation."));
    assert_eq!(parsed["child_session_id"], "research-child");
}

#[test]
fn agent_tool_result_budget_preserves_every_multi_task_summary() {
    let tasks = (0..12)
        .map(|index| {
            json!({
                "parent_session_id": "parent",
                "child_session_id": format!("child-{index}"),
                "state": "succeeded",
                "output": format!(
                    "Task {index} result.\n{}\nTask {index} validation passed.",
                    "details ".repeat(500)
                )
            })
        })
        .collect::<Vec<_>>();
    let output = json!({"action":"wait","timed_out":false,"tasks":tasks}).to_string();

    let compacted = super::compact_tool_output("agent_tool", &output);
    let parsed: serde_json::Value = serde_json::from_str(&compacted).expect("valid compacted json");
    let tasks = parsed["tasks"].as_array().expect("tasks");
    assert_eq!(tasks.len(), 12);
    for (index, task) in tasks.iter().enumerate() {
        let report = task["output"].as_str().expect("task report");
        assert!(report.starts_with(&format!("Task {index} result.")));
        assert!(report.ends_with(&format!("Task {index} validation passed.")));
        assert_eq!(task["child_session_id"], format!("child-{index}"));
    }
}

#[test]
fn agent_tool_result_budget_invalid_json_uses_generic_fallback() {
    let invalid = format!("not-json {}", "x".repeat(4 * 1024));
    let compacted = super::compact_tool_output("agent", &invalid);

    assert!(compacted.ends_with("…<truncated>"));
    assert!(compacted.len() < invalid.len());
}

#[test]
fn compact_tool_output_for_ui_has_read_specific_rules() {
    let file = format!("<file>{}</file>", "a".repeat(17 * 1024));
    let compact_file = super::compact_tool_output_for_ui("read", &file);
    assert!(compact_file.ends_with("…<truncated>"));

    let with_link =
        concat!("<file_link>\npath: a.md\n</file_link>\n", "内容已隐藏，点击文件名打开");
    let compact_link = super::compact_tool_output_for_ui("file_read", with_link);
    assert!(compact_link.contains("<file_link>"));
    assert!(!compact_link.contains("内容已隐藏"));

    let plain = super::compact_tool_output_for_ui("pdf_read", &"x".repeat(700));
    assert!(plain.ends_with("…<truncated>"));

    let unchanged = "x".repeat(700);
    assert_eq!(super::compact_tool_output_for_ui("bash", &unchanged), unchanged);
}

#[test]
fn rewrite_todowrite_completed_when_no_work_handles_noops_and_rewrites() {
    assert_eq!(super::rewrite_todowrite_completed_when_no_work(""), "");
    assert_eq!(super::rewrite_todowrite_completed_when_no_work("not json"), "not json");
    assert_eq!(super::rewrite_todowrite_completed_when_no_work("{bad"), "{bad");
    assert_eq!(
        super::rewrite_todowrite_completed_when_no_work(r#"{"merge":true}"#),
        r#"{"merge":true}"#
    );
    assert_eq!(
        super::rewrite_todowrite_completed_when_no_work(r#"{"todos":[{"status":"pending"}]}"#),
        r#"{"todos":[{"status":"pending"}]}"#
    );

    let pending = super::rewrite_todowrite_completed_when_no_work(
        r#"{"todos":[{"id":"1","status":"completed"},42,{"id":"2"}]}"#,
    );
    let pending: serde_json::Value = serde_json::from_str(&pending).expect("pending json");
    assert_eq!(pending["todos"][0]["status"], "pending");
    assert_eq!(pending["todos"][1].as_i64(), Some(42));

    let in_progress = super::rewrite_todowrite_completed_when_no_work(
        r#"{"merge":true,"todos":[{"id":"1","status":"completed"}]}"#,
    );
    let in_progress: serde_json::Value = serde_json::from_str(&in_progress).expect("progress json");
    assert_eq!(in_progress["todos"][0]["status"], "in_progress");
}
