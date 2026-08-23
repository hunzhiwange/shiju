//! 验证工具块身份、输入输出、代理批次及路径字段的兼容解析规则。
//! 多种旧字段别名只在明确位置接受，特殊工具隐藏和项目路径拼接保持保守边界。

use super::tool_parse::{
    AgentToolSessionStatus, ExploreToolKind, agent_tool_session_payloads_from_content,
    explore_item_dedupe_key, explore_tool_kind, is_explore_tool, should_hide_tool_block,
    tool_call_id_from_raw, tool_change_file_summaries, tool_change_files, tool_error_text,
    tool_identity_from_raw, tool_input, tool_input_path, tool_name_from_raw, tool_output_path,
    tool_output_text, tool_render_hint_metadata, tool_result_data, tool_status,
    tool_status_from_raw, tool_structured_diff_text, tool_summary_text,
};
use crate::app::App;
use crate::app::components::chat_panel::tools::types::ChangeFileSummary;

#[test]
fn tool_name_and_call_id_are_read_from_raw_tool_block() {
    let raw = r#"tool bash
{"call_id":"call-1","status":"completed","input":"echo ok"}"#;

    assert_eq!(tool_name_from_raw(raw), Some("bash".to_string()));
    assert_eq!(tool_call_id_from_raw(raw), Some("call-1".to_string()));
    assert_eq!(tool_status_from_raw(raw), Some("completed".to_string()));
}

#[test]
fn tool_parse_agent_tool_session_payloads_normalize_supported_shapes() {
    let cases = [
        (
            r#"tool Agent
{"status":"running","metadata":{"parent_session_id":" parent ","child_session_id":" child-running ","agent":" writer ","task":" inspect "}}"#,
            AgentToolSessionStatus::Running,
            "child-running",
            Some("parent"),
        ),
        (
            r#"tool AgentTool
{"status":"completed","output":"{\"child_session_id\":\"child-completed\",\"agent\":\"reviewer\",\"task\":\"review\"}"}"#,
            AgentToolSessionStatus::Completed,
            "child-completed",
            None,
        ),
        (
            r#"tool agent_tool
{"status":"error","output":"{\"session_id\":\"child-failed\",\"agent\":\"tester\",\"task\":\"test\"}"}"#,
            AgentToolSessionStatus::Failed,
            "child-failed",
            None,
        ),
        (
            r#"tool AgentTool
{"status":"waiting","metadata":{"child_session_id":"child-waiting"}}"#,
            AgentToolSessionStatus::Waiting,
            "child-waiting",
            None,
        ),
        (
            r#"tool AgentTool
{"status":"cancelled","metadata":{"child_session_id":"child-cancelled"}}"#,
            AgentToolSessionStatus::Cancelled,
            "child-cancelled",
            None,
        ),
    ];

    for (raw, status, child_session_id, parent_session_id) in cases {
        let parsed = agent_tool_session_payloads_from_content(raw);
        assert_eq!(parsed.len(), 1, "{raw}");
        assert_eq!(parsed[0].status, status, "{raw}");
        assert_eq!(parsed[0].child_session_id, child_session_id, "{raw}");
        assert_eq!(parsed[0].parent_session_id.as_deref(), parent_session_id, "{raw}");
    }

    let ignored = [
        "tool bash\n{}",
        "tool AgentTool\nnot-json",
        r#"tool AgentTool
{"status":"running","metadata":{"child_session_id":"   "}}"#,
        r#"tool AgentTool
{"status":"unknown","metadata":{"child_session_id":"child"}}"#,
    ];
    for raw in ignored {
        assert!(agent_tool_session_payloads_from_content(raw).is_empty(), "{raw}");
    }
}

#[test]
fn agent_tool_batch_lifecycle_parser_expands_mixed_task_snapshots() {
    let raw = r#"tool AgentTool
{"status":"waiting","output":"{\"action\":\"wait\",\"timed_out\":true,\"tasks\":[{\"parent_session_id\":\"parent\",\"child_session_id\":\"done\",\"agent\":\"researcher\",\"task\":\"search\",\"state\":\"succeeded\"},{\"parent_session_id\":\"parent\",\"child_session_id\":\"failed\",\"agent\":\"tester\",\"task\":\"verify\",\"state\":\"failed\"},{\"parent_session_id\":\"parent\",\"child_session_id\":\"cancelled\",\"agent\":\"writer\",\"task\":\"edit\",\"state\":\"cancelled\"},{\"parent_session_id\":\"parent\",\"child_session_id\":\"active\",\"agent\":\"reviewer\",\"task\":\"review\",\"state\":\"running\"}]}"}"#;

    let parsed = agent_tool_session_payloads_from_content(raw);
    assert_eq!(parsed.len(), 4);
    assert_eq!(
        parsed.iter().map(|payload| payload.status).collect::<Vec<_>>(),
        vec![
            AgentToolSessionStatus::Completed,
            AgentToolSessionStatus::Failed,
            AgentToolSessionStatus::Cancelled,
            AgentToolSessionStatus::Waiting,
        ]
    );
    assert_eq!(
        parsed.iter().map(|payload| payload.child_session_id.as_str()).collect::<Vec<_>>(),
        vec!["done", "failed", "cancelled", "active"]
    );

    let status_raw = raw.replace(r#"\"action\":\"wait\""#, r#"\"action\":\"status\""#);
    let status = agent_tool_session_payloads_from_content(&status_raw);
    assert_eq!(status[3].status, AgentToolSessionStatus::Running);
}

#[test]
fn explore_tool_kind_classifies_read_like_tools() {
    assert!(is_explore_tool("read"));
    assert!(explore_tool_kind("read").is_some());
    assert!(
        explore_item_dedupe_key(
            r#"tool read
{"input":"{\"filePath\":\"src/main.rs\"}"}"#
        )
        .is_some()
    );
}

#[test]
fn tool_input_accepts_raw_input_alias() {
    let value = serde_json::json!({
        "rawInput": "{\"command\":\"date\"}"
    });

    assert_eq!(tool_input(&value), "{\"command\":\"date\"}");
}

#[test]
fn tool_input_path_accepts_claude_file_path_alias() {
    assert_eq!(
        tool_input_path(r#"{"file_path":"docs/demo.md"}"#),
        Some("docs/demo.md".to_string())
    );
}

#[test]
fn tool_name_prefers_canonical_id_from_render_hint() {
    let raw = r#"tool Read
{"renderHint":{"metadata":{"canonical_tool_id":"file_read"}}}"#;
    let nested_raw = r#"tool search
{"result":{"renderHint":{"metadata":{"canonical_tool_id":"grep_search"}}}}"#;

    assert_eq!(tool_name_from_raw(raw), Some("read".to_string()));
    assert_eq!(tool_name_from_raw(nested_raw), Some("grep_search".to_string()));
}

#[test]
fn tool_call_id_and_summary_are_found_in_nested_locations() {
    let raw = r#"tool read
{"result":{"metadata":{"toolUseId":"call-2"}},"render_hint":{"summary":"peek file"}}"#;

    assert_eq!(tool_call_id_from_raw(raw), Some("call-2".to_string()));
    assert_eq!(tool_status_from_raw(raw), Some("".to_string()));

    let value = serde_json::json!({
        "render_hint": {"summary": "peek file"},
        "result": {"success": true}
    });
    assert_eq!(tool_summary_text(&value).as_deref(), Some("peek file"));
    assert_eq!(tool_status(&value), "completed");
}

#[test]
fn tool_output_helpers_fall_back_to_structured_result_content() {
    let value = serde_json::json!({
        "result": {
            "content": [
                {
                    "type": "structured_patch",
                    "hunks": [
                        {
                            "path": "src/main.rs",
                            "header": "@@ -1 +1 @@",
                            "lines": ["-old", "+new"]
                        }
                    ]
                }
            ]
        }
    });

    let diff = tool_structured_diff_text(&value).expect("diff text");
    assert!(diff.contains("--- a/src/main.rs"));
    assert!(tool_output_text(&value).expect("output").contains("+new"));
    assert_eq!(
        tool_change_file_summaries(&value),
        vec![ChangeFileSummary {
            kind: 'M',
            path: "src/main.rs".to_string(),
            additions: 1,
            deletions: 1,
        }]
    );
}

#[test]
fn tool_identity_and_hide_rules_cover_special_cases() {
    let bash_raw = r#"tool bash
{"input":"{\"command\":\"cargo test\"}"}"#;
    let grep_raw = r#"tool grep
{"input":"{\"pattern\":\"main\",\"include\":\"*.rs\",\"path\":\"src\"}"}"#;
    let todo_raw = r#"tool todowrite
{"status":"completed"}"#;
    let todo_read_raw = r#"tool todoread
{"status":"completed"}"#;

    assert_eq!(tool_identity_from_raw(bash_raw).as_deref(), Some("bash:cargo test"));
    assert_eq!(tool_identity_from_raw(grep_raw).as_deref(), Some("grep:main|*.rs|src"));
    assert!(should_hide_tool_block(todo_raw));
    assert!(should_hide_tool_block(todo_read_raw));
}

#[test]
fn output_path_and_error_text_handle_aliases() {
    let value = serde_json::json!({
        "status": "error",
        "output": "permission denied",
        "renderHint": { "metadata": { "outputPath": "[main](/tmp/out.rs#L7)" } }
    });

    assert_eq!(tool_output_path(&value).as_deref(), Some("/tmp/out.rs"));
    assert_eq!(tool_error_text(&value).as_deref(), Some("permission denied"));
}

#[test]
fn resolve_output_path_joins_project_root_for_relative_values() {
    let mut app = App::new().0;
    app.project_path = Some("/tmp/demo".to_string());

    assert_eq!(super::tool_parse::resolve_output_path(&app, "file:///tmp/out.rs"), "tmp/out.rs");
    assert_eq!(
        super::tool_parse::resolve_output_path(&app, "logs/out.txt"),
        "/tmp/demo/logs/out.txt"
    );
}

#[test]
fn raw_tool_helpers_reject_malformed_blocks() {
    assert_eq!(tool_name_from_raw("tool bash"), None);
    assert_eq!(tool_name_from_raw("tool \n{}"), None);
    assert_eq!(tool_call_id_from_raw("tool read\nnot json"), None);
    assert_eq!(tool_status_from_raw("tool read\nnot json"), None);
    assert_eq!(tool_identity_from_raw("read\n{}"), None);
    assert_eq!(tool_identity_from_raw("tool read\nnot json"), None);
    assert!(!should_hide_tool_block("tool todowrite"));
    assert!(!should_hide_tool_block("bad todowrite\n{\"status\":\"completed\"}"));
    assert!(!should_hide_tool_block("tool read\n{\"status\":\"completed\"}"));
    assert!(!should_hide_tool_block("tool todowrite\nnot json"));
    assert!(!should_hide_tool_block("tool todowrite\n{\"status\":\"running\"}"));
}

#[test]
fn call_id_is_found_across_supported_metadata_shapes() {
    let cases = [
        (
            r#"tool read
{"metadata":{"toolCallId":"meta-call"}}"#,
            "meta-call",
        ),
        (
            r#"tool read
{"renderHint":{"callId":"hint-call"}}"#,
            "hint-call",
        ),
        (
            r#"tool read
{"renderHint":{"metadata":{"tool_use_id":"hint-meta-call"}}}"#,
            "hint-meta-call",
        ),
        (
            r#"tool read
{"result":{"call_id":"result-call"}}"#,
            "result-call",
        ),
        (
            r#"tool read
{"result":{"metadata":{"toolUseId":"result-meta-call"}}}"#,
            "result-meta-call",
        ),
    ];

    for (raw, expected) in cases {
        assert_eq!(tool_call_id_from_raw(raw).as_deref(), Some(expected));
        assert_eq!(explore_item_dedupe_key(raw), Some(format!("call:{expected}")));
    }
}

#[test]
fn explore_tool_kind_covers_search_list_and_unknown_tools() {
    assert_eq!(explore_tool_kind("file_read"), Some(ExploreToolKind::Read));
    assert_eq!(explore_tool_kind("grep_search"), Some(ExploreToolKind::Search));
    assert_eq!(explore_tool_kind("list_dir"), Some(ExploreToolKind::List));
    assert_eq!(explore_tool_kind("bash"), None);
}

#[test]
fn result_data_and_render_metadata_prefer_supported_locations() {
    let value = serde_json::json!({
        "data": {"top": true},
        "result": {
            "data": {"nested": true},
            "renderHint": {
                "metadata": {"label": "nested hint"}
            }
        }
    });
    let fallback = serde_json::json!({
        "data": {"top": true},
        "render_hint": {
            "metadata": {"label": "top hint"}
        }
    });

    assert_eq!(
        tool_result_data(&value).and_then(|v| v.get("nested")).and_then(|v| v.as_bool()),
        Some(true)
    );
    assert_eq!(
        tool_render_hint_metadata(&value)
            .and_then(|metadata| metadata.get("label"))
            .and_then(|label| label.as_str()),
        Some("nested hint")
    );
    assert_eq!(
        tool_result_data(&fallback).and_then(|v| v.get("top")).and_then(|v| v.as_bool()),
        Some(true)
    );
    assert_eq!(
        tool_render_hint_metadata(&fallback)
            .and_then(|metadata| metadata.get("label"))
            .and_then(|label| label.as_str()),
        Some("top hint")
    );
}

#[test]
fn output_text_extracts_nested_content_values() {
    let content_value = serde_json::json!({
        "result": {
            "content": [
                {"type": "image", "text": "ignored"},
                {"type": "text", "text": "   "},
                {"type": "json", "value": [null, {"stderr": "from stderr"}]}
            ]
        }
    });
    let model_value = serde_json::json!({
        "result": {"model_result": {"value": 42}}
    });
    let data_value = serde_json::json!({
        "result": {"data": {"message": false}}
    });

    assert_eq!(tool_output_text(&content_value).as_deref(), Some("from stderr"));
    assert_eq!(tool_output_text(&model_value).as_deref(), Some("42"));
    assert_eq!(tool_output_text(&data_value).as_deref(), Some("false"));
}

#[test]
fn output_summary_and_error_helpers_skip_blank_values() {
    let summary = serde_json::json!({
        "summary": "  direct summary  ",
        "renderHint": {"summary": "fallback"}
    });
    let no_error = serde_json::json!({
        "status": "completed",
        "output": "done"
    });
    let blank_output = serde_json::json!({
        "output": "   ",
        "result": {"success": false}
    });

    assert_eq!(tool_summary_text(&summary).as_deref(), Some("direct summary"));
    assert_eq!(tool_error_text(&no_error), None);
    assert_eq!(tool_output_text(&blank_output), None);
    assert_eq!(tool_error_text(&blank_output), None);
}

#[test]
fn output_path_accepts_top_level_metadata() {
    let value = serde_json::json!({
        "metadata": {
            "outputPath": "`file:///tmp/out.rs#L12`"
        }
    });

    assert_eq!(tool_output_path(&value).as_deref(), Some("tmp/out.rs"));
}

#[test]
fn output_path_accepts_result_render_hint_metadata() {
    let value = serde_json::json!({
        "result": {
            "renderHint": {
                "metadata": {
                    "outputPath": "[main](src/generated.rs#L3)"
                }
            }
        }
    });

    assert_eq!(tool_output_path(&value).as_deref(), Some("src/generated.rs"));
}

#[test]
fn structured_patch_files_cover_add_delete_and_merged_hunks() {
    let value = serde_json::json!({
        "result": {
            "content": [
                {
                    "type": "structured_patch",
                    "hunks": [
                        {"path": "src/add.rs", "lines": ["+new"]},
                        {"path": "src/delete.rs", "lines": ["-old"]},
                        {"path": "src/edit.rs", "header": "@@ -1 +1 @@", "lines": [" context", "-old"]},
                        {"path": "src/edit.rs", "header": "@@ -4 +4 @@", "lines": ["+new", "same"]}
                    ]
                }
            ]
        }
    });

    let summaries = tool_change_file_summaries(&value);
    assert_eq!(summaries[0].kind, 'A');
    assert_eq!(summaries[1].kind, 'D');
    assert_eq!(summaries[2].kind, 'M');
    assert_eq!(summaries[2].additions, 1);
    assert_eq!(summaries[2].deletions, 1);

    let files = tool_change_files(&value);
    let edited = files.iter().find(|file| file.path == "src/edit.rs").expect("edit file");
    assert_eq!(edited.before, "context\nold\nsame\n");
    assert_eq!(edited.after, "context\nnew\nsame\n");

    let diff = tool_structured_diff_text(&value).expect("diff");
    assert_eq!(diff.matches("--- a/src/edit.rs").count(), 1);
    assert!(diff.contains("@@ -4 +4 @@\n+new\nsame\n"));
}

#[test]
fn malformed_structured_patch_blocks_are_ignored() {
    let value = serde_json::json!({
        "result": {
            "content": [
                {"type": "text", "text": "ignored"},
                {"type": "structured_patch"},
                {"type": "structured_patch", "hunks": [
                    {"path": "   ", "lines": ["+missing"]},
                    {"lines": ["+missing path"]},
                    {"path": "src/ok.rs", "lines": ["+kept", 3, null]}
                ]}
            ]
        }
    });

    let files = tool_change_files(&value);
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].path, "src/ok.rs");
    assert_eq!(files[0].after, "kept\n");
}

#[test]
fn tool_identity_covers_edit_paths_and_default_inputs() {
    let write_raw = r#"tool write
{"input":"{\"path\":\"docs/new.md\"}"}"#;
    let edit_raw = r#"tool edit
{"input":"","metadata":{"path":"[main](src/main.rs#L5)"}}"#;
    let notebook_raw = r#"tool notebook_edit
{"input":"plain.ipynb"}"#;
    let custom_raw = r#"tool custom
{"input":" payload "}"#;
    let bash_bad_json_raw = r#"tool bash
{"input":"{\"cmd\":\"missing command\"}"}"#;
    let grep_plain_raw = r#"tool grep
{"input":"needle"}"#;

    assert_eq!(tool_identity_from_raw(write_raw).as_deref(), Some("file_write:docs/new.md"));
    assert_eq!(tool_identity_from_raw(edit_raw).as_deref(), Some("file_edit:src/main.rs"));
    assert_eq!(tool_identity_from_raw(notebook_raw).as_deref(), Some("notebook_edit:plain.ipynb"));
    assert_eq!(tool_identity_from_raw(custom_raw).as_deref(), Some("custom:payload"));
    assert_eq!(tool_identity_from_raw(bash_bad_json_raw).as_deref(), Some("bash:"));
    assert_eq!(tool_identity_from_raw(grep_plain_raw).as_deref(), Some("grep:needle"));
}

#[test]
fn tool_input_path_handles_invalid_and_plain_inputs() {
    assert_eq!(tool_input_path(r#"{"path":""}"#), None);
    assert_eq!(tool_input_path(r#"{"other":"src/main.rs"}"#), None);
    assert_eq!(tool_input_path("{not json"), None);
    assert_eq!(tool_input_path("  'file:///tmp/demo.rs#line-8'  ").as_deref(), Some("tmp/demo.rs"));
    assert_eq!(tool_input_path("   "), None);
}

#[test]
fn resolve_output_path_handles_absolute_and_missing_project_root() {
    let mut app = App::new().0;
    app.project_path = None;

    assert_eq!(super::tool_parse::resolve_output_path(&app, "/tmp/out.rs"), "/tmp/out.rs");
    assert_eq!(super::tool_parse::resolve_output_path(&app, "logs/out.txt"), "logs/out.txt");
}

#[test]
fn raw_name_and_identity_handle_edge_inputs_without_losing_valid_blocks() {
    assert_eq!(tool_name_from_raw("tool shell\nnot-json"), Some("bash".to_string()));
    assert_eq!(tool_identity_from_raw("tool \n{}"), None);
    assert_eq!(
        tool_identity_from_raw("tool bash\n{\"input\":\"  printf ok  \"}").as_deref(),
        Some("bash:printf ok")
    );
    assert_eq!(
        explore_item_dedupe_key("tool custom\n{\"input\":\" payload \"}").as_deref(),
        Some("identity:custom:payload")
    );
}

#[test]
fn output_text_uses_non_blank_text_content_and_failed_result_status() {
    let value = serde_json::json!({
        "result": {
            "success": false,
            "content": [
                {"type": "text", "text": "rendered failure"},
                {"type": "json", "value": {"text": "later value"}}
            ]
        }
    });

    assert_eq!(tool_status(&value), "error");
    assert_eq!(tool_output_text(&value).as_deref(), Some("rendered failure"));
    assert_eq!(tool_error_text(&value).as_deref(), Some("rendered failure"));
}
