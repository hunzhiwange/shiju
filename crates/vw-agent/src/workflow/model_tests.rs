//! 验证工作流 YAML 模型的兼容解析、错误报告与字段默认值。

use super::{array_field, edge_handle_field, parse_workflow_yaml, string_field};
use serde_json::json;

#[test]
fn parse_workflow_yaml_accepts_root_graph_and_node_type_fallbacks() {
    let graph = parse_workflow_yaml(
        r#"
graph:
  nodes:
    - id: start
      type: start
    - id: custom
      data:
        title: "  Custom Title  "
    - id: from-data
      type: ignored
      data:
        type: answer
  edges:
    - source: start
      target: custom
      sourceHandle: true
    - source: custom
      target: from-data
      sourceHandle: 7
    - source: missing-target-only
"#,
    )
    .expect("graph parses");

    assert_eq!(graph.start_node_ids, vec!["start"]);
    assert_eq!(graph.nodes["start"].node_type, "start");
    assert_eq!(graph.nodes["custom"].node_type, "custom");
    assert_eq!(graph.nodes["custom"].title, "Custom Title");
    assert_eq!(graph.nodes["from-data"].node_type, "answer");
    assert_eq!(graph.edges.len(), 2);
    assert_eq!(graph.edges[0].source_handle.as_deref(), Some("true"));
    assert_eq!(graph.edges[1].source_handle.as_deref(), Some("7"));
}

#[test]
fn parse_workflow_yaml_reports_schema_errors() {
    assert!(
        parse_workflow_yaml("not: [")
            .expect_err("yaml error")
            .contains("解析 Dify workflow YAML 失败")
    );
    assert_eq!(
        parse_workflow_yaml("workflow: {}").expect_err("missing graph"),
        "未找到 workflow.graph"
    );
    assert_eq!(
        parse_workflow_yaml("workflow:\n  graph:\n    nodes: {}\n").expect_err("nodes array"),
        "workflow.graph.nodes 必须是数组"
    );
    assert_eq!(
        parse_workflow_yaml("workflow:\n  graph:\n    nodes:\n      - data: { type: start }\n")
            .expect_err("node id"),
        "workflow node 缺少 id"
    );
    assert_eq!(
        parse_workflow_yaml(
            "workflow:\n  graph:\n    nodes:\n      - id: a\n        data: { type: answer }\n"
        )
        .expect_err("start"),
        "workflow 缺少 start 节点"
    );
}

#[test]
fn field_helpers_return_expected_defaults() {
    let value = json!({
        "name": "demo",
        "items": [1, 2],
        "emptyHandle": "   ",
        "objectHandle": {}
    });

    assert_eq!(string_field(&value, "name"), Some("demo"));
    assert_eq!(string_field(&value, "missing"), None);
    assert_eq!(array_field(&value, "items").len(), 2);
    assert!(array_field(&value, "missing").is_empty());
    assert_eq!(edge_handle_field(&value, "missing"), None);
    assert_eq!(edge_handle_field(&value, "emptyHandle"), None);
    assert_eq!(edge_handle_field(&value, "objectHandle"), None);
}
