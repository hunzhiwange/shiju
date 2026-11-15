//! # Workflow 标识生成
//!
//! 该模块生成应用、节点、变量、分支、条件和连线等运行时 ID，并做句柄规范化。

use super::*;

pub fn fitted_viewport(
    document: &WorkflowDocument,
    window_size: (f32, f32),
) -> (Vector, f32) {
    let Some(bounds) = document.bounds() else {
        return (Vector::new(120.0, 120.0), 1.0);
    };

    let usable_width = (window_size.0 - 380.0).max(320.0);
    let usable_height = (window_size.1 - 220.0).max(260.0);
    let scale_x = usable_width / bounds.width.max(1.0);
    let scale_y = usable_height / bounds.height.max(1.0);
    let zoom = (scale_x.min(scale_y) * 0.92).clamp(0.1, 4.0);

    let world_center = Point::new(bounds.x + bounds.width / 2.0, bounds.y + bounds.height / 2.0);
    let screen_center = Vector::new(usable_width / 2.0 + 24.0, usable_height / 2.0 + 24.0);

    (screen_center - Vector::new(world_center.x * zoom, world_center.y * zoom), zoom)
}

pub fn generate_app_id() -> String {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    format!("workflow-app-{}", suffix)
}

pub fn generate_node_id(block_type: &str) -> String {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    format!("{}-node-{}", sanitize_handle_id(block_type), suffix)
}

pub fn generate_variable_id(prefix: &str) -> String {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    format!("{}-var-{}", sanitize_handle_id(prefix), suffix)
}

pub fn generate_start_variable_name() -> String {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    format!("input_{}", suffix)
}

pub fn generate_case_id() -> String {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    format!("case-{}", suffix)
}

pub fn generate_condition_id() -> String {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    format!("condition-{}", suffix)
}

pub fn generate_prompt_item_id(role: &str) -> String {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    format!("{}-prompt-{}", sanitize_handle_id(role), suffix)
}

pub fn normalize_connection_endpoints(
    first: &WorkflowConnectionEndpoint,
    second: &WorkflowConnectionEndpoint,
) -> Option<(WorkflowConnectionEndpoint, WorkflowConnectionEndpoint)> {
    match (first.kind, second.kind) {
        (WorkflowHandleKind::Source, WorkflowHandleKind::Target) => {
            Some((first.clone(), second.clone()))
        }
        (WorkflowHandleKind::Target, WorkflowHandleKind::Source) => {
            Some((second.clone(), first.clone()))
        }
        _ => None,
    }
}

pub fn generate_edge_id(
    source: &WorkflowConnectionEndpoint,
    target: &WorkflowConnectionEndpoint,
) -> String {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);

    format!(
        "manual-{}-{}-{}-{}-{}",
        source.node_id,
        sanitize_handle_id(&source.handle_id),
        target.node_id,
        sanitize_handle_id(&target.handle_id),
        suffix,
    )
}

pub fn sanitize_handle_id(handle_id: &str) -> String {
    handle_id.chars().map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' }).collect()
}
