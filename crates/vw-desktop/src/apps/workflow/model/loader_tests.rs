//! 工作流模型加载的测试，覆盖 YAML 解析、格式识别和错误报告。
//! 临界输入验证空文档、非法结构及缺失字段会得到明确而安全的结果。
use super::*;
use iced::{Point, Size};
use serde_yaml::Value;

#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("loader_tests"));
}

#[test]
fn load_document_accepts_edges_without_ids() {
    let loaded = load_document_from_text(
        None,
        r#"
workflow:
  graph:
    nodes:
      - id: start
        data:
          title: Start
          type: start
      - id: answer
        data:
          title: Answer
          type: answer
    edges:
      - source: start
        sourceHandle: source
        target: answer
"#
        .to_string(),
    )
    .expect("workflow should load when generated DSL omits edge ids");

    assert_eq!(loaded.document.edges.len(), 1);
    assert_eq!(loaded.document.edges[0].source, "start");
    assert_eq!(loaded.document.edges[0].target, "answer");
    assert_eq!(loaded.document.edges[0].id, "start-source-answer-target-1");
}

fn field<'a>(value: &'a Value, key: &str) -> &'a Value {
    value
        .as_mapping()
        .and_then(|mapping| mapping.get(&Value::String(key.to_string())))
        .expect("expected field")
}

#[test]
fn load_document_reads_app_graph_variables_viewport_and_handles() {
    let loaded = load_document_from_text(
        Some("/tmp/original.yml".to_string()),
        r##"
app:
  name: "  "
  description: " desc "
  icon: ""
  icon_background: ""
  mode: workflow
  use_icon_as_answer_icon: true
  max_active_requests: -8
workflow:
  environment_variables:
    - id: ""
      name: "  "
      value_type: ""
      value: hello
      description: "  env desc  "
  conversation_variables:
    - id: ""
      name: "  "
      value_type: ""
      value:
        count: 1
      description: "  conv desc  "
  graph:
    viewport:
      x: 7
      y: 8
      zoom: 0
    nodes:
      - id: start
        type: custom
        selected: true
        position:
          x: 1
          y: 2
        width: 20
        height: 10
        sourcePosition: top
        targetPosition: bottom
        zIndex: 2
        data:
          title: "  "
          desc: "  trimmed  "
          type: start
          selected: false
          variables:
            - variable: q
              type: paragraph
            - label: file
              type: file
      - id: branch
        type: custom
        positionAbsolute:
          x: 20
          y: 40
        parentId: group
        width: 300
        height: 180
        sourcePosition: left
        targetPosition: right
        data:
          title: Branch
          type: if-else
          error_strategy: fail-branch
          cases:
            - case_id: "true"
            - id: custom-case
            - case_id: "long-branch-name-with-dash"
      - id: end
        type: end
        data:
          title: ""
          type: ""
    edges:
      - id: "  "
        source: start
        sourceHandle: source
        target: branch
        targetHandle: target
        selected: true
        zIndex: 5
        data:
          sourceType: ""
          targetType: ""
"##
        .to_string(),
    )
    .expect("workflow should load");

    assert_eq!(loaded.source_name, "original");
    assert_eq!(loaded.app_meta.name, "original");
    assert_eq!(loaded.app_meta.description, " desc ");
    assert_eq!(loaded.app_meta.icon, "");
    assert_eq!(loaded.app_meta.icon_background, "");
    assert_eq!(loaded.app_meta.mode, "workflow");
    assert!(loaded.app_meta.use_icon_as_answer_icon);
    assert_eq!(loaded.app_meta.max_active_requests, 0);
    assert!(loaded.had_viewport);
    assert_eq!(loaded.document.viewport, WorkflowViewport { x: 7.0, y: 8.0, zoom: 0.1 });
    assert_eq!(loaded.document.nodes.len(), 3);

    let start = loaded.document.node("start").expect("start node");
    assert_eq!(start.title, "开始");
    assert_eq!(start.description, "trimmed");
    assert_eq!(start.position, Point::new(1.0, 2.0));
    assert_eq!(start.size, Size::new(120.0, 148.0));
    assert!(start.selected);
    assert_eq!(start.source_side, WorkflowHandleSide::Top);
    assert_eq!(start.target_side, WorkflowHandleSide::Bottom);
    assert_eq!(start.source_handles.len(), 1);
    assert!(start.target_handles.is_empty());

    let branch = loaded.document.node("branch").expect("branch node");
    assert_eq!(branch.parent_id.as_deref(), Some("group"));
    assert_eq!(branch.position, Point::new(20.0, 40.0));
    assert_eq!(branch.source_side, WorkflowHandleSide::Left);
    assert_eq!(branch.target_side, WorkflowHandleSide::Right);
    assert_eq!(
        branch
            .source_handles
            .iter()
            .map(|handle| (handle.id.as_str(), handle.label.as_str()))
            .collect::<Vec<_>>(),
        vec![
            ("true", "是"),
            ("custom-case", "分支 2"),
            ("long-branch-name-with-dash", "分支 3"),
            ("fail-branch", "异常")
        ]
    );
    assert_eq!(branch.target_handles.len(), 1);

    let end = loaded.document.node("end").expect("end node");
    assert_eq!(end.block_type, "end");
    assert_eq!(end.title, "结束");
    assert!(end.source_handles.is_empty());
    assert_eq!(end.target_handles.len(), 1);

    let edge = &loaded.document.edges[0];
    assert_eq!(edge.id, "start-source-branch-target-1");
    assert_eq!(edge.source_type, "start");
    assert_eq!(edge.target_type, "if-else");
    assert!(edge.selected);
    assert_eq!(edge.z_index, 5.0);

    assert_eq!(loaded.environment_variables[0].id, "env-var");
    assert_eq!(loaded.environment_variables[0].name, "environment_var");
    assert_eq!(loaded.environment_variables[0].value_type, "string");
    assert_eq!(loaded.environment_variables[0].description, "env desc");
    assert_eq!(loaded.conversation_variables[0].id, "conversation-var");
    assert_eq!(loaded.conversation_variables[0].name, "conversation_var");
    assert_eq!(loaded.conversation_variables[0].value_type, "string");
    assert_eq!(loaded.conversation_variables[0].description, "conv desc");
}

#[test]
fn load_document_supports_root_graph_and_default_app_meta() {
    let loaded = load_document_from_text(
        None,
        r#"
graph:
  nodes:
    - id: trigger
      type: trigger-webhook
      data:
        type: ""
        title: ""
    - id: answer
      data:
        type: answer
        title: Answer
  edges:
    - id: edge-1
      source: trigger
      target: answer
      data:
        sourceType: trigger-webhook
        targetType: answer
"#
        .to_string(),
    )
    .expect("root graph should load");

    assert_eq!(loaded.source_name, "未命名应用");
    assert_eq!(loaded.app_meta, WorkflowAppMeta::default());
    assert!(!loaded.had_viewport);
    assert_eq!(loaded.document.viewport, WorkflowViewport::default());

    let trigger = loaded.document.node("trigger").expect("trigger node");
    assert_eq!(trigger.block_type, "trigger-webhook");
    assert!(trigger.target_handles.is_empty());
    assert_eq!(trigger.source_handles.len(), 1);
}

#[test]
fn load_document_applies_defaults_for_metadata_nodes_edges_and_handles() {
    let loaded = load_document_from_text(
        Some("/tmp/ignored-name.yml".to_string()),
        r#"
app:
  name: Named App
workflow:
  graph:
    nodes:
      - id: custom
        type: renderer-kind
        selected: false
        data:
          title: "  Custom Title  "
          desc: " desc "
          type: ""
          selected: true
      - id: branch
        data:
          type: if-else
          cases:
            - case_id: "false"
            - case_id: source
            - id: simple
            - case_id: ""
          error_strategy: fail-branch
      - id: fail-branch-existing
        data:
          type: question-classifier
          cases:
            - case_id: fail-branch
          error_strategy: fail-branch
      - id: end-with-empty-case
        type: end
        data:
          type: ""
          cases:
            - case_id: ""
    edges:
      - id: explicit-edge
        source: unknown-source
        target: unknown-target
      - source: branch
        sourceHandle: "false"
        target: custom
        targetHandle: target
        data:
          sourceType: explicit-source
          targetType: explicit-target
"#
        .to_string(),
    )
    .expect("workflow should load");

    assert_eq!(loaded.source_path.as_deref(), Some("/tmp/ignored-name.yml"));
    assert_eq!(loaded.source_name, "Named App");
    assert_eq!(loaded.app_meta.description, "");
    assert_eq!(loaded.app_meta.icon, "🤖");
    assert_eq!(loaded.app_meta.icon_background, "#FFEAD5");
    assert_eq!(loaded.app_meta.mode, "advanced-chat");
    assert!(!loaded.app_meta.use_icon_as_answer_icon);
    assert_eq!(loaded.app_meta.max_active_requests, 0);

    let custom = loaded.document.node("custom").expect("custom node");
    assert_eq!(custom.block_type, "renderer-kind");
    assert_eq!(custom.title, "Custom Title");
    assert_eq!(custom.description, "desc");
    assert_eq!(custom.position, Point::new(0.0, 0.0));
    assert_eq!(custom.source_side, WorkflowHandleSide::Right);
    assert_eq!(custom.target_side, WorkflowHandleSide::Left);
    assert!(!custom.selected);
    assert_eq!(custom.z_index, 0.0);
    assert_eq!(field(&custom.raw_node, "type").as_str(), Some("renderer-kind"));

    let branch = loaded.document.node("branch").expect("branch node");
    assert_eq!(
        branch
            .source_handles
            .iter()
            .map(|handle| (handle.id.as_str(), handle.label.as_str(), handle.kind))
            .collect::<Vec<_>>(),
        vec![
            ("false", "否", WorkflowHandleKind::Source),
            ("source", "", WorkflowHandleKind::Source),
            ("simple", "simple", WorkflowHandleKind::Source),
            ("fail-branch", "异常", WorkflowHandleKind::Source),
        ]
    );

    let existing_fail = loaded.document.node("fail-branch-existing").expect("fail branch node");
    assert_eq!(existing_fail.source_handles.len(), 1);
    assert_eq!(existing_fail.source_handles[0].id, "fail-branch");

    let end = loaded.document.node("end-with-empty-case").expect("end node");
    assert!(end.source_handles.is_empty());
    assert_eq!(end.block_type, "end");

    assert_eq!(loaded.document.edges[0].id, "explicit-edge");
    assert_eq!(loaded.document.edges[0].source_type, "custom");
    assert_eq!(loaded.document.edges[0].target_type, "custom");
    assert_eq!(loaded.document.edges[1].id, "branch-false-custom-target-2");
    assert_eq!(loaded.document.edges[1].source_type, "explicit-source");
    assert_eq!(loaded.document.edges[1].target_type, "explicit-target");
}

#[test]
fn load_document_uses_data_selected_when_node_selection_is_absent_and_clamps_height() {
    let loaded = load_document_from_text(
        None,
        r#"
app:
  name: Data Selected
workflow:
  graph:
    nodes:
      - id: answer
        width: 80
        height: 10
        data:
          type: answer
          title: Answer
          selected: true
          custom_field: kept
    edges: []
"#
        .to_string(),
    )
    .expect("workflow should load");

    let node = loaded.document.node("answer").expect("answer node");
    assert!(node.selected);
    assert_eq!(node.size, Size::new(120.0, 48.0));
    assert_eq!(field(field(&node.raw_node, "data"), "custom_field").as_str(), Some("kept"));
}

#[test]
fn load_document_preserves_named_variables_and_raw_values() {
    let loaded = load_document_from_text(
        None,
        r#"
app:
  name: Variable App
workflow:
  environment_variables:
    - id: env-id
      name: " token "
      value_type: secret
      value:
        nested: true
      description: " desc "
  conversation_variables:
    - id: conv-id
      name: " count "
      value_type: number
      value: 3
      description: " conv "
  graph:
    nodes: []
    edges: []
"#
        .to_string(),
    )
    .expect("workflow should load");

    let env = &loaded.environment_variables[0];
    assert_eq!(env.id, "env-id");
    assert_eq!(env.name, "token");
    assert_eq!(env.value_type, "secret");
    assert_eq!(env.description, "desc");
    assert_eq!(field(&env.raw_variable, "id").as_str(), Some("env-id"));
    assert_eq!(field(&env.value, "nested").as_bool(), Some(true));

    let conv = &loaded.conversation_variables[0];
    assert_eq!(conv.id, "conv-id");
    assert_eq!(conv.name, "count");
    assert_eq!(conv.value_type, "number");
    assert_eq!(conv.description, "conv");
    assert_eq!(field(&conv.raw_variable, "id").as_str(), Some("conv-id"));
    assert_eq!(conv.value.as_i64(), Some(3));
}

#[test]
fn load_document_reports_parse_and_shape_errors() {
    let parse_error = load_document_from_text(None, "workflow: [".to_string())
        .expect_err("invalid yaml should fail");
    assert!(parse_error.starts_with("解析 Dify DSL 失败:"));

    let shape_error = load_document_from_value(None, yaml_map(vec![("app", yaml_map(vec![]))]))
        .expect_err("missing graph should fail");
    assert_eq!(shape_error, "未找到 workflow.graph 节点，无法构建画布");
}

#[test]
fn load_document_from_value_reports_deserialization_errors() {
    let error = load_document_from_value(None, Value::String("not a workflow".to_string()))
        .expect_err("a scalar root cannot deserialize as a workflow file");

    assert!(error.starts_with("解析 Dify DSL 失败:"));
}

#[test]
fn suggested_workflow_file_name_sanitizes_titles() {
    assert_eq!(suggested_workflow_file_name(" Sales Bot "), "Sales_Bot.yml");
    assert_eq!(suggested_workflow_file_name("客户/审批:*"), "客户_审批.yml");
    assert_eq!(suggested_workflow_file_name(" !!! "), "workflow_app.yml");
}

#[test]
fn load_document_uses_builtin_name_when_blank_app_name_and_path_has_no_stem() {
    let loaded = load_document_from_text(
        Some("/".to_string()),
        r#"
app:
  name: " "
workflow:
  graph:
    nodes: []
    edges: []
"#
        .to_string(),
    )
    .expect("workflow should load");

    assert_eq!(loaded.source_name, "工作流示例");
    assert_eq!(loaded.app_meta.name, "工作流示例");
    assert_eq!(loaded.document.name, "工作流示例");
}

#[test]
fn helper_functions_cover_handle_and_label_fallbacks() {
    assert_eq!(
        parse_handle_side(Some("LEFT"), WorkflowHandleSide::Right),
        WorkflowHandleSide::Left
    );
    assert_eq!(
        parse_handle_side(Some(" right "), WorkflowHandleSide::Left),
        WorkflowHandleSide::Right
    );
    assert_eq!(parse_handle_side(Some("Top"), WorkflowHandleSide::Left), WorkflowHandleSide::Top);
    assert_eq!(
        parse_handle_side(Some("bottom"), WorkflowHandleSide::Left),
        WorkflowHandleSide::Bottom
    );
    assert_eq!(
        parse_handle_side(Some("diagonal"), WorkflowHandleSide::Bottom),
        WorkflowHandleSide::Bottom
    );
    assert_eq!(parse_handle_side(None, WorkflowHandleSide::Top), WorkflowHandleSide::Top);

    assert_eq!(normalized_block_type(&Some("renderer".to_string()), " "), "renderer");
    assert_eq!(normalized_block_type(&None, " "), "custom");
    assert_eq!(normalized_block_type(&Some("renderer".to_string()), " answer "), "answer");

    assert_eq!(pretty_handle_label("false", 0), "否");
    assert_eq!(pretty_handle_label("source", 0), "");
    assert_eq!(pretty_handle_label("target", 0), "");
    assert_eq!(pretty_handle_label("short", 0), "short");
    assert_eq!(pretty_handle_label("averyveryveryverylongid", 1), "分支 2");
}

#[test]
fn build_source_handles_uses_default_when_cases_are_blank_and_avoids_duplicate_fail_branch() {
    let blank_case_data = DifyNodeData {
        block_type: "if-else".to_string(),
        cases: vec![DifyCase { case_id: Some(" ".to_string()), id: None }],
        ..DifyNodeData::default()
    };
    let handles = build_source_handles(&blank_case_data, &None, "if-else");
    assert_eq!(handles.len(), 1);
    assert_eq!(handles[0].id, "source");
    assert_eq!(handles[0].kind, WorkflowHandleKind::Source);

    let end_handles = build_source_handles(
        &DifyNodeData {
            block_type: "end".to_string(),
            cases: vec![DifyCase { case_id: Some(" ".to_string()), id: None }],
            ..DifyNodeData::default()
        },
        &None,
        "end",
    );
    assert!(end_handles.is_empty());

    let fail_branch_data = DifyNodeData {
        block_type: "tool".to_string(),
        error_strategy: "fail-branch".to_string(),
        cases: vec![DifyCase {
            case_id: Some("fail-branch".to_string()),
            id: Some("ignored".to_string()),
        }],
        ..DifyNodeData::default()
    };
    let handles = build_source_handles(&fail_branch_data, &None, "tool");
    assert_eq!(handles.len(), 1);
    assert_eq!(handles[0].id, "fail-branch");
    assert_eq!(handles[0].label, "异常");
}

#[test]
fn build_target_handles_skips_start_like_nodes_and_keeps_custom_targets() {
    for block_type in [
        "start",
        "trigger-webhook",
        "trigger-schedule",
        "trigger-plugin",
        "iteration-start",
        "loop-start",
    ] {
        assert!(
            build_target_handles(&None, block_type).is_empty(),
            "{block_type} should not accept incoming handles"
        );
    }

    let handles = build_target_handles(&Some("answer".to_string()), "");
    assert_eq!(handles.len(), 1);
    assert_eq!(handles[0].id, "target");
    assert_eq!(handles[0].kind, WorkflowHandleKind::Target);
}

#[test]
fn workflow_edge_from_dify_preserves_explicit_data_and_falls_back_for_unknown_nodes() {
    let mut node_types = HashMap::new();
    node_types.insert("source-node", "known-source");
    let raw_edge = yaml_map(vec![("marker", Value::String("raw".to_string()))]);

    let explicit = workflow_edge_from_dify(
        &DifyEdge {
            id: Some(" explicit ".to_string()),
            source: "source-node".to_string(),
            target: "target-node".to_string(),
            source_handle: None,
            target_handle: Some("custom-target".to_string()),
            selected: None,
            data: Some(DifyEdgeData {
                source_type: "configured-source".to_string(),
                target_type: "configured-target".to_string(),
            }),
            z_index: None,
        },
        0,
        &node_types,
        raw_edge.clone(),
    );

    assert_eq!(explicit.id, "explicit");
    assert_eq!(explicit.source_type, "configured-source");
    assert_eq!(explicit.target_type, "configured-target");
    assert!(!explicit.selected);
    assert_eq!(explicit.z_index, 0.0);
    assert_eq!(explicit.raw_edge, raw_edge);

    let generated = workflow_edge_from_dify(
        &DifyEdge {
            id: None,
            source: "missing-source".to_string(),
            target: "missing-target".to_string(),
            source_handle: None,
            target_handle: None,
            selected: None,
            data: None,
            z_index: None,
        },
        2,
        &node_types,
        Value::Null,
    );

    assert_eq!(generated.id, "missing-source-source-missing-target-target-3");
    assert_eq!(generated.source_type, "custom");
    assert_eq!(generated.target_type, "custom");
}

#[test]
fn workflow_app_meta_defaults_optional_fields_without_trimming_present_values() {
    let meta = workflow_app_meta_from_dify(Some(&DifyApp {
        name: None,
        description: None,
        icon: None,
        icon_background: None,
        mode: None,
        use_icon_as_answer_icon: None,
        max_active_requests: Some(12),
    }));

    assert_eq!(meta.name, "未命名应用");
    assert_eq!(meta.description, "");
    assert_eq!(meta.icon, "🤖");
    assert_eq!(meta.icon_background, "#FFEAD5");
    assert_eq!(meta.mode, "advanced-chat");
    assert!(!meta.use_icon_as_answer_icon);
    assert_eq!(meta.max_active_requests, 12);
}

#[test]
fn workflow_variables_trim_names_and_keep_raw_values() {
    let raw = yaml_map(vec![("raw", Value::String("env".to_string()))]);
    let env = workflow_environment_variable_from_dify(
        &DifyEnvironmentVariable {
            id: Some(" env-id ".to_string()),
            name: " name ".to_string(),
            value_type: " secret ".to_string(),
            value: Value::Bool(true),
            description: " description ".to_string(),
        },
        raw.clone(),
    );

    assert_eq!(env.id, " env-id ");
    assert_eq!(env.name, "name");
    assert_eq!(env.value_type, "secret");
    assert_eq!(env.value, Value::Bool(true));
    assert_eq!(env.description, "description");
    assert_eq!(env.raw_variable, raw);

    let raw = yaml_map(vec![("raw", Value::String("conversation".to_string()))]);
    let conversation = workflow_conversation_variable_from_dify(
        &DifyConversationVariable {
            id: Some(" conv-id ".to_string()),
            name: " topic ".to_string(),
            value_type: " object ".to_string(),
            value: yaml_map(vec![("count", serde_yaml::to_value(1).expect("number"))]),
            description: " memory ".to_string(),
        },
        raw.clone(),
    );

    assert_eq!(conversation.id, " conv-id ");
    assert_eq!(conversation.name, "topic");
    assert_eq!(conversation.value_type, "object");
    assert_eq!(conversation.description, "memory");
    assert_eq!(conversation.raw_variable, raw);
}

#[test]
fn serialize_workflow_yaml_patches_root_and_removes_legacy_graph() {
    let loaded = create_blank_workflow(WorkflowAppMeta {
        name: "保存测试".to_string(),
        description: "旧描述".to_string(),
        ..WorkflowAppMeta::default()
    })
    .expect("blank workflow should load");
    let mut raw_root = loaded.raw_root.clone();
    raw_root
        .as_mapping_mut()
        .expect("root map")
        .insert(Value::String("graph".to_string()), yaml_map(vec![]));
    let environment_variables = vec![WorkflowEnvironmentVariable {
        id: "env-id".to_string(),
        name: "token".to_string(),
        value_type: "secret".to_string(),
        value: Value::String("abc".to_string()),
        description: "密钥".to_string(),
        raw_variable: Value::Null,
    }];
    let conversation_variables = vec![WorkflowConversationVariable {
        id: "conv-id".to_string(),
        name: "count".to_string(),
        value_type: "number".to_string(),
        value: serde_yaml::to_value(3).expect("number value"),
        description: "计数".to_string(),
        raw_variable: Value::Null,
    }];

    let yaml = serialize_workflow_yaml(
        &WorkflowAppMeta {
            name: "新名称".to_string(),
            description: "新描述".to_string(),
            icon: "N".to_string(),
            icon_background: "#FFFFFF".to_string(),
            mode: "advanced-chat".to_string(),
            use_icon_as_answer_icon: true,
            max_active_requests: 9,
        },
        &loaded.document,
        &environment_variables,
        &conversation_variables,
        &raw_root,
        WorkflowViewport { x: 1.0, y: 2.0, zoom: 1.5 },
    )
    .expect("workflow yaml should serialize");
    let value = serde_yaml::from_str::<Value>(&yaml).expect("serialized yaml should parse");

    assert!(
        value.as_mapping().expect("root map").get(&Value::String("graph".to_string())).is_none()
    );
    assert_eq!(field(field(&value, "app"), "name").as_str(), Some("新名称"));
    assert_eq!(
        field(field(field(&value, "workflow"), "graph"), "nodes")
            .as_sequence()
            .expect("nodes")
            .len(),
        loaded.document.nodes.len()
    );
    assert_eq!(
        field(field(field(field(&value, "workflow"), "graph"), "viewport"), "zoom").as_f64(),
        Some(1.5)
    );
    assert_eq!(
        field(
            &field(field(&value, "workflow"), "environment_variables")
                .as_sequence()
                .expect("env vars")[0],
            "value_type"
        )
        .as_str(),
        Some("secret")
    );
    assert_eq!(
        field(
            &field(field(&value, "workflow"), "conversation_variables")
                .as_sequence()
                .expect("conv vars")[0],
            "name"
        )
        .as_str(),
        Some("count")
    );
}
