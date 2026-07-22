use super::*;
use serde_json::json;
use std::path::Path;

const WORKSPACE: &str = "/workspace";

#[test]
fn location_response_parses_location_link_and_decodes_relative_path() {
    let raw = json!({
        "targetUri": "file:///workspace/src/lib%20file.rs",
        "targetSelectionRange": {
            "start": { "line": 2, "character": 4 }
        }
    });

    let response = format_operation_result("goToDefinition", &raw, Path::new(WORKSPACE));

    assert_eq!(response.result_count, 1);
    assert_eq!(response.file_count, 1);
    assert_eq!(response.summary, "找到 1 个定义，涉及 1 个文件");
    assert_eq!(
        response.result_text,
        "找到 1 个定义，涉及 1 个文件：\npath: src/lib file.rs\n  - 3:5"
    );
    let LspPayload::Locations { items } = response.payload else {
        panic!("expected locations payload");
    };
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].path, "src/lib file.rs");
    assert_eq!(items[0].absolute_path, "/workspace/src/lib file.rs");
    assert_eq!(items[0].uri, "file:///workspace/src/lib%20file.rs");
    assert_eq!((items[0].line, items[0].character), (3, 5));
}

#[test]
fn location_response_filters_invalid_entries_and_counts_unique_files() {
    let raw = json!([
        {
            "uri": "file:///workspace/src/main.rs",
            "range": { "start": { "line": 0, "character": 1 } }
        },
        {
            "uri": "file:///workspace/src/main.rs",
            "targetRange": { "start": { "line": 9, "character": 3 } }
        },
        {
            "uri": "https://example.com/not-a-file.rs",
            "range": { "start": { "line": 0, "character": 0 } }
        },
        { "uri": "file:///workspace/src/missing-range.rs" }
    ]);

    let response = format_operation_result("findReferences", &raw, Path::new(WORKSPACE));

    assert_eq!(response.result_count, 2);
    assert_eq!(response.file_count, 1);
    assert_eq!(response.summary, "找到 2 个引用，涉及 1 个文件");
    let LspPayload::Locations { items } = response.payload else {
        panic!("expected locations payload");
    };
    assert_eq!(items.len(), 2);
    assert_eq!((items[0].line, items[0].character), (1, 2));
    assert_eq!((items[1].line, items[1].character), (10, 4));
}

#[test]
fn location_operations_report_their_specific_empty_messages() {
    for (operation, expected) in [
        ("goToDefinition", "未找到定义。"),
        ("findReferences", "未找到引用。"),
        ("goToImplementation", "未找到实现。"),
    ] {
        let response = format_operation_result(operation, &Value::Null, Path::new(WORKSPACE));

        assert_eq!(response.result_text, expected);
        assert_eq!(response.summary, expected);
        assert_eq!(response.result_count, 0);
        assert_eq!(response.file_count, 0);
        assert!(
            matches!(response.payload, LspPayload::Locations { ref items } if items.is_empty())
        );
    }
}

#[test]
fn hover_response_flattens_mixed_markup_and_converts_position_to_one_based() {
    let raw = json!({
        "contents": [
            "fn answer() -> u32",
            { "language": "rust", "value": "Returns **42**." },
            "   ",
            7
        ],
        "range": {
            "start": { "line": 10, "character": 6 }
        }
    });

    let response = format_operation_result("hover", &raw, Path::new(WORKSPACE));

    assert_eq!(response.result_text, "fn answer() -> u32\n\nReturns **42**.");
    assert_eq!(response.summary, "悬停信息可用");
    assert_eq!(response.result_count, 1);
    assert_eq!(response.file_count, 1);
    let LspPayload::Hover { contents, line, character } = response.payload else {
        panic!("expected hover payload");
    };
    assert_eq!(contents, "fn answer() -> u32\n\nReturns **42**.");
    assert_eq!(line, Some(11));
    assert_eq!(character, Some(7));
}

#[test]
fn hover_response_reports_empty_content_and_keeps_partial_range() {
    let raw = json!({
        "contents": [{ "value": "  " }, false],
        "range": { "start": { "line": 3, "character": "invalid" } }
    });

    let response = format_operation_result("hover", &raw, Path::new(WORKSPACE));

    assert_eq!(response.result_text, "没有可用的悬停信息。");
    assert_eq!(response.summary, "没有可用的悬停信息。");
    assert_eq!(response.result_count, 0);
    assert_eq!(response.file_count, 0);
    assert!(matches!(response.payload, LspPayload::Hover { line: Some(4), character: None, .. }));
}

#[test]
fn document_symbols_flatten_children_and_apply_missing_field_defaults() {
    let raw = json!([{
        "name": "Root",
        "kind": 5,
        "detail": "struct detail",
        "uri": "file:///workspace/src/lib.rs",
        "range": { "start": { "line": 4, "character": 2 } },
        "children": [{
            "detail": "   ",
            "children": []
        }]
    }]);

    let response = format_operation_result("documentSymbol", &raw, Path::new(WORKSPACE));

    assert_eq!(response.result_count, 2);
    assert_eq!(response.file_count, 2);
    assert_eq!(response.summary, "找到 2 个文档符号");
    assert_eq!(
        response.result_text,
        "文档符号：\nRoot (Class) struct detail - src/lib.rs:5\n  <unknown> (Unknown) - :1"
    );
    let LspPayload::DocumentSymbols { items } = response.payload else {
        panic!("expected document symbols payload");
    };
    assert_eq!(items[0].depth, 0);
    assert_eq!((items[0].line, items[0].character), (5, 3));
    assert_eq!(items[1].name, "<unknown>");
    assert_eq!(items[1].kind, "Unknown");
    assert_eq!(items[1].path, "");
    assert_eq!(items[1].absolute_path, WORKSPACE);
    assert_eq!((items[1].line, items[1].character, items[1].depth), (1, 1, 1));
}

#[test]
fn document_symbols_accept_workspace_symbol_shape_and_use_container_as_detail() {
    let raw = json!([
        {
            "name": "run",
            "kind": 12,
            "containerName": "Worker",
            "location": {
                "uri": "file:///workspace/src/worker.rs",
                "range": { "start": { "line": 7, "character": 1 } }
            }
        },
        { "name": "missing location" }
    ]);

    let response = format_operation_result("documentSymbol", &raw, Path::new(WORKSPACE));

    assert_eq!(response.result_count, 1);
    assert_eq!(response.file_count, 1);
    assert_eq!(response.result_text, "文档符号：\nrun (Function) Worker - src/worker.rs:8");
    let LspPayload::DocumentSymbols { items } = response.payload else {
        panic!("expected document symbols payload");
    };
    assert_eq!(items[0].detail.as_deref(), Some("Worker"));
    assert_eq!(items[0].depth, 0);
}

#[test]
fn document_symbols_report_empty_for_non_array_input() {
    let response = format_operation_result("documentSymbol", &json!({}), Path::new(WORKSPACE));

    assert_eq!(response.result_text, "文档中未找到符号。");
    assert_eq!(response.summary, "文档中未找到符号");
    assert_eq!(response.result_count, 0);
    assert_eq!(response.file_count, 0);
    assert!(matches!(
        response.payload,
        LspPayload::DocumentSymbols { ref items } if items.is_empty()
    ));
}

#[test]
fn workspace_symbols_preserve_metadata_and_filter_malformed_locations() {
    let raw = json!([
        {
            "name": "Widget",
            "kind": 23,
            "detail": "public",
            "containerName": "ui",
            "location": {
                "uri": "file:///workspace/src/widget.rs",
                "range": { "start": { "line": 11, "character": 0 } }
            }
        },
        {
            "name": "Broken",
            "kind": 5,
            "location": { "uri": "file:///workspace/src/broken.rs" }
        }
    ]);

    let response = format_operation_result("workspaceSymbol", &raw, Path::new(WORKSPACE));

    assert_eq!(response.result_count, 1);
    assert_eq!(response.file_count, 1);
    assert_eq!(response.summary, "找到 1 个工作区符号，涉及 1 个文件");
    let LspPayload::Locations { items } = response.payload else {
        panic!("expected locations payload");
    };
    assert_eq!(items[0].name.as_deref(), Some("Widget"));
    assert_eq!(items[0].kind.as_deref(), Some("Struct"));
    assert_eq!(items[0].detail.as_deref(), Some("public"));
    assert_eq!(items[0].container_name.as_deref(), Some("ui"));
    assert_eq!((items[0].line, items[0].character), (12, 1));
}

#[test]
fn prepare_call_hierarchy_formats_items_and_defaults_optional_fields() {
    let raw = json!([
        {
            "name": "caller",
            "kind": 6,
            "detail": "impl Service",
            "uri": "file:///workspace/src/service.rs",
            "range": { "start": { "line": 5, "character": 8 } }
        },
        {
            "uri": "file:///external/generated.rs"
        },
        {
            "uri": "https://example.com/rejected.rs"
        }
    ]);

    let response = format_operation_result("prepareCallHierarchy", &raw, Path::new(WORKSPACE));

    assert_eq!(response.result_count, 2);
    assert_eq!(response.file_count, 2);
    assert_eq!(response.summary, "找到 2 个调用层级项");
    assert_eq!(
        response.result_text,
        "找到 2 个调用层级项：\n- caller (Method) - src/service.rs:6\n- <unknown> (Unknown) - /external/generated.rs:1"
    );
    let LspPayload::CallHierarchyItems { items } = response.payload else {
        panic!("expected call hierarchy items payload");
    };
    assert_eq!(items[0].detail.as_deref(), Some("impl Service"));
    assert_eq!((items[0].line, items[0].character), (6, 9));
    assert_eq!((items[1].line, items[1].character), (1, 1));
    assert!(items.iter().all(|item| item.call_sites.is_empty()));
}

#[test]
fn prepare_call_hierarchy_reports_empty_when_all_items_are_invalid() {
    let response = format_operation_result(
        "prepareCallHierarchy",
        &json!([{ "name": "missing uri" }]),
        Path::new(WORKSPACE),
    );

    assert_eq!(response.result_text, "当前位置没有可用的调用层级项。");
    assert_eq!(response.summary, "没有可用的调用层级项");
    assert_eq!(response.result_count, 0);
    assert_eq!(response.file_count, 0);
}

#[test]
fn incoming_calls_collect_valid_call_sites_and_ignore_malformed_ranges() {
    let raw = json!([{
        "from": {
            "name": "entry",
            "kind": 12,
            "uri": "file:///workspace/src/main.rs",
            "range": { "start": { "line": 20, "character": 2 } }
        },
        "fromRanges": [
            { "start": { "line": 3, "character": 4 } },
            { "start": { "line": "invalid", "character": 0 } },
            {}
        ]
    }]);

    let response = format_operation_result("incomingCalls", &raw, Path::new(WORKSPACE));

    assert_eq!(response.result_count, 1);
    assert_eq!(response.file_count, 1);
    assert_eq!(response.summary, "找到 1 个入向调用");
    assert_eq!(
        response.result_text,
        "找到 1 个入向调用：\n- entry (Function) - src/main.rs:21 [4:5]"
    );
    let LspPayload::CallHierarchyCalls { items } = response.payload else {
        panic!("expected call hierarchy calls payload");
    };
    assert_eq!(items[0].call_sites.len(), 1);
    assert_eq!((items[0].call_sites[0].line, items[0].call_sites[0].character), (4, 5));
}

#[test]
fn outgoing_calls_accept_to_target_without_call_sites() {
    let raw = json!([{
        "to": {
            "name": "leaf",
            "kind": 12,
            "uri": "file:///workspace/src/leaf.rs",
            "range": { "start": { "line": 1, "character": 0 } }
        }
    }]);

    let response = format_operation_result("outgoingCalls", &raw, Path::new(WORKSPACE));

    assert_eq!(response.summary, "找到 1 个出向调用");
    assert_eq!(response.result_text, "找到 1 个出向调用：\n- leaf (Function) - src/leaf.rs:2");
    let LspPayload::CallHierarchyCalls { items } = response.payload else {
        panic!("expected call hierarchy calls payload");
    };
    assert!(items[0].call_sites.is_empty());
}

#[test]
fn call_hierarchy_calls_report_operation_specific_empty_messages() {
    for (operation, label) in [("incomingCalls", "入向调用"), ("outgoingCalls", "出向调用")]
    {
        let response = format_operation_result(operation, &json!([{}]), Path::new(WORKSPACE));

        assert_eq!(response.result_text, format!("未找到{label}。"));
        assert_eq!(response.summary, format!("未找到{label}"));
        assert_eq!(response.result_count, 0);
        assert_eq!(response.file_count, 0);
    }
}

#[test]
fn unsupported_operation_returns_explicit_message_payload() {
    let response = format_operation_result("rename", &json!({}), Path::new(WORKSPACE));

    assert_eq!(response.result_text, "不支持的 LSP 操作。");
    assert_eq!(response.summary, "LSP 操作不受支持");
    assert_eq!(response.result_count, 0);
    assert_eq!(response.file_count, 0);
    assert!(matches!(
        response.payload,
        LspPayload::Message { ref message } if message == "不支持的 LSP 操作。"
    ));
}

#[test]
fn path_helpers_decode_file_uris_and_preserve_external_paths() {
    assert_eq!(
        decode_file_uri("file:///workspace/a%20b.rs"),
        Some(PathBuf::from("/workspace/a b.rs"))
    );
    assert_eq!(decode_file_uri("https://example.com/a.rs"), None);
    assert_eq!(decode_file_uri("file:///workspace/%FF.rs"), None);
    assert_eq!(
        display_path(Path::new(WORKSPACE), Path::new("/workspace/src/lib.rs")),
        "src/lib.rs"
    );
    assert_eq!(
        display_path(Path::new(WORKSPACE), Path::new("/external/lib.rs")),
        "/external/lib.rs"
    );
    assert_eq!(unique_file_count(["a.rs", "a.rs", "b.rs"].into_iter()), 2);
}

#[test]
fn symbol_kind_labels_cover_protocol_values_and_unknown_values() {
    let expected = [
        "File",
        "Module",
        "Namespace",
        "Package",
        "Class",
        "Method",
        "Property",
        "Field",
        "Constructor",
        "Enum",
        "Interface",
        "Function",
        "Variable",
        "Constant",
        "String",
        "Number",
        "Boolean",
        "Array",
        "Object",
        "Key",
        "Null",
        "EnumMember",
        "Struct",
        "Event",
        "Operator",
        "TypeParameter",
    ];

    for (kind, label) in (1_u64..=26).zip(expected) {
        assert_eq!(symbol_kind_label(kind), label);
    }
    assert_eq!(symbol_kind_label(0), "Unknown");
    assert_eq!(symbol_kind_label(27), "Unknown");
}

#[test]
fn payload_serialization_uses_kind_tag_and_omits_absent_fields() {
    let hover = serde_json::to_value(LspPayload::Hover {
        contents: "docs".to_string(),
        line: None,
        character: Some(2),
    })
    .expect("hover payload should serialize");
    assert_eq!(
        hover,
        json!({
            "kind": "hover",
            "contents": "docs",
            "character": 2
        })
    );

    let calls = serde_json::to_value(LspPayload::CallHierarchyCalls {
        items: vec![LspCallHierarchyItem {
            name: "target".to_string(),
            kind: "Function".to_string(),
            detail: None,
            path: "src/lib.rs".to_string(),
            absolute_path: "/workspace/src/lib.rs".to_string(),
            uri: "file:///workspace/src/lib.rs".to_string(),
            line: 1,
            character: 1,
            call_sites: Vec::new(),
        }],
    })
    .expect("call hierarchy payload should serialize");
    assert_eq!(calls["kind"], "call_hierarchy_calls");
    assert!(calls["items"][0].get("detail").is_none());
    assert!(calls["items"][0].get("call_sites").is_none());
}

#[test]
fn payload_deserialization_restores_location_metadata_and_optional_fields() {
    let payload: LspPayload = serde_json::from_value(json!({
        "kind": "locations",
        "items": [{
            "path": "src/lib.rs",
            "absolute_path": "/workspace/src/lib.rs",
            "uri": "file:///workspace/src/lib.rs",
            "line": 4,
            "character": 2,
            "name": "answer",
            "kind": "Function"
        }]
    }))
    .expect("locations payload should deserialize");

    let LspPayload::Locations { items } = payload else {
        panic!("expected locations payload");
    };
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].path, "src/lib.rs");
    assert_eq!(items[0].absolute_path, "/workspace/src/lib.rs");
    assert_eq!(items[0].name.as_deref(), Some("answer"));
    assert_eq!(items[0].kind.as_deref(), Some("Function"));
    assert_eq!(items[0].detail, None);
    assert_eq!(items[0].container_name, None);
}
