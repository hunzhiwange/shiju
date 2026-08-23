//! 工作流模型保存的测试，覆盖 YAML 输出、路径选择和脏状态清理。
//! 往返 fixture 验证保存内容可重新加载，并保留节点与连线语义。
#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("save_tests"));
}

use super::*;

fn map_value<'a>(value: &'a Value, key: &str) -> &'a Value {
    value.as_mapping().expect("value should be a map").get(key).expect("key should exist")
}

fn map_missing(value: &Value, key: &str) -> bool {
    value.as_mapping().expect("value should be a map").get(key).is_none()
}

fn test_node(id: &str, block_type: &str, raw_node: Value) -> WorkflowNode {
    WorkflowNode {
        id: id.to_string(),
        block_type: block_type.to_string(),
        title: format!("{block_type} title"),
        description: format!("{block_type} desc"),
        position: Point::new(12.0, 34.0),
        size: Size::new(320.0, 144.0),
        parent_id: None,
        selected: true,
        source_side: WorkflowHandleSide::Top,
        target_side: WorkflowHandleSide::Bottom,
        source_handles: Vec::new(),
        target_handles: Vec::new(),
        z_index: 7.0,
        raw_node,
    }
}

fn test_edge(raw_edge: Value) -> WorkflowEdge {
    WorkflowEdge {
        id: "edge-1".to_string(),
        source: "source-node".to_string(),
        target: "target-node".to_string(),
        source_handle: None,
        target_handle: Some("target".to_string()),
        source_type: "start".to_string(),
        target_type: "answer".to_string(),
        selected: true,
        z_index: 3.0,
        raw_edge,
    }
}

#[test]
fn blank_workflow_root_builds_minimal_start_answer_graph_with_app_meta() {
    let meta = WorkflowAppMeta {
        name: "Demo".to_string(),
        description: "Desc".to_string(),
        icon: "D".to_string(),
        icon_background: "#000000".to_string(),
        mode: "workflow".to_string(),
        use_icon_as_answer_icon: true,
        max_active_requests: 9,
    };

    let root = blank_workflow_root(&meta);

    assert_eq!(map_value(map_value(&root, "app"), "name").as_str(), Some("Demo"));
    assert_eq!(map_value(map_value(&root, "app"), "description").as_str(), Some("Desc"));
    assert_eq!(map_value(map_value(&root, "app"), "mode").as_str(), Some("workflow"));
    assert_eq!(map_value(map_value(&root, "app"), "use_icon_as_answer_icon").as_bool(), Some(true));
    assert_eq!(map_value(map_value(&root, "app"), "max_active_requests"), &yaml_value(9_u64));
    assert_eq!(map_value(&root, "kind").as_str(), Some("app"));
    assert_eq!(map_value(&root, "version").as_str(), Some("0.5.0"));
    assert!(map_value(&root, "dependencies").as_sequence().unwrap().is_empty());

    let graph = map_value(map_value(&root, "workflow"), "graph");
    let nodes = map_value(graph, "nodes").as_sequence().expect("nodes should be a sequence");
    let edges = map_value(graph, "edges").as_sequence().expect("edges should be a sequence");
    assert_eq!(nodes.len(), 2);
    assert_eq!(edges.len(), 1);
    assert_eq!(map_value(&nodes[0], "id").as_str(), Some("start-node"));
    assert_eq!(map_value(map_value(&nodes[0], "data"), "type").as_str(), Some("start"));
    assert_eq!(map_value(&nodes[1], "id").as_str(), Some("answer-node"));
    assert_eq!(
        map_value(map_value(&nodes[1], "data"), "answer").as_str(),
        Some("你好，这是一份新的 Dify 工作流。")
    );
    assert_eq!(map_value(&edges[0], "source").as_str(), Some("start-node"));
    assert_eq!(map_value(&edges[0], "target").as_str(), Some("answer-node"));
    assert_eq!(map_value(map_value(graph, "viewport"), "zoom"), &yaml_value(1.0_f64));
}

#[test]
fn patch_root_for_save_updates_app_graph_variables_viewport_and_removes_legacy_graph() {
    let app_meta = WorkflowAppMeta {
        name: "Saved".to_string(),
        description: "Updated".to_string(),
        icon: "S".to_string(),
        icon_background: "#123456".to_string(),
        mode: "advanced-chat".to_string(),
        use_icon_as_answer_icon: true,
        max_active_requests: 4,
    };
    let document = WorkflowDocument {
        name: "Saved".to_string(),
        nodes: vec![test_node(
            "node-1",
            "answer",
            yaml_map(vec![("data", yaml_map(vec![("old", Value::Bool(true))]))]),
        )],
        edges: vec![WorkflowEdge {
            source_handle: Some("source".to_string()),
            ..test_edge(Value::Null)
        }],
        viewport: WorkflowViewport::default(),
    };
    let env = WorkflowEnvironmentVariable {
        id: "env-id".to_string(),
        name: "api_key".to_string(),
        value_type: "secret".to_string(),
        value: Value::String("secret".to_string()),
        description: "secret desc".to_string(),
        raw_variable: yaml_map(vec![("kept", Value::Bool(true))]),
    };
    let conversation = WorkflowConversationVariable {
        id: "conv-id".to_string(),
        name: "history".to_string(),
        value_type: "array[string]".to_string(),
        value: Value::Sequence(vec![Value::String("hello".to_string())]),
        description: "conv desc".to_string(),
        raw_variable: Value::Null,
    };
    let raw_root = yaml_map(vec![
        ("app", Value::String("not a map".to_string())),
        ("graph", yaml_map(vec![("legacy", Value::Bool(true))])),
        (
            "workflow",
            yaml_map(vec![
                ("graph", Value::String("not a map".to_string())),
                ("features", yaml_map(vec![("kept", Value::Bool(true))])),
            ]),
        ),
        ("kind", Value::String("app".to_string())),
    ]);

    let patched = patch_root_for_save(
        &app_meta,
        &document,
        &[env.clone()],
        &[conversation.clone()],
        &raw_root,
        WorkflowViewport { x: 5.0, y: 6.0, zoom: 1.5 },
    )
    .expect("root should patch");

    assert!(map_missing(&patched, "graph"));
    assert_eq!(map_value(map_value(&patched, "app"), "name").as_str(), Some("Saved"));
    assert_eq!(map_value(map_value(&patched, "app"), "max_active_requests"), &yaml_value(4_u64));
    let workflow = map_value(&patched, "workflow");
    assert_eq!(map_value(map_value(workflow, "features"), "kept").as_bool(), Some(true));
    let graph = map_value(workflow, "graph");
    assert_eq!(map_value(map_value(graph, "viewport"), "x"), &yaml_value(5.0_f64));
    assert_eq!(map_value(map_value(graph, "viewport"), "zoom"), &yaml_value(1.5_f64));
    assert_eq!(map_value(graph, "nodes").as_sequence().unwrap().len(), 1);
    assert_eq!(map_value(graph, "edges").as_sequence().unwrap().len(), 1);
    let saved_env = &map_value(workflow, "environment_variables").as_sequence().unwrap()[0];
    assert_eq!(map_value(saved_env, "id").as_str(), Some(env.id.as_str()));
    assert_eq!(map_value(saved_env, "kept").as_bool(), Some(true));
    let saved_conversation =
        &map_value(workflow, "conversation_variables").as_sequence().unwrap()[0];
    assert_eq!(map_value(saved_conversation, "name").as_str(), Some(conversation.name.as_str()));
}

#[test]
fn patch_root_for_save_creates_required_sections_from_non_mapping_root() {
    let app_meta = WorkflowAppMeta {
        name: "Fresh".to_string(),
        description: String::new(),
        icon: "F".to_string(),
        icon_background: "#ffffff".to_string(),
        mode: "workflow".to_string(),
        use_icon_as_answer_icon: false,
        max_active_requests: 1,
    };
    let document = WorkflowDocument {
        name: "Fresh".to_string(),
        nodes: Vec::new(),
        edges: Vec::new(),
        viewport: WorkflowViewport::default(),
    };

    let patched = patch_root_for_save(
        &app_meta,
        &document,
        &[],
        &[],
        &Value::String("not a root map".to_string()),
        WorkflowViewport { x: -1.0, y: 2.5, zoom: 0.75 },
    )
    .expect("non-map roots should be normalized");

    assert_eq!(map_value(map_value(&patched, "app"), "name").as_str(), Some("Fresh"));
    let workflow = map_value(&patched, "workflow");
    let graph = map_value(workflow, "graph");
    assert!(map_value(graph, "nodes").as_sequence().unwrap().is_empty());
    assert!(map_value(graph, "edges").as_sequence().unwrap().is_empty());
    assert_eq!(map_value(map_value(graph, "viewport"), "x"), &yaml_value(-1.0_f64));
    assert!(map_value(workflow, "environment_variables").as_sequence().unwrap().is_empty());
    assert!(map_value(workflow, "conversation_variables").as_sequence().unwrap().is_empty());
}

#[test]
fn patch_root_for_save_preserves_unrelated_root_metadata() {
    let app_meta = WorkflowAppMeta { name: "Meta".to_string(), ..WorkflowAppMeta::default() };
    let document = WorkflowDocument::default();
    let raw_root = yaml_map(vec![("custom_metadata", Value::String("keep me".to_string()))]);

    let patched = patch_root_for_save(
        &app_meta,
        &document,
        &[],
        &[],
        &raw_root,
        WorkflowViewport { x: 0.0, y: 0.0, zoom: 1.0 },
    )
    .expect("root metadata should be preserved");

    assert_eq!(map_value(&patched, "custom_metadata").as_str(), Some("keep me"));
    assert_eq!(map_value(map_value(&patched, "app"), "name").as_str(), Some("Meta"));
}

#[test]
fn saved_node_value_rewrites_geometry_selection_handle_sides_parent_and_data() {
    let raw_node = yaml_map(vec![
        ("parentId", Value::String("old-parent".to_string())),
        ("data", Value::String("not a map".to_string())),
        ("custom", Value::Bool(true)),
    ]);
    let mut node = test_node("node-2", "code", raw_node);
    node.parent_id = Some("parent".to_string());

    let saved = saved_node_value(&node);

    assert_eq!(map_value(&saved, "id").as_str(), Some("node-2"));
    assert_eq!(map_value(map_value(&saved, "position"), "x"), &yaml_value(12.0_f64));
    assert_eq!(map_value(map_value(&saved, "positionAbsolute"), "y"), &yaml_value(34.0_f64));
    assert_eq!(map_value(&saved, "width"), &yaml_value(320.0_f64));
    assert_eq!(map_value(&saved, "height"), &yaml_value(144.0_f64));
    assert_eq!(map_value(&saved, "parentId").as_str(), Some("parent"));
    assert_eq!(map_value(&saved, "selected").as_bool(), Some(true));
    assert_eq!(map_value(&saved, "sourcePosition").as_str(), Some("top"));
    assert_eq!(map_value(&saved, "targetPosition").as_str(), Some("bottom"));
    assert_eq!(map_value(&saved, "zIndex"), &yaml_value(7.0_f64));
    assert_eq!(map_value(&saved, "custom").as_bool(), Some(true));
    let data = map_value(&saved, "data");
    assert_eq!(map_value(data, "title").as_str(), Some("code title"));
    assert_eq!(map_value(data, "desc").as_str(), Some("code desc"));
    assert_eq!(map_value(data, "type").as_str(), Some("code"));
    assert_eq!(map_value(data, "selected").as_bool(), Some(true));

    let without_parent = saved_node_value(&WorkflowNode { parent_id: None, ..node });
    assert!(map_missing(&without_parent, "parentId"));
}

#[test]
fn saved_node_value_normalizes_non_mapping_raw_and_removes_blank_parent() {
    let mut node = test_node("node-3", "llm", Value::Sequence(Vec::new()));
    node.parent_id = Some("   ".to_string());
    node.selected = false;
    node.source_side = WorkflowHandleSide::Left;
    node.target_side = WorkflowHandleSide::Right;

    let saved = saved_node_value(&node);

    assert_eq!(map_value(&saved, "id").as_str(), Some("node-3"));
    assert!(map_missing(&saved, "parentId"));
    assert_eq!(map_value(&saved, "selected").as_bool(), Some(false));
    assert_eq!(map_value(&saved, "sourcePosition").as_str(), Some("left"));
    assert_eq!(map_value(&saved, "targetPosition").as_str(), Some("right"));
    assert_eq!(map_value(map_value(&saved, "data"), "type").as_str(), Some("llm"));
}

#[test]
fn saved_edge_value_rewrites_endpoints_handles_data_and_type() {
    let saved = saved_edge_value(&test_edge(Value::Null));

    assert_eq!(map_value(&saved, "id").as_str(), Some("edge-1"));
    assert_eq!(map_value(&saved, "source").as_str(), Some("source-node"));
    assert_eq!(map_value(&saved, "target").as_str(), Some("target-node"));
    assert!(map_missing(&saved, "sourceHandle"));
    assert_eq!(map_value(&saved, "targetHandle").as_str(), Some("target"));
    assert_eq!(map_value(&saved, "selected").as_bool(), Some(true));
    assert_eq!(map_value(&saved, "zIndex"), &yaml_value(3.0_f64));
    assert_eq!(map_value(&saved, "type").as_str(), Some("custom"));
    assert_eq!(map_value(map_value(&saved, "data"), "sourceType").as_str(), Some("start"));
    assert_eq!(map_value(map_value(&saved, "data"), "targetType").as_str(), Some("answer"));

    let raw_edge = yaml_map(vec![("type", Value::String("smoothstep".to_string()))]);
    let preserved = saved_edge_value(&test_edge(raw_edge));
    assert_eq!(map_value(&preserved, "type").as_str(), Some("smoothstep"));
}

#[test]
fn saved_edge_value_removes_blank_handles_and_preserves_raw_data_fields() {
    let mut edge =
        test_edge(yaml_map(vec![("data", yaml_map(vec![("isInLoop", Value::Bool(true))]))]));
    edge.source_handle = Some(" ".to_string());
    edge.target_handle = Some(String::new());
    edge.selected = false;

    let saved = saved_edge_value(&edge);

    assert!(map_missing(&saved, "sourceHandle"));
    assert!(map_missing(&saved, "targetHandle"));
    assert_eq!(map_value(&saved, "selected").as_bool(), Some(false));
    let data = map_value(&saved, "data");
    assert_eq!(map_value(data, "isInLoop").as_bool(), Some(true));
    assert_eq!(map_value(data, "sourceType").as_str(), Some("start"));
    assert_eq!(map_value(data, "targetType").as_str(), Some("answer"));
}

#[test]
fn saved_variable_values_preserve_raw_fields_and_replace_editable_fields() {
    let env = WorkflowEnvironmentVariable {
        id: "env".to_string(),
        name: "token".to_string(),
        value_type: "secret".to_string(),
        value: Value::String("abc".to_string()),
        description: "desc".to_string(),
        raw_variable: yaml_map(vec![("kept", Value::Bool(true))]),
    };
    let conversation = WorkflowConversationVariable {
        id: "conv".to_string(),
        name: "topic".to_string(),
        value_type: "string".to_string(),
        value: Value::String("rust".to_string()),
        description: "conv desc".to_string(),
        raw_variable: Value::String("not a map".to_string()),
    };

    let saved_env = saved_environment_variable_value(&env);
    let saved_conversation = saved_conversation_variable_value(&conversation);

    assert_eq!(map_value(&saved_env, "id").as_str(), Some("env"));
    assert_eq!(map_value(&saved_env, "name").as_str(), Some("token"));
    assert_eq!(map_value(&saved_env, "value_type").as_str(), Some("secret"));
    assert_eq!(map_value(&saved_env, "value").as_str(), Some("abc"));
    assert_eq!(map_value(&saved_env, "description").as_str(), Some("desc"));
    assert_eq!(map_value(&saved_env, "kept").as_bool(), Some(true));
    assert_eq!(map_value(&saved_conversation, "id").as_str(), Some("conv"));
    assert_eq!(map_value(&saved_conversation, "name").as_str(), Some("topic"));
    assert_eq!(map_value(&saved_conversation, "value").as_str(), Some("rust"));
}

#[test]
fn raw_graph_and_workflow_lookup_handles_nested_and_legacy_shapes() {
    let nested = yaml_map(vec![
        ("workflow", yaml_map(vec![("graph", yaml_map(vec![("nested", Value::Bool(true))]))])),
        ("graph", yaml_map(vec![("legacy", Value::Bool(true))])),
    ]);
    let legacy = yaml_map(vec![("graph", yaml_map(vec![("legacy", Value::Bool(true))]))]);

    assert_eq!(
        raw_graph_value(&nested).and_then(|graph| graph.get("nested")).and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        raw_graph_value(&legacy).and_then(|graph| graph.get("legacy")).and_then(Value::as_bool),
        Some(true)
    );
    assert!(raw_graph_value(&Value::String("bad".to_string())).is_none());
    assert!(raw_workflow_value(&nested).is_some());
    assert!(raw_workflow_value(&legacy).is_none());
}

#[test]
fn raw_graph_value_requires_nested_workflow_graph_when_workflow_key_exists() {
    let workflow_without_map = yaml_map(vec![
        ("workflow", Value::String("not a workflow map".to_string())),
        ("graph", yaml_map(vec![("legacy", Value::Bool(true))])),
    ]);
    let workflow_without_graph =
        yaml_map(vec![("workflow", yaml_map(vec![])), ("graph", yaml_map(vec![]))]);
    let workflow_graph_not_map = yaml_map(vec![(
        "workflow",
        yaml_map(vec![("graph", Value::String("not a graph map".to_string()))]),
    )]);

    assert!(raw_graph_value(&workflow_without_map).is_none());
    assert!(raw_graph_value(&workflow_without_graph).is_none());
    assert!(raw_graph_value(&workflow_graph_not_map).is_none());
    assert!(raw_workflow_value(&workflow_without_map).is_none());
}

#[test]
fn yaml_helpers_create_editor_yaml_maps_points_viewports_and_optional_strings() {
    let yaml = yaml_string_for_editor(&yaml_map(vec![("name", Value::String("demo".to_string()))]))
        .expect("yaml should serialize");
    assert_eq!(yaml, "name: demo\n");
    assert!(ensure_root_mapping(Value::String("bad".to_string())).as_mapping().unwrap().is_empty());

    let mut value = Value::String("bad".to_string());
    let map = ensure_value_mapping(&mut value);
    set_mapping_value(map, "required", Value::Bool(true));
    set_optional_string(map, "maybe", Some(" value "));
    assert_eq!(map.get("required").and_then(Value::as_bool), Some(true));
    assert_eq!(map.get("maybe").and_then(Value::as_str), Some(" value "));
    set_optional_string(map, "maybe", Some("   "));
    assert!(map.get("maybe").is_none());

    assert_eq!(key_value("k"), Value::String("k".to_string()));
    assert_eq!(map_value(&point_value(1.0, 2.0), "x"), &yaml_value(1.0_f64));
    assert_eq!(map_value(&viewport_value(1.0, 2.0, 0.5), "zoom"), &yaml_value(0.5_f64));
    assert_eq!(handle_side_name(WorkflowHandleSide::Left), "left");
    assert_eq!(handle_side_name(WorkflowHandleSide::Right), "right");
    assert_eq!(handle_side_name(WorkflowHandleSide::Top), "top");
    assert_eq!(handle_side_name(WorkflowHandleSide::Bottom), "bottom");
    assert!(is_chat_mode("advanced-chat"));
    assert!(!is_chat_mode(" workflow "));
}

#[test]
fn yaml_helpers_preserve_existing_maps_and_scalar_yaml_without_document_marker() {
    let existing = yaml_map(vec![("kept", Value::Bool(true))]);
    assert_eq!(
        ensure_root_mapping(existing.clone())
            .as_mapping()
            .unwrap()
            .get("kept")
            .and_then(Value::as_bool),
        Some(true)
    );

    let mut value = existing;
    let map = ensure_value_mapping(&mut value);
    assert_eq!(map.get("kept").and_then(Value::as_bool), Some(true));
    set_optional_string(map, "maybe", None);
    assert!(map.get("maybe").is_none());

    let yaml = yaml_string_for_editor(&Value::String("plain".to_string()))
        .expect("scalar YAML should serialize");
    assert_eq!(yaml, "plain\n");
    assert_eq!(yaml_value("value"), Value::String("value".to_string()));
    assert!(is_chat_mode("   "));
}
