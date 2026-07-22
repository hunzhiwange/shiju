use super::lsp_view::{LspToolData, action_title, meta_text, summary_from_data, tool_lsp_view};
use crate::app::{App, Message};
use serde_json::{Value, json};

fn test_app() -> App {
    App::new().0
}

fn keep_element(element: iced::Element<'_, Message>) {
    std::hint::black_box(element);
}

fn lsp_visible(value: Value) -> String {
    format!("tool lsp\n{value}")
}

fn keep_lsp_view(app: &App, value: Value) {
    let visible = lsp_visible(value);
    keep_element(tool_lsp_view(app, 1, 1, &visible).expect("lsp view"));
}

#[test]
fn lsp_summary_and_meta_use_structured_counts() {
    let data: LspToolData = serde_json::from_value(json!({
        "success": true,
        "operation": "definition",
        "file_path": "src/main.rs",
        "result_count": 2,
        "file_count": 1,
        "payload": {"kind": "message", "message": "ok"}
    }))
    .expect("valid lsp payload");

    assert!(summary_from_data(&data).contains("2"));
    assert!(meta_text(&data).contains("2"));
    assert_eq!(action_title("hover"), "悬停信息");

    let hover_data: LspToolData = serde_json::from_value(json!({
        "success": true,
        "operation": "hover",
        "result_count": 1,
        "payload": {"kind": "hover", "contents": "info"}
    }))
    .expect("valid lsp payload");
    assert_eq!(meta_text(&hover_data), "悬停信息可用");
    assert_eq!(summary_from_data(&hover_data), "悬停信息可用");
}

#[test]
fn lsp_meta_text_covers_operation_specific_labels() {
    for (operation, expected) in [
        ("hover", "无悬停信息"),
        ("documentSymbol", "0 个文档符号"),
        ("workspaceSymbol", "0 个工作区符号 / 2 个文件"),
        ("prepareCallHierarchy", "0 个调用层级项"),
        ("incomingCalls", "0 个入向调用 / 2 个文件"),
        ("outgoingCalls", "0 个出向调用 / 2 个文件"),
        ("unknown", "0 个结果 / 2 个文件"),
    ] {
        let data: LspToolData = serde_json::from_value(json!({
            "success": true,
            "operation": operation,
            "result_count": 0,
            "file_count": 2,
            "payload": {"kind": "message", "message": "ok"}
        }))
        .expect("valid lsp payload");

        assert_eq!(meta_text(&data), expected);
    }
}

#[test]
fn lsp_summary_falls_back_to_query_then_file_path() {
    let with_query: LspToolData = serde_json::from_value(json!({
        "success": true,
        "operation": "workspaceSymbol",
        "query": "Widget",
        "payload": {"kind": "message", "message": "ok"}
    }))
    .expect("valid lsp payload");
    assert_eq!(summary_from_data(&with_query), "Widget");

    let whitespace_query: LspToolData = serde_json::from_value(json!({
        "success": true,
        "operation": "workspaceSymbol",
        "query": "   ",
        "file_path": "src/lib.rs",
        "payload": {"kind": "message", "message": "ok"}
    }))
    .expect("valid lsp payload");
    assert_eq!(summary_from_data(&whitespace_query), "src/lib.rs");

    let with_file: LspToolData = serde_json::from_value(json!({
        "success": true,
        "operation": "hover",
        "file_path": "src/main.rs",
        "payload": {"kind": "message", "message": "ok"}
    }))
    .expect("valid lsp payload");
    assert_eq!(summary_from_data(&with_file), "src/main.rs");
}

#[test]
fn lsp_summary_is_empty_without_counts_query_or_file_path() {
    let data: LspToolData = serde_json::from_value(json!({
        "success": true,
        "operation": "workspaceSymbol",
        "payload": {"kind": "message", "message": "no matches"}
    }))
    .expect("valid empty-summary lsp payload");

    assert_eq!(summary_from_data(&data), "");
}

#[test]
fn lsp_action_title_covers_known_operations_and_default() {
    assert_eq!(action_title("goToDefinition"), "跳转定义");
    assert_eq!(action_title("findReferences"), "查找引用");
    assert_eq!(action_title("documentSymbol"), "文档符号");
    assert_eq!(action_title("workspaceSymbol"), "工作区符号");
    assert_eq!(action_title("goToImplementation"), "跳转实现");
    assert_eq!(action_title("prepareCallHierarchy"), "调用层级");
    assert_eq!(action_title("incomingCalls"), "入向调用");
    assert_eq!(action_title("outgoingCalls"), "出向调用");
    assert_eq!(action_title("other"), "LSP");
}

#[test]
fn tool_lsp_view_rejects_non_lsp_or_invalid_json() {
    let app = test_app();

    assert!(tool_lsp_view(&app, 0, 0, "tool lsp").is_none());
    assert!(tool_lsp_view(&app, 0, 0, "lsp\n{}").is_none());
    assert!(tool_lsp_view(&app, 0, 0, "tool \n{}").is_none());
    assert!(tool_lsp_view(&app, 0, 0, "tool read\n{}").is_none());
    assert!(tool_lsp_view(&app, 0, 0, "tool lsp\nnot json").is_none());
}

#[test]
fn tool_lsp_view_builds_running_error_text_and_structured_bodies() {
    let app = test_app();

    let running = tool_lsp_view(
        &app,
        1,
        1,
        r#"tool lsp
{"status":"running","input":"{}"}"#,
    )
    .expect("running lsp view");
    keep_element(running);

    let error = tool_lsp_view(
        &app,
        1,
        2,
        r#"tool lsp
{"status":"error","error":"server failed"}"#,
    )
    .expect("error lsp view");
    keep_element(error);

    let text = tool_lsp_view(
        &app,
        1,
        3,
        r#"tool lsp
{"status":"completed","output":"plain lsp output"}"#,
    )
    .expect("text lsp view");
    keep_element(text);

    let structured = tool_lsp_view(&app, 1, 4, r#"tool lsp
{"status":"completed","result":{"data":{"success":true,"operation":"goToDefinition","result_count":1,"file_count":1,"payload":{"kind":"locations","items":[{"path":"src/main.rs","absolute_path":"/tmp/src/main.rs","line":3,"character":4,"name":"main","kind":"fn","detail":"detail","container_name":"crate"}]}}}}"#)
    .expect("structured lsp view");
    keep_element(structured);
}

#[test]
fn tool_lsp_view_builds_all_success_payload_variants() {
    let app = test_app();

    keep_lsp_view(
        &app,
        json!({
            "status": "completed",
            "result": {
                "data": {
                    "success": true,
                    "operation": "documentSymbol",
                    "result_count": 2,
                    "file_count": 1,
                    "payload": {
                        "kind": "document_symbols",
                        "items": [
                            {
                                "name": "App",
                                "kind": "struct",
                                "detail": "fields",
                                "path": "src/app.rs",
                                "absolute_path": "/tmp/src/app.rs",
                                "line": 10,
                                "character": 2,
                                "depth": 0
                            },
                            {
                                "name": "run",
                                "kind": "fn",
                                "path": "src/app.rs",
                                "absolute_path": "/tmp/src/app.rs",
                                "line": 20,
                                "character": 4,
                                "depth": 1
                            }
                        ]
                    }
                }
            }
        }),
    );

    keep_lsp_view(
        &app,
        json!({
            "status": "completed",
            "result": {
                "data": {
                    "success": true,
                    "operation": "incomingCalls",
                    "result_count": 2,
                    "file_count": 1,
                    "payload": {
                        "kind": "call_hierarchy_calls",
                        "items": [
                            {
                                "name": "caller",
                                "kind": "fn",
                                "path": "src/caller.rs",
                                "absolute_path": "/tmp/src/caller.rs",
                                "line": 7,
                                "character": 8,
                                "call_sites": [
                                    {"line": 9, "character": 10},
                                    {"line": 11, "character": 12}
                                ]
                            },
                            {
                                "name": "fallback",
                                "kind": "method",
                                "path": "src/fallback.rs",
                                "absolute_path": "/tmp/src/fallback.rs",
                                "line": 13,
                                "character": 14
                            }
                        ]
                    }
                }
            }
        }),
    );

    keep_lsp_view(
        &app,
        json!({
            "status": "completed",
            "data": {
                "success": true,
                "operation": "hover",
                "result_count": 1,
                "payload": {
                    "kind": "hover",
                    "contents": "hover body",
                    "line": 3,
                    "character": 5
                }
            }
        }),
    );

    keep_lsp_view(
        &app,
        json!({
            "status": "completed",
            "data": {
                "success": true,
                "operation": "hover",
                "result_count": 1,
                "payload": {
                    "kind": "hover",
                    "contents": "hover without position"
                }
            }
        }),
    );

    keep_lsp_view(
        &app,
        json!({
            "status": "completed",
            "data": {
                "success": true,
                "operation": "prepareCallHierarchy",
                "payload": {
                    "kind": "call_hierarchy_items",
                    "items": []
                }
            }
        }),
    );
}

#[test]
fn tool_lsp_view_builds_empty_lists_messages_and_error_fallbacks() {
    let app = test_app();

    keep_lsp_view(
        &app,
        json!({
            "status": "completed",
            "summary": "empty locations",
            "result": {
                "data": {
                    "success": true,
                    "operation": "goToDefinition",
                    "payload": {
                        "kind": "locations",
                        "items": []
                    }
                }
            }
        }),
    );

    keep_lsp_view(
        &app,
        json!({
            "status": "completed",
            "result": {
                "data": {
                    "success": true,
                    "operation": "documentSymbol",
                    "payload": {
                        "kind": "document_symbols",
                        "items": []
                    }
                }
            }
        }),
    );

    keep_lsp_view(
        &app,
        json!({
            "status": "completed",
            "output": "fallback output from malformed data",
            "result": {
                "data": {
                    "success": true,
                    "operation": "goToDefinition",
                    "payload": {"kind": "unknown"}
                }
            }
        }),
    );

    keep_lsp_view(
        &app,
        json!({
            "status": "completed",
            "output": "summary from output",
            "result": {
                "data": {
                    "success": true,
                    "operation": "outgoingCalls",
                    "payload": {
                        "kind": "message",
                        "message": "plain structured message"
                    }
                }
            }
        }),
    );

    keep_lsp_view(
        &app,
        json!({
            "status": "completed",
            "result": {
                "data": {
                    "success": false,
                    "operation": "hover",
                    "error": "language server rejected request",
                    "payload": {
                        "kind": "message",
                        "message": "failed"
                    }
                }
            }
        }),
    );

    keep_lsp_view(
        &app,
        json!({
            "status": "denied",
            "error": "permission denied"
        }),
    );
}

#[test]
fn tool_lsp_view_location_items_fall_back_to_path_when_names_are_absent() {
    let app = test_app();

    keep_lsp_view(
        &app,
        json!({
            "status": "completed",
            "result": {
                "data": {
                    "success": true,
                    "operation": "findReferences",
                    "result_count": 1,
                    "file_count": 1,
                    "payload": {
                        "kind": "locations",
                        "items": [{
                            "path": "src/lib.rs",
                            "absolute_path": "/tmp/project/src/lib.rs",
                            "line": 1,
                            "character": 2,
                            "detail": "   ",
                            "container_name": ""
                        }]
                    }
                }
            }
        }),
    );
}
