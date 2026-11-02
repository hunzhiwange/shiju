//! # Workflow 模型创建
//!
//! 该模块提供空白工作流创建、节点创建以及节点 YAML 重建能力。

use super::*;

pub fn create_blank_workflow(
    app_meta: WorkflowAppMeta,
) -> Result<LoadedWorkflow, crate::app::i18n::workflow::Feedback> {
    let root = blank_workflow_root(&app_meta);
    load_document_from_value(None, root)
}

pub fn create_node_from_type(
    block_type: &str,
    node_id: String,
    position: Point,
    z_index: f32,
) -> Result<WorkflowNode, crate::app::i18n::workflow::Feedback> {
    let raw_node = blank_node_value(block_type, node_id, position, z_index);
    let parsed: DifyNode = serde_yaml::from_value(raw_node.clone()).map_err(|error| {
        crate::app::i18n::workflow::Feedback::new("创建节点失败: %s", vec![error.to_string()])
    })?;
    Ok(workflow_node_from_dify(&parsed, raw_node))
}

pub fn default_node_data_yaml(
    block_type: &str,
) -> Result<String, crate::app::i18n::workflow::Feedback> {
    yaml_string_for_editor(&default_node_data_value(block_type))
}

pub fn node_data_yaml(node: &WorkflowNode) -> Result<String, crate::app::i18n::workflow::Feedback> {
    let data = node
        .raw_node
        .as_mapping()
        .and_then(|map| map.get(&key_value("data")))
        .cloned()
        .unwrap_or_else(|| yaml_map(vec![]));
    yaml_string_for_editor(&data)
}

pub fn rebuild_node_from_parts(
    node: &WorkflowNode,
    title: &str,
    description: &str,
    raw_data_yaml: &str,
) -> Result<WorkflowNode, crate::app::i18n::workflow::Feedback> {
    let data_value = if raw_data_yaml.trim().is_empty() {
        yaml_map(vec![])
    } else {
        serde_yaml::from_str::<Value>(raw_data_yaml).map_err(|error| {
            crate::app::i18n::workflow::Feedback::new(
                "节点 data YAML 解析失败: %s",
                vec![error.to_string()],
            )
        })?
    };

    if !data_value.is_mapping() {
        return Err(crate::app::i18n::workflow::Feedback::new(
            "节点 data 必须是对象映射（YAML map）",
            vec![],
        ));
    }

    let mut raw_node = ensure_root_mapping(node.raw_node.clone());
    let raw_map = ensure_value_mapping(&mut raw_node);
    set_mapping_value(raw_map, "selected", Value::Bool(node.selected));

    let data_entry = raw_map.entry(key_value("data")).or_insert_with(|| yaml_map(vec![]));
    *data_entry = data_value;
    let data_map = ensure_value_mapping(data_entry);

    set_mapping_value(data_map, "title", Value::String(title.trim().to_string()));
    set_mapping_value(data_map, "desc", Value::String(description.trim().to_string()));
    set_mapping_value(data_map, "type", Value::String(node.block_type.clone()));
    set_mapping_value(data_map, "selected", Value::Bool(node.selected));

    let parsed: DifyNode = serde_yaml::from_value(raw_node.clone()).map_err(|error| {
        crate::app::i18n::workflow::Feedback::new("重建节点失败: %s", vec![error.to_string()])
    })?;
    Ok(workflow_node_from_dify(&parsed, raw_node))
}

pub fn load_document_from_path(
    path: &str,
) -> Result<LoadedWorkflow, crate::app::i18n::workflow::Feedback> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let text = std::fs::read_to_string(path).map_err(|error| {
            crate::app::i18n::workflow::Feedback::new(
                "读取工作流文件失败: %s",
                vec![error.to_string()],
            )
        })?;
        return load_document_from_text(Some(path.to_string()), text);
    }

    #[cfg(target_arch = "wasm32")]
    {
        let _ = path;
        Err(crate::app::i18n::workflow::Feedback::new(
            "Web 平台暂不支持直接读取本地工作流文件",
            vec![],
        ))
    }
}

pub fn load_document_from_text(
    source_path: Option<String>,
    text: String,
) -> Result<LoadedWorkflow, crate::app::i18n::workflow::Feedback> {
    let raw_root: Value = serde_yaml::from_str(&text).map_err(|error| {
        crate::app::i18n::workflow::Feedback::new("解析 Dify DSL 失败: %s", vec![error.to_string()])
    })?;
    load_document_from_value(source_path, raw_root)
}
