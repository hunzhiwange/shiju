//! 工作流模型测试，验证节点、连接和应用元数据的数据结构行为。

use super::*;
use iced::{Point, Rectangle, Size};
use serde_yaml::Value;

fn workflow_node_with_raw(block_type: &str, raw_node: Value) -> WorkflowNode {
    WorkflowNode {
        id: "node_start".to_string(),
        block_type: block_type.to_string(),
        title: "节点".to_string(),
        description: String::new(),
        position: Point::new(0.0, 0.0),
        size: Size::new(180.0, 120.0),
        parent_id: None,
        selected: false,
        source_side: WorkflowHandleSide::Right,
        target_side: WorkflowHandleSide::Left,
        source_handles: Vec::new(),
        target_handles: Vec::new(),
        z_index: 0.0,
        raw_node,
    }
}

#[test]
fn start_node_variables_prefer_variable_name_and_map_value_types() {
    let raw_node = serde_yaml::from_str::<Value>(
        r#"
data:
  variables:
    - variable: user_query
      label: 用户问题
      type: paragraph
    - label: 重试次数
      type: number
    - type: file-list
"#,
    )
    .expect("start node yaml should parse");
    let node = workflow_node_with_raw("start", raw_node.clone());

    let variables = workflow_start_node_variables(&node);

    assert_eq!(variables.len(), 3);
    assert_eq!(variables[0].name, "user_query");
    assert_eq!(variables[0].value_type, "string");
    assert_eq!(variables[1].name, "重试次数");
    assert_eq!(variables[1].value_type, "number");
    assert_eq!(variables[2].name, "变量 3");
    assert_eq!(variables[2].value_type, "array[file]");
    assert_eq!(workflow_start_node_min_height(&raw_node), 190.0);
}

#[test]
fn start_node_min_height_uses_empty_baseline_without_variables() {
    let raw_node =
        serde_yaml::from_str::<Value>("data: {}").expect("empty start node yaml should parse");

    assert_eq!(workflow_start_node_min_height(&raw_node), 120.0);
}

#[test]
fn start_node_min_height_keeps_baseline_for_single_variable() {
    let raw_node = serde_yaml::from_str::<Value>(
        r#"
data:
  variables:
    - variable: query
      type: text-input
"#,
    )
    .expect("single start variable yaml should parse");

    assert_eq!(workflow_start_node_min_height(&raw_node), 120.0);
}

#[test]
fn start_node_variables_reject_malformed_container_shapes() {
    let malformed_values = [
        Value::Null,
        serde_yaml::from_str::<Value>("data: []").expect("sequence data yaml should parse"),
        serde_yaml::from_str::<Value>("data:\n  variables: invalid")
            .expect("scalar variables yaml should parse"),
    ];

    for raw_node in malformed_values {
        assert!(workflow_start_node_variables_from_raw(&raw_node).is_empty());
    }
}

#[test]
fn start_node_variables_trim_names_and_fall_back_for_non_mapping_entries() {
    let raw_node = serde_yaml::from_str::<Value>(
        r#"
data:
  variables:
    - variable: "  "
      label: "  用户输入  "
    - variable: 42
      label: " "
      type: unknown
    - null
"#,
    )
    .expect("fallback start variable yaml should parse");

    assert_eq!(
        workflow_start_node_variables_from_raw(&raw_node),
        vec![
            WorkflowStartVariableSummary {
                name: "用户输入".to_string(),
                value_type: "string".to_string(),
            },
            WorkflowStartVariableSummary {
                name: "变量 2".to_string(),
                value_type: "string".to_string(),
            },
            WorkflowStartVariableSummary {
                name: "变量 3".to_string(),
                value_type: "string".to_string(),
            },
        ]
    );
}

#[test]
fn default_start_node_data_initializes_query_and_files_variables() {
    let raw_data = default_node_data_value("start");
    let variables = workflow_start_node_variables_from_raw(&yaml_map(vec![("data", raw_data)]));

    assert_eq!(variables.len(), 2);
    assert_eq!(variables[0].name, "query");
    assert_eq!(variables[0].value_type, "string");
    assert_eq!(variables[1].name, "files");
    assert_eq!(variables[1].value_type, "array[file]");
}

#[test]
fn viewport_and_app_meta_defaults_match_new_workflow_baseline() {
    let viewport = WorkflowViewport::default();
    let app_meta = WorkflowAppMeta::default();

    assert_eq!(viewport, WorkflowViewport { x: 120.0, y: 120.0, zoom: 1.0 });
    assert_eq!(app_meta.name, "未命名应用");
    assert_eq!(app_meta.description, "");
    assert_eq!(app_meta.icon, "🤖");
    assert_eq!(app_meta.icon_background, "#FFEAD5");
    assert_eq!(app_meta.mode, "advanced-chat");
    assert!(!app_meta.use_icon_as_answer_icon);
    assert_eq!(app_meta.max_active_requests, 0);
}

#[test]
fn workflow_node_helpers_return_rect_group_and_handles() {
    let node = WorkflowNode {
        source_handles: vec![WorkflowHandle {
            id: "source".to_string(),
            label: "输出".to_string(),
            kind: WorkflowHandleKind::Source,
        }],
        target_handles: vec![WorkflowHandle {
            id: "target".to_string(),
            label: "输入".to_string(),
            kind: WorkflowHandleKind::Target,
        }],
        size: Size::new(200.0, 140.0),
        position: Point::new(12.0, 34.0),
        ..workflow_node_with_raw("custom", Value::Null)
    };

    assert_eq!(node.rect_world(), Rectangle::new(Point::new(12.0, 34.0), Size::new(200.0, 140.0)));
    assert!(node.is_group());
    assert_eq!(
        node.handle(WorkflowHandleKind::Source, "source").map(|handle| handle.label.as_str()),
        Some("输出")
    );
    assert_eq!(
        node.handle(WorkflowHandleKind::Target, "target").map(|handle| handle.label.as_str()),
        Some("输入")
    );
    assert!(node.handle(WorkflowHandleKind::Source, "missing").is_none());

    let iteration = WorkflowNode {
        size: Size::new(80.0, 60.0),
        ..workflow_node_with_raw("iteration", Value::Null)
    };
    assert!(iteration.is_group());

    let regular = workflow_node_with_raw("answer", Value::Null);
    assert!(!regular.is_group());
}

#[test]
fn workflow_start_node_variables_ignore_non_start_nodes_and_map_all_supported_types() {
    let raw_node = serde_yaml::from_str::<Value>(
        r#"
data:
  variables:
    - variable: amount
      type: number
    - variable: agreed
      type: checkbox
    - variable: avatar
      type: file
    - variable: attachments
      type: file-list
    - variable: note
      type: text-input
"#,
    )
    .expect("start variable yaml should parse");
    let start_node = workflow_node_with_raw("start", raw_node);
    let answer_node = workflow_node_with_raw("answer", start_node.raw_node.clone());

    assert!(workflow_start_node_variables(&answer_node).is_empty());
    assert_eq!(
        workflow_start_node_variables(&start_node)
            .iter()
            .map(|item| (item.name.as_str(), item.value_type.as_str()))
            .collect::<Vec<_>>(),
        vec![
            ("amount", "number"),
            ("agreed", "boolean"),
            ("avatar", "file"),
            ("attachments", "array[file]"),
            ("note", "string"),
        ]
    );
}

#[test]
fn workflow_document_bounds_and_lookup_helpers_cover_nested_nodes_and_edges() {
    let parent = WorkflowNode {
        id: "parent".to_string(),
        position: Point::new(10.0, 20.0),
        size: Size::new(100.0, 80.0),
        ..workflow_node_with_raw("loop", Value::Null)
    };
    let child = WorkflowNode {
        id: "child".to_string(),
        parent_id: Some("parent".to_string()),
        position: Point::new(-30.0, 150.0),
        size: Size::new(60.0, 40.0),
        ..workflow_node_with_raw("code", Value::Null)
    };
    let grandchild = WorkflowNode {
        id: "grandchild".to_string(),
        parent_id: Some("child".to_string()),
        position: Point::new(240.0, -10.0),
        size: Size::new(30.0, 20.0),
        ..workflow_node_with_raw("answer", Value::Null)
    };
    let edge = WorkflowEdge {
        id: "edge".to_string(),
        source: "parent".to_string(),
        target: "grandchild".to_string(),
        source_handle: Some("source".to_string()),
        target_handle: Some("target".to_string()),
        source_type: "loop".to_string(),
        target_type: "answer".to_string(),
        selected: false,
        z_index: 2.0,
        raw_edge: Value::Null,
    };
    let mut document = WorkflowDocument {
        name: "Doc".to_string(),
        nodes: vec![parent, child, grandchild],
        edges: vec![edge],
        viewport: WorkflowViewport::default(),
    };

    let bounds = document.bounds().expect("document should have bounds");
    assert_eq!(bounds.x, -30.0);
    assert_eq!(bounds.y, -10.0);
    assert_eq!(bounds.width, 300.0);
    assert_eq!(bounds.height, 200.0);
    assert_eq!(document.node("child").map(|node| node.block_type.as_str()), Some("code"));
    assert_eq!(document.node_mut("child").map(|node| node.title = "Child".to_string()), Some(()));
    assert_eq!(document.edge("edge").map(|edge| edge.source.as_str()), Some("parent"));
    assert_eq!(document.group_child_count("parent"), 1);
    assert_eq!(
        document.ancestor_ids("grandchild"),
        vec!["child".to_string(), "parent".to_string()]
    );
    assert_eq!(
        document.descendant_ids("parent"),
        vec!["child".to_string(), "grandchild".to_string()]
    );
    assert_eq!(document.remove_edge("edge").map(|edge| edge.id), Some("edge".to_string()));
    assert!(document.remove_edge("edge").is_none());
}

#[test]
fn empty_workflow_document_has_no_bounds() {
    assert!(WorkflowDocument::default().bounds().is_none());
}

#[test]
fn workflow_document_missing_lookups_and_orphan_relationships_are_safe() {
    let orphan = WorkflowNode {
        id: "orphan".to_string(),
        parent_id: Some("missing-parent".to_string()),
        ..workflow_node_with_raw("code", Value::Null)
    };
    let mut document = WorkflowDocument { nodes: vec![orphan], ..WorkflowDocument::default() };

    assert!(document.node("missing").is_none());
    assert!(document.node_mut("missing").is_none());
    assert!(document.edge("missing").is_none());
    assert_eq!(document.group_child_count("missing-parent"), 1);
    assert_eq!(document.group_child_count("other"), 0);
    assert_eq!(document.ancestor_ids("missing"), Vec::<String>::new());
    assert_eq!(document.ancestor_ids("orphan"), vec!["missing-parent".to_string()]);
    assert_eq!(document.descendant_ids("missing-parent"), vec!["orphan".to_string()]);
    assert_eq!(document.descendant_ids("other"), Vec::<String>::new());
}

#[test]
fn workflow_document_bounds_clamp_zero_sized_extent() {
    let point_node = WorkflowNode {
        position: Point::new(-5.0, 7.0),
        size: Size::new(0.0, 0.0),
        ..workflow_node_with_raw("answer", Value::Null)
    };
    let document = WorkflowDocument { nodes: vec![point_node], ..WorkflowDocument::default() };

    assert_eq!(document.bounds(), Some(Rectangle { x: -5.0, y: 7.0, width: 1.0, height: 1.0 }));
}
