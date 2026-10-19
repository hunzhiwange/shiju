//! # Workflow 模型加载
//!
//! 该模块将原始 Dify DSL 值解析为内部工作流文档、节点、连线和变量结构。

use super::*;

pub fn load_document_from_value(
    source_path: Option<String>,
    raw_root: Value,
) -> Result<LoadedWorkflow, String> {
    let parsed: DifyWorkflowFile = serde_yaml::from_value(raw_root.clone())
        .map_err(|error| format!("解析 Dify DSL 失败: {error}"))?;

    let app_meta = workflow_app_meta_from_dify(parsed.app.as_ref());

    let source_name = if app_meta.name.trim().is_empty() {
        source_path
            .as_deref()
            .and_then(file_stem_string)
            .unwrap_or_else(|| "工作流示例".to_string())
    } else {
        app_meta.name.clone()
    };

    let graph = parsed
        .workflow
        .as_ref()
        .map(|workflow| &workflow.graph)
        .or(parsed.graph.as_ref())
        .ok_or_else(|| "未找到 workflow.graph 节点，无法构建画布".to_string())?;

    let raw_graph = raw_graph_value(&raw_root);
    let raw_nodes = raw_graph
        .and_then(|graph| graph.get("nodes"))
        .and_then(Value::as_sequence)
        .cloned()
        .unwrap_or_default();
    let raw_edges = raw_graph
        .and_then(|graph| graph.get("edges"))
        .and_then(Value::as_sequence)
        .cloned()
        .unwrap_or_default();
    let raw_workflow = raw_workflow_value(&raw_root);
    let raw_environment_variables = raw_workflow
        .and_then(|workflow| workflow.get("environment_variables"))
        .and_then(Value::as_sequence)
        .cloned()
        .unwrap_or_default();
    let raw_conversation_variables = raw_workflow
        .and_then(|workflow| workflow.get("conversation_variables"))
        .and_then(Value::as_sequence)
        .cloned()
        .unwrap_or_default();

    let nodes = graph
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| {
            workflow_node_from_dify(node, raw_nodes.get(index).cloned().unwrap_or(Value::Null))
        })
        .collect::<Vec<_>>();

    let node_types = nodes
        .iter()
        .map(|node| (node.id.as_str(), node.block_type.as_str()))
        .collect::<HashMap<_, _>>();

    let edges = graph
        .edges
        .iter()
        .enumerate()
        .map(|(index, edge)| {
            workflow_edge_from_dify(
                edge,
                index,
                &node_types,
                raw_edges.get(index).cloned().unwrap_or(Value::Null),
            )
        })
        .collect::<Vec<_>>();

    let viewport = graph.viewport.as_ref().map(workflow_viewport_from_dify).unwrap_or_default();

    let environment_variables = parsed
        .workflow
        .as_ref()
        .map(|workflow| {
            workflow
                .environment_variables
                .iter()
                .enumerate()
                .map(|(index, variable)| {
                    workflow_environment_variable_from_dify(
                        variable,
                        raw_environment_variables.get(index).cloned().unwrap_or(Value::Null),
                    )
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let conversation_variables = parsed
        .workflow
        .as_ref()
        .map(|workflow| {
            workflow
                .conversation_variables
                .iter()
                .enumerate()
                .map(|(index, variable)| {
                    workflow_conversation_variable_from_dify(
                        variable,
                        raw_conversation_variables.get(index).cloned().unwrap_or(Value::Null),
                    )
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    Ok(LoadedWorkflow {
        local_uuid: None,
        source_path,
        source_name: source_name.clone(),
        app_meta: WorkflowAppMeta { name: source_name.clone(), ..app_meta },
        document: WorkflowDocument { name: source_name, nodes, edges, viewport },
        environment_variables,
        conversation_variables,
        had_viewport: graph.viewport.is_some(),
        raw_root,
    })
}

pub fn serialize_workflow_yaml(
    app_meta: &WorkflowAppMeta,
    document: &WorkflowDocument,
    environment_variables: &[WorkflowEnvironmentVariable],
    conversation_variables: &[WorkflowConversationVariable],
    raw_root: &Value,
    viewport: WorkflowViewport,
) -> Result<String, String> {
    let root = patch_root_for_save(
        app_meta,
        document,
        environment_variables,
        conversation_variables,
        raw_root,
        viewport,
    )?;
    serde_yaml::to_string(&root).map_err(|error| format!("序列化工作流 yml 失败: {error}"))
}

pub fn suggested_workflow_file_name(title: &str) -> String {
    let stem = title
        .trim()
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ('\u{4e00}'..='\u{9fff}').contains(&ch) {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_string();

    let stem = if stem.is_empty() { "workflow_app".to_string() } else { stem };
    format!("{}.yml", stem)
}

fn file_stem_string(path: &str) -> Option<String> {
    Path::new(path).file_stem().and_then(|stem| stem.to_str()).map(|stem| stem.to_string())
}

fn workflow_viewport_from_dify(viewport: &DifyViewport) -> WorkflowViewport {
    WorkflowViewport { x: viewport.x, y: viewport.y, zoom: viewport.zoom.max(0.1) }
}

pub(super) fn workflow_node_from_dify(node: &DifyNode, raw_node: Value) -> WorkflowNode {
    let position = node.position_absolute.as_ref().or(node.position.as_ref());
    let position = position.cloned().unwrap_or_default();
    let block_type = if node.data.block_type.trim().is_empty() {
        node.renderer_type.clone().unwrap_or_else(|| "custom".to_string())
    } else {
        node.data.block_type.clone()
    };

    let title = if node.data.title.trim().is_empty() {
        pretty_block_type(&block_type)
    } else {
        node.data.title.trim().to_string()
    };
    let default_size = default_node_size(&block_type);
    let height = node.height.unwrap_or(default_size.height);
    let height = if block_type == "start" {
        height.max(workflow_start_node_min_height(&raw_node))
    } else {
        height.max(48.0)
    };

    WorkflowNode {
        id: node.id.clone(),
        block_type,
        title,
        description: node.data.desc.trim().to_string(),
        position: Point::new(position.x, position.y),
        size: Size::new(node.width.unwrap_or(default_size.width).max(120.0), height),
        parent_id: node.parent_id.clone(),
        selected: node.selected.or(node.data.selected).unwrap_or(false),
        source_side: parse_handle_side(node.source_position.as_deref(), WorkflowHandleSide::Right),
        target_side: parse_handle_side(node.target_position.as_deref(), WorkflowHandleSide::Left),
        source_handles: build_source_handles(
            &node.data,
            &node.renderer_type,
            &node.data.block_type,
        ),
        target_handles: build_target_handles(&node.renderer_type, &node.data.block_type),
        z_index: node.z_index.unwrap_or(0.0),
        raw_node,
    }
}

pub(super) fn workflow_edge_from_dify(
    edge: &DifyEdge,
    index: usize,
    node_types: &HashMap<&str, &str>,
    raw_edge: Value,
) -> WorkflowEdge {
    WorkflowEdge {
        id: edge
            .id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| generated_edge_id(edge, index)),
        source: edge.source.clone(),
        target: edge.target.clone(),
        source_handle: edge.source_handle.clone(),
        target_handle: edge.target_handle.clone(),
        source_type: edge
            .data
            .as_ref()
            .map(|data| data.source_type.clone())
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| {
                node_types.get(edge.source.as_str()).copied().unwrap_or("custom").to_string()
            }),
        target_type: edge
            .data
            .as_ref()
            .map(|data| data.target_type.clone())
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| {
                node_types.get(edge.target.as_str()).copied().unwrap_or("custom").to_string()
            }),
        selected: edge.selected.unwrap_or(false),
        z_index: edge.z_index.unwrap_or(0.0),
        raw_edge,
    }
}

fn generated_edge_id(edge: &DifyEdge, index: usize) -> String {
    let source_handle = edge.source_handle.as_deref().unwrap_or("source");
    let target_handle = edge.target_handle.as_deref().unwrap_or("target");
    format!("{}-{}-{}-{}-{}", edge.source, source_handle, edge.target, target_handle, index + 1)
}

pub(super) fn parse_handle_side(
    value: Option<&str>,
    default: WorkflowHandleSide,
) -> WorkflowHandleSide {
    match value.unwrap_or_default().trim().to_ascii_lowercase().as_str() {
        "left" => WorkflowHandleSide::Left,
        "right" => WorkflowHandleSide::Right,
        "top" => WorkflowHandleSide::Top,
        "bottom" => WorkflowHandleSide::Bottom,
        _ => default,
    }
}

pub(super) fn build_source_handles(
    data: &DifyNodeData,
    renderer_type: &Option<String>,
    block_type: &str,
) -> Vec<WorkflowHandle> {
    let effective_type = normalized_block_type(renderer_type, block_type);

    let branch_cases = if data.cases.is_empty() { &data.classes } else { &data.cases };
    let mut handles = if !branch_cases.is_empty() {
        let handles = branch_cases
            .iter()
            .enumerate()
            .filter_map(|(index, case)| {
                let handle_id = case
                    .case_id
                    .as_deref()
                    .or(case.id.as_deref())
                    .map(str::trim)
                    .filter(|value| !value.is_empty())?;

                Some(WorkflowHandle {
                    id: handle_id.to_string(),
                    label: pretty_handle_label(handle_id, index),
                    kind: WorkflowHandleKind::Source,
                })
            })
            .collect::<Vec<_>>();

        if handles.is_empty() {
            if matches!(effective_type.as_str(), "end") {
                Vec::new()
            } else {
                vec![WorkflowHandle {
                    id: "source".to_string(),
                    label: String::new(),
                    kind: WorkflowHandleKind::Source,
                }]
            }
        } else {
            handles
        }
    } else if matches!(effective_type.as_str(), "end") {
        Vec::new()
    } else {
        vec![WorkflowHandle {
            id: "source".to_string(),
            label: String::new(),
            kind: WorkflowHandleKind::Source,
        }]
    };

    if data.error_strategy.trim() == "fail-branch"
        && !handles.iter().any(|handle| handle.id == "fail-branch")
    {
        handles.push(WorkflowHandle {
            id: "fail-branch".to_string(),
            label: pretty_handle_label("fail-branch", handles.len()),
            kind: WorkflowHandleKind::Source,
        });
    }

    handles
}

pub(super) fn build_target_handles(
    renderer_type: &Option<String>,
    block_type: &str,
) -> Vec<WorkflowHandle> {
    let effective_type = normalized_block_type(renderer_type, block_type);

    if matches!(
        effective_type.as_str(),
        "start"
            | "trigger-webhook"
            | "trigger-schedule"
            | "trigger-plugin"
            | "iteration-start"
            | "loop-start"
    ) {
        Vec::new()
    } else {
        vec![WorkflowHandle {
            id: "target".to_string(),
            label: String::new(),
            kind: WorkflowHandleKind::Target,
        }]
    }
}

pub(super) fn normalized_block_type(renderer_type: &Option<String>, block_type: &str) -> String {
    if block_type.trim().is_empty() {
        renderer_type.clone().unwrap_or_else(|| "custom".to_string())
    } else {
        block_type.trim().to_string()
    }
}

pub(super) fn pretty_handle_label(handle_id: &str, index: usize) -> String {
    match handle_id {
        "true" => "是".to_string(),
        "false" => "否".to_string(),
        "fail-branch" => "异常".to_string(),
        "source" | "target" => String::new(),
        other if other.len() > 20 || other.contains('-') => format!("分支 {}", index + 1),
        other => other.to_string(),
    }
}

pub(super) fn workflow_app_meta_from_dify(app: Option<&DifyApp>) -> WorkflowAppMeta {
    let Some(app) = app else {
        return WorkflowAppMeta::default();
    };

    WorkflowAppMeta {
        name: app.name.clone().unwrap_or_else(|| "未命名应用".to_string()),
        description: app.description.clone().unwrap_or_default(),
        icon: app.icon.clone().unwrap_or_else(|| "🤖".to_string()),
        icon_background: app.icon_background.clone().unwrap_or_else(|| "#FFEAD5".to_string()),
        mode: app.mode.clone().unwrap_or_else(|| "advanced-chat".to_string()),
        use_icon_as_answer_icon: app.use_icon_as_answer_icon.unwrap_or(false),
        max_active_requests: app.max_active_requests.unwrap_or(0).max(0) as u32,
    }
}

pub(super) fn workflow_environment_variable_from_dify(
    variable: &DifyEnvironmentVariable,
    raw_variable: Value,
) -> WorkflowEnvironmentVariable {
    WorkflowEnvironmentVariable {
        id: variable
            .id
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or("env-var")
            .to_string(),
        name: if variable.name.trim().is_empty() {
            "environment_var".to_string()
        } else {
            variable.name.trim().to_string()
        },
        value_type: if variable.value_type.trim().is_empty() {
            "string".to_string()
        } else {
            variable.value_type.trim().to_string()
        },
        value: variable.value.clone(),
        description: variable.description.trim().to_string(),
        raw_variable,
    }
}

pub(super) fn workflow_conversation_variable_from_dify(
    variable: &DifyConversationVariable,
    raw_variable: Value,
) -> WorkflowConversationVariable {
    WorkflowConversationVariable {
        id: variable
            .id
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or("conversation-var")
            .to_string(),
        name: if variable.name.trim().is_empty() {
            "conversation_var".to_string()
        } else {
            variable.name.trim().to_string()
        },
        value_type: if variable.value_type.trim().is_empty() {
            "string".to_string()
        } else {
            variable.value_type.trim().to_string()
        },
        value: variable.value.clone(),
        description: variable.description.trim().to_string(),
        raw_variable,
    }
}
