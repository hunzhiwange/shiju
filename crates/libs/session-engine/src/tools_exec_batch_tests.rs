//! 批量结果汇总、错误与调用参数描述的局部契约。

use serde_json::{Value, json};

fn computed_success(tool: &str, output: Option<&str>) -> super::BatchComputed {
    super::BatchComputed {
        call_id: format!("call-{tool}"),
        tool: tool.to_string(),
        input_sanitized: "{}".to_string(),
        ui_payload: json!({}),
        session_message: format!("tool {tool}\n{{}}\n"),
        is_non_todo: true,
        success: true,
        output_for_model: output.map(str::to_string),
        error_for_model: None,
    }
}

fn computed_error(tool: &str, error: Option<&str>) -> super::BatchComputed {
    super::BatchComputed {
        call_id: format!("call-{tool}"),
        tool: tool.to_string(),
        input_sanitized: "{}".to_string(),
        ui_payload: json!({}),
        session_message: format!("tool {tool}\n{{}}\n"),
        is_non_todo: true,
        success: false,
        output_for_model: None,
        error_for_model: error.map(str::to_string),
    }
}

#[test]
fn batch_join_error_computed_formats_error_payload_for_ui_session_and_model() {
    let computed = super::batch_join_error_computed("worker panicked");

    assert_eq!(computed.tool, "batch");
    assert!(!computed.success);
    assert!(!computed.is_non_todo);
    assert!(computed.input_sanitized.is_empty());
    assert_eq!(computed.ui_payload["status"], "error");
    assert_eq!(computed.ui_payload["input"], "");
    assert!(computed.session_message.contains("\"status\":\"error\""));
    assert_eq!(
        computed.error_for_model.as_deref(),
        Some("Batch subtask join error: worker panicked")
    );
}

#[test]
fn batch_model_output_formats_empty_success_mixed_and_errorless_results() {
    assert_eq!(super::batch_model_output(0, &[]), "未执行任何子任务");

    let success = super::batch_model_output(
        3,
        &[
            computed_success("todoread", Some("  first output  ")),
            computed_success("empty-success", Some("   ")),
            computed_success("metadata-only", None),
        ],
    );
    assert!(success.contains("全部 3 个工具均执行成功"));
    assert!(success.contains("first output"));
    assert!(!success.contains("metadata-only"));

    let mixed = super::batch_model_output(
        4,
        &[
            computed_success("todoread", Some("ok")),
            computed_error("denied", Some("needs approval")),
            computed_error("silent-error", None),
        ],
    );
    assert!(mixed.contains("已成功执行 1/3 个工具，失败 2 个。"));
    assert!(mixed.contains("错误："));
    assert!(mixed.contains("- denied: needs approval"));
    assert!(!mixed.contains("silent-error:"));
}

#[test]
fn batch_ui_output_formats_empty_and_ran_lists() {
    assert_eq!(super::batch_ui_output(0, &[]), "未执行任何子任务");
    assert_eq!(
        super::batch_ui_output(2, &["- read src/lib.rs".to_string(), "- grep TODO".to_string()]),
        "已执行 2 个子任务：\n- read src/lib.rs\n- grep TODO"
    );
}

#[test]
fn describe_batch_call_formats_known_tools_and_falls_back_to_sanitized_json() {
    assert_eq!(
        super::describe_batch_call("bash", &json!({ "command": "echo ok" })),
        "- bash echo ok"
    );
    assert_eq!(
        super::describe_batch_call(
            "read",
            &json!({ "path": "src/lib.rs", "offset": -2, "limit": 10 })
        ),
        "- read src/lib.rs [offset=1, limit=10]"
    );
    assert_eq!(
        super::describe_batch_call("file_read", &json!({ "offset": 5 })),
        "- file_read [offset=5]"
    );
    assert_eq!(
        super::describe_batch_call("file_edit", &json!({ "file_path": "src/main.rs" })),
        "- file_edit src/main.rs"
    );
    assert_eq!(
        super::describe_batch_call("glob", &json!({ "pattern": "**/*.rs" })),
        "- glob **/*.rs"
    );
    assert_eq!(
        super::describe_batch_call("grep", &json!({ "pattern": "TODO", "path": "src" })),
        "- grep pattern=TODO path=src"
    );
    assert_eq!(
        super::describe_batch_call("grep", &json!({ "pattern": "TODO" })),
        "- grep pattern=TODO"
    );
    assert_eq!(super::describe_batch_call("grep", &json!({ "path": "src" })), "- grep path=src");
    assert_eq!(super::describe_batch_call("ls", &json!({ "path": "src" })), "- ls src");
    assert_eq!(super::describe_batch_call("custom", &json!({})), "- custom {}");
}

#[test]
fn describe_batch_call_handles_aliases_trimmed_values_and_missing_paths() {
    assert_eq!(
        super::describe_batch_call("shell", &json!({ "command": "  printf ok  " })),
        "- shell printf ok"
    );
    assert_eq!(
        super::describe_batch_call("read", &json!({ "filePath": "src/lib.rs" })),
        "- read src/lib.rs"
    );
    assert_eq!(
        super::describe_batch_call("read", &json!({ "file_path": "src/alias.rs" })),
        "- read src/alias.rs"
    );
    assert_eq!(
        super::describe_batch_call("file_read", &json!({ "offset": 0, "limit": -5 })),
        "- file_read [offset=1, limit=-5]"
    );
    assert_eq!(
        super::describe_batch_call("write", &json!({ "filePath": "camel.md" })),
        "- write camel.md"
    );
    assert_eq!(
        super::describe_batch_call("file_write", &json!({ "file_path": "snake.md" })),
        "- file_write snake.md"
    );
    for tool in ["write", "file_write", "apply_patch"] {
        assert_eq!(
            super::describe_batch_call(tool, &json!({ "path": "notes.md" })),
            format!("- {tool} notes.md")
        );
    }
    assert_eq!(super::describe_batch_call("custom", &Value::Null), "- custom null");
}

#[test]
fn describe_batch_call_covers_empty_string_values_and_unmatched_grep_details() {
    assert_eq!(
        super::describe_batch_call("shell", &json!({ "command": "   " })),
        "- shell {\"command\":\"   \"}"
    );
    assert_eq!(super::describe_batch_call("grep", &json!({})), "- grep {}");
    assert_eq!(super::describe_batch_call("ls", &json!({ "path": "" })), "- ls {\"path\":\"\"}");
    assert_eq!(
        super::describe_batch_call("glob", &json!({ "pattern": "   " })),
        "- glob {\"pattern\":\"   \"}"
    );
}
