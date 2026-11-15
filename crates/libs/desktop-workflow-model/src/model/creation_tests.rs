//! 工作流模型创建的测试，覆盖空白文档、默认节点和初始元数据。
//! 生成结果会核对标识、坐标及必要字段，避免创建不可编辑的草稿。
#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("creation_tests"));
}

use super::*;
use iced::Point;
use serde_yaml::Value;

const MINIMAL_WORKFLOW_YAML: &str = r#"
workflow:
  graph:
    nodes:
      - id: start
        data:
          title: Start
          type: start
    edges: []
"#;

fn mapping_field<'a>(value: &'a Value, key: &str) -> &'a Value {
    value
        .as_mapping()
        .and_then(|mapping| mapping.get(&Value::String(key.to_string())))
        .expect("expected mapping field")
}

#[test]
fn create_blank_workflow_builds_named_default_graph() {
    let loaded = create_blank_workflow(WorkflowAppMeta {
        name: "审批助手".to_string(),
        description: "处理审批".to_string(),
        icon: "A".to_string(),
        icon_background: "#112233".to_string(),
        mode: "workflow".to_string(),
        use_icon_as_answer_icon: true,
        max_active_requests: 4,
    })
    .expect("blank workflow should load");

    assert_eq!(loaded.source_name, "审批助手");
    assert_eq!(loaded.app_meta.description, "处理审批");
    assert_eq!(loaded.app_meta.icon, "A");
    assert!(loaded.app_meta.use_icon_as_answer_icon);
    assert_eq!(loaded.app_meta.max_active_requests, 4);
    assert_eq!(loaded.document.nodes.len(), 2);
    assert_eq!(loaded.document.edges.len(), 1);
    assert!(loaded.document.node("start-node").is_some());
    assert!(loaded.document.node("answer-node").is_some());
    assert!(loaded.had_viewport);
}

#[test]
fn create_node_from_type_uses_default_geometry_data_and_handles() {
    let node = create_node_from_type("if-else", "branch".to_string(), Point::new(12.0, 34.0), 7.0)
        .expect("if-else node should be created");

    assert_eq!(node.id, "branch");
    assert_eq!(node.block_type, "if-else");
    assert_eq!(node.title, "条件分支");
    assert_eq!(node.position, Point::new(12.0, 34.0));
    assert_eq!(node.size.width, 242.0);
    assert_eq!(node.size.height, 180.0);
    assert_eq!(node.z_index, 7.0);
    assert_eq!(
        node.source_handles.iter().map(|handle| handle.id.as_str()).collect::<Vec<_>>(),
        vec!["true", "false"]
    );
    assert_eq!(node.target_handles.len(), 1);

    let data = mapping_field(&node.raw_node, "data");
    assert_eq!(mapping_field(data, "type").as_str(), Some("if-else"));
    assert_eq!(mapping_field(&node.raw_node, "zIndex").as_f64(), Some(7.0));
}

#[test]
fn default_and_existing_node_data_yaml_are_editor_ready() {
    let default_yaml = default_node_data_yaml("code").expect("default yaml should serialize");
    assert!(!default_yaml.starts_with("---"));
    assert!(default_yaml.contains("code_language: python3"));

    let node = create_node_from_type("answer", "answer".to_string(), Point::ORIGIN, 1.0)
        .expect("answer node should be created");
    let node_yaml = node_data_yaml(&node).expect("node data yaml should serialize");
    assert!(node_yaml.contains("answer:"));
    assert!(node_yaml.contains("type: answer"));

    let node_without_data = WorkflowNode { raw_node: Value::Null, ..node };
    assert_eq!(node_data_yaml(&node_without_data).expect("empty data should serialize"), "{}\n");
}

#[test]
fn rebuild_node_from_parts_updates_mapping_and_reports_invalid_yaml() {
    let base = create_node_from_type("answer", "answer".to_string(), Point::new(1.0, 2.0), 3.0)
        .expect("answer node should be created");

    let rebuilt =
        rebuild_node_from_parts(&base, "  新标题  ", "  新描述  ", "answer: hello\nextra: true\n")
            .expect("mapping data should rebuild");

    assert_eq!(rebuilt.title, "新标题");
    assert_eq!(rebuilt.description, "新描述");
    assert_eq!(rebuilt.block_type, "answer");
    assert_eq!(rebuilt.selected, base.selected);
    let data = mapping_field(&rebuilt.raw_node, "data");
    assert_eq!(mapping_field(data, "title").as_str(), Some("新标题"));
    assert_eq!(mapping_field(data, "desc").as_str(), Some("新描述"));
    assert_eq!(mapping_field(data, "answer").as_str(), Some("hello"));
    assert_eq!(mapping_field(data, "extra").as_bool(), Some(true));

    let empty_data =
        rebuild_node_from_parts(&base, "标题", "描述", " \n").expect("blank data should rebuild");
    assert_eq!(
        mapping_field(mapping_field(&empty_data.raw_node, "data"), "title").as_str(),
        Some("标题")
    );

    let non_mapping_error =
        rebuild_node_from_parts(&base, "标题", "", "- item\n").expect_err("sequence data fails");
    assert_eq!(non_mapping_error, "节点 data 必须是对象映射（YAML map）");

    let parse_error =
        rebuild_node_from_parts(&base, "标题", "", "foo: [").expect_err("bad yaml fails");
    assert!(parse_error.starts_with("节点 data YAML 解析失败:"));
}

#[test]
fn rebuild_node_from_parts_reports_invalid_raw_node_shape() {
    let base = create_node_from_type("answer", "answer".to_string(), Point::ORIGIN, 1.0)
        .expect("answer node should be created");
    let malformed = WorkflowNode { raw_node: Value::Null, ..base };

    let error = rebuild_node_from_parts(&malformed, "标题", "描述", "answer: hello\n")
        .expect_err("a node without its required identity fields should fail to rebuild");

    assert!(error.starts_with("重建节点失败:"));
}

#[test]
fn load_document_from_text_reports_yaml_and_graph_errors() {
    let parse_error =
        load_document_from_text(None, "workflow: [".to_string()).expect_err("invalid YAML fails");
    assert!(parse_error.starts_with("解析 Dify DSL 失败:"));

    let graph_error = load_document_from_text(
        Some("/tmp/no-graph.yml".to_string()),
        "app:\n  name: Missing Graph\n".to_string(),
    )
    .expect_err("missing graph fails");
    assert_eq!(graph_error, "未找到 workflow.graph 节点，无法构建画布");
}

#[test]
fn load_document_from_path_reads_file_and_reports_read_errors() {
    let temp = tempfile::tempdir().expect("temp dir should be created");
    let workflow_path = temp.path().join("from-path.yml");
    std::fs::write(&workflow_path, MINIMAL_WORKFLOW_YAML).expect("workflow file should be written");

    let loaded = load_document_from_path(&workflow_path.to_string_lossy())
        .expect("workflow should load from path");

    assert_eq!(loaded.source_path.as_deref(), Some(workflow_path.to_string_lossy().as_ref()));
    assert_eq!(loaded.source_name, "未命名应用");
    assert_eq!(loaded.document.nodes.len(), 1);
    assert_eq!(loaded.document.nodes[0].id, "start");

    let missing_path = temp.path().join("missing.yml");
    let read_error = load_document_from_path(&missing_path.to_string_lossy())
        .expect_err("missing file should fail");
    assert!(read_error.starts_with("读取工作流文件失败:"));
}
