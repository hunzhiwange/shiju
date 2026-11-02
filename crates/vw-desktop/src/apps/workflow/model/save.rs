//! # Workflow 模型保存
//!
//! 该模块负责将当前工作流状态回填为可保存的 DSL 根结构与 YAML 文本。

use super::*;

pub(super) fn blank_workflow_root(app_meta: &WorkflowAppMeta) -> Value {
    let start_node = yaml_map(vec![
        (
            "data",
            yaml_map(vec![
                ("desc", Value::String(String::new())),
                ("selected", Value::Bool(false)),
                ("title", Value::String("开始".to_string())),
                ("type", Value::String("start".to_string())),
                ("variables", Value::Sequence(Vec::new())),
            ]),
        ),
        ("height", yaml_value(120.0_f64)),
        ("id", Value::String("start-node".to_string())),
        ("position", point_value(-180.0, 120.0)),
        ("positionAbsolute", point_value(-180.0, 120.0)),
        ("selected", Value::Bool(false)),
        ("sourcePosition", Value::String("right".to_string())),
        ("targetPosition", Value::String("left".to_string())),
        ("type", Value::String("custom".to_string())),
        ("width", yaml_value(240.0_f64)),
    ]);

    let answer_node = yaml_map(vec![
        (
            "data",
            yaml_map(vec![
                ("answer", Value::String("你好，这是一份新的 Dify 工作流。".to_string())),
                ("desc", Value::String("".to_string())),
                ("selected", Value::Bool(false)),
                ("title", Value::String("回复".to_string())),
                ("type", Value::String("answer".to_string())),
                ("variables", Value::Sequence(Vec::new())),
            ]),
        ),
        ("height", yaml_value(116.0_f64)),
        ("id", Value::String("answer-node".to_string())),
        ("position", point_value(140.0, 120.0)),
        ("positionAbsolute", point_value(140.0, 120.0)),
        ("selected", Value::Bool(false)),
        ("sourcePosition", Value::String("right".to_string())),
        ("targetPosition", Value::String("left".to_string())),
        ("type", Value::String("custom".to_string())),
        ("width", yaml_value(240.0_f64)),
    ]);

    let edge = yaml_map(vec![
        (
            "data",
            yaml_map(vec![
                ("isInLoop", Value::Bool(false)),
                ("sourceType", Value::String("start".to_string())),
                ("targetType", Value::String("answer".to_string())),
            ]),
        ),
        ("id", Value::String("start-node-source-answer-node-target".to_string())),
        ("selected", Value::Bool(false)),
        ("source", Value::String("start-node".to_string())),
        ("sourceHandle", Value::String("source".to_string())),
        ("target", Value::String("answer-node".to_string())),
        ("targetHandle", Value::String("target".to_string())),
        ("type", Value::String("custom".to_string())),
        ("zIndex", yaml_value(0.0_f64)),
    ]);

    yaml_map(vec![
        (
            "app",
            yaml_map(vec![
                ("description", Value::String(app_meta.description.clone())),
                ("icon", Value::String(app_meta.icon.clone())),
                ("icon_background", Value::String(app_meta.icon_background.clone())),
                ("mode", Value::String(app_meta.mode.clone())),
                ("name", Value::String(app_meta.name.clone())),
                ("use_icon_as_answer_icon", Value::Bool(app_meta.use_icon_as_answer_icon)),
                ("max_active_requests", yaml_value(app_meta.max_active_requests as u64)),
            ]),
        ),
        ("dependencies", Value::Sequence(Vec::new())),
        ("kind", Value::String("app".to_string())),
        ("version", Value::String("0.5.0".to_string())),
        (
            "workflow",
            yaml_map(vec![
                ("conversation_variables", Value::Sequence(Vec::new())),
                ("environment_variables", Value::Sequence(Vec::new())),
                ("features", yaml_map(vec![])),
                (
                    "graph",
                    yaml_map(vec![
                        ("edges", Value::Sequence(vec![edge])),
                        ("nodes", Value::Sequence(vec![start_node, answer_node])),
                        ("viewport", viewport_value(160.0, 120.0, 1.0)),
                    ]),
                ),
            ]),
        ),
    ])
}

pub(super) fn patch_root_for_save(
    app_meta: &WorkflowAppMeta,
    document: &WorkflowDocument,
    environment_variables: &[WorkflowEnvironmentVariable],
    conversation_variables: &[WorkflowConversationVariable],
    raw_root: &Value,
    viewport: WorkflowViewport,
) -> Result<Value, crate::app::i18n::workflow::Feedback> {
    let mut root = ensure_root_mapping(raw_root.clone());
    let root_map = root.as_mapping_mut().ok_or_else(|| {
        crate::app::i18n::workflow::Feedback::new("工作流根节点必须是一个对象", vec![])
    })?;

    let app_value =
        root_map.entry(Value::String("app".to_string())).or_insert_with(|| yaml_map(vec![]));
    let app_map = ensure_value_mapping(app_value);
    set_mapping_value(app_map, "name", Value::String(app_meta.name.clone()));
    set_mapping_value(app_map, "description", Value::String(app_meta.description.clone()));
    set_mapping_value(app_map, "icon", Value::String(app_meta.icon.clone()));
    set_mapping_value(app_map, "icon_background", Value::String(app_meta.icon_background.clone()));
    set_mapping_value(app_map, "mode", Value::String(app_meta.mode.clone()));
    set_mapping_value(
        app_map,
        "use_icon_as_answer_icon",
        Value::Bool(app_meta.use_icon_as_answer_icon),
    );
    set_mapping_value(
        app_map,
        "max_active_requests",
        yaml_value(app_meta.max_active_requests as u64),
    );

    let workflow_value =
        root_map.entry(Value::String("workflow".to_string())).or_insert_with(|| yaml_map(vec![]));
    let workflow_map = ensure_value_mapping(workflow_value);
    let graph_value =
        workflow_map.entry(Value::String("graph".to_string())).or_insert_with(|| yaml_map(vec![]));
    let graph_map = ensure_value_mapping(graph_value);
    set_mapping_value(
        graph_map,
        "nodes",
        Value::Sequence(document.nodes.iter().map(saved_node_value).collect()),
    );
    set_mapping_value(
        graph_map,
        "edges",
        Value::Sequence(document.edges.iter().map(saved_edge_value).collect()),
    );
    set_mapping_value(graph_map, "viewport", viewport_value(viewport.x, viewport.y, viewport.zoom));
    set_mapping_value(
        workflow_map,
        "environment_variables",
        Value::Sequence(
            environment_variables.iter().map(saved_environment_variable_value).collect(),
        ),
    );
    set_mapping_value(
        workflow_map,
        "conversation_variables",
        Value::Sequence(
            conversation_variables.iter().map(saved_conversation_variable_value).collect(),
        ),
    );
    root_map.remove(&key_value("graph"));

    Ok(root)
}

pub(super) fn saved_node_value(node: &WorkflowNode) -> Value {
    let mut raw = ensure_root_mapping(node.raw_node.clone());
    let raw_map = ensure_value_mapping(&mut raw);

    set_mapping_value(raw_map, "id", Value::String(node.id.clone()));
    set_mapping_value(raw_map, "position", point_value(node.position.x, node.position.y));
    set_mapping_value(raw_map, "positionAbsolute", point_value(node.position.x, node.position.y));
    set_mapping_value(raw_map, "width", yaml_value(node.size.width as f64));
    set_mapping_value(raw_map, "height", yaml_value(node.size.height as f64));
    set_optional_string(raw_map, "parentId", node.parent_id.as_deref());
    set_mapping_value(raw_map, "selected", Value::Bool(node.selected));
    set_mapping_value(
        raw_map,
        "sourcePosition",
        Value::String(handle_side_name(node.source_side).to_string()),
    );
    set_mapping_value(
        raw_map,
        "targetPosition",
        Value::String(handle_side_name(node.target_side).to_string()),
    );
    set_mapping_value(raw_map, "zIndex", yaml_value(node.z_index as f64));

    let data_value =
        raw_map.entry(Value::String("data".to_string())).or_insert_with(|| yaml_map(vec![]));
    let data_map = ensure_value_mapping(data_value);
    set_mapping_value(data_map, "title", Value::String(node.title.clone()));
    set_mapping_value(data_map, "desc", Value::String(node.description.clone()));
    set_mapping_value(data_map, "type", Value::String(node.block_type.clone()));
    set_mapping_value(data_map, "selected", Value::Bool(node.selected));

    raw
}

pub(super) fn saved_edge_value(edge: &WorkflowEdge) -> Value {
    let mut raw = ensure_root_mapping(edge.raw_edge.clone());
    let raw_map = ensure_value_mapping(&mut raw);

    set_mapping_value(raw_map, "id", Value::String(edge.id.clone()));
    set_mapping_value(raw_map, "source", Value::String(edge.source.clone()));
    set_mapping_value(raw_map, "target", Value::String(edge.target.clone()));
    set_optional_string(raw_map, "sourceHandle", edge.source_handle.as_deref());
    set_optional_string(raw_map, "targetHandle", edge.target_handle.as_deref());
    set_mapping_value(raw_map, "selected", Value::Bool(edge.selected));
    set_mapping_value(raw_map, "zIndex", yaml_value(edge.z_index as f64));
    if !raw_map.contains_key(&key_value("type")) {
        set_mapping_value(raw_map, "type", Value::String("custom".to_string()));
    }

    let data_value =
        raw_map.entry(Value::String("data".to_string())).or_insert_with(|| yaml_map(vec![]));
    let data_map = ensure_value_mapping(data_value);
    set_mapping_value(data_map, "sourceType", Value::String(edge.source_type.clone()));
    set_mapping_value(data_map, "targetType", Value::String(edge.target_type.clone()));

    raw
}

pub(super) fn saved_environment_variable_value(variable: &WorkflowEnvironmentVariable) -> Value {
    let mut raw = ensure_root_mapping(variable.raw_variable.clone());
    let raw_map = ensure_value_mapping(&mut raw);

    set_mapping_value(raw_map, "id", Value::String(variable.id.clone()));
    set_mapping_value(raw_map, "name", Value::String(variable.name.clone()));
    set_mapping_value(raw_map, "value_type", Value::String(variable.value_type.clone()));
    set_mapping_value(raw_map, "value", variable.value.clone());
    set_mapping_value(raw_map, "description", Value::String(variable.description.clone()));

    raw
}

pub(super) fn saved_conversation_variable_value(variable: &WorkflowConversationVariable) -> Value {
    let mut raw = ensure_root_mapping(variable.raw_variable.clone());
    let raw_map = ensure_value_mapping(&mut raw);

    set_mapping_value(raw_map, "id", Value::String(variable.id.clone()));
    set_mapping_value(raw_map, "name", Value::String(variable.name.clone()));
    set_mapping_value(raw_map, "value_type", Value::String(variable.value_type.clone()));
    set_mapping_value(raw_map, "value", variable.value.clone());
    set_mapping_value(raw_map, "description", Value::String(variable.description.clone()));

    raw
}

pub(super) fn raw_graph_value(root: &Value) -> Option<&Mapping> {
    let root_map = root.as_mapping()?;
    if let Some(workflow) = root_map.get(&key_value("workflow")) {
        let workflow_map = workflow.as_mapping()?;
        return workflow_map.get(&key_value("graph")).and_then(Value::as_mapping);
    }

    root_map.get(&key_value("graph")).and_then(Value::as_mapping)
}

pub(super) fn raw_workflow_value(root: &Value) -> Option<&Mapping> {
    root.as_mapping()?.get(&key_value("workflow")).and_then(Value::as_mapping)
}

pub(super) fn yaml_string_for_editor(
    value: &Value,
) -> Result<String, crate::app::i18n::workflow::Feedback> {
    let yaml = serde_yaml::to_string(value).map_err(|error| {
        crate::app::i18n::workflow::Feedback::new("生成 YAML 文本失败: %s", vec![error.to_string()])
    })?;
    Ok(yaml.strip_prefix("---\n").unwrap_or(&yaml).to_string())
}

pub(super) fn ensure_root_mapping(value: Value) -> Value {
    if value.is_mapping() { value } else { yaml_map(vec![]) }
}

pub(super) fn ensure_value_mapping(value: &mut Value) -> &mut Mapping {
    if !value.is_mapping() {
        *value = yaml_map(vec![]);
    }
    value.as_mapping_mut().expect("mapping just initialized")
}

pub(super) fn set_mapping_value(map: &mut Mapping, key: &str, value: Value) {
    map.insert(Value::String(key.to_string()), value);
}

pub(super) fn set_optional_string(map: &mut Mapping, key: &str, value: Option<&str>) {
    match value.filter(|value| !value.trim().is_empty()) {
        Some(value) => set_mapping_value(map, key, Value::String(value.to_string())),
        None => {
            map.remove(&key_value(key));
        }
    }
}

pub(super) fn key_value(key: &str) -> Value {
    Value::String(key.to_string())
}

pub(super) fn point_value(x: f32, y: f32) -> Value {
    yaml_map(vec![("x", yaml_value(x as f64)), ("y", yaml_value(y as f64))])
}

pub(super) fn viewport_value(x: f32, y: f32, zoom: f32) -> Value {
    yaml_map(vec![
        ("x", yaml_value(x as f64)),
        ("y", yaml_value(y as f64)),
        ("zoom", yaml_value(zoom as f64)),
    ])
}

pub(crate) fn yaml_map(entries: Vec<(&str, Value)>) -> Value {
    let mut map = Mapping::new();
    for (key, value) in entries {
        map.insert(Value::String(key.to_string()), value);
    }
    Value::Mapping(map)
}

pub(super) fn yaml_value<T: Serialize>(value: T) -> Value {
    serde_yaml::to_value(value).unwrap_or(Value::Null)
}

pub(super) fn handle_side_name(side: WorkflowHandleSide) -> &'static str {
    match side {
        WorkflowHandleSide::Left => "left",
        WorkflowHandleSide::Right => "right",
        WorkflowHandleSide::Top => "top",
        WorkflowHandleSide::Bottom => "bottom",
    }
}

pub(super) fn is_chat_mode(mode: &str) -> bool {
    !matches!(mode.trim(), "workflow")
}
