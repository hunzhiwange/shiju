//! 验证聊天工具渲染分派。
//! 测试保护不同工具输出选择正确视图组件。

use super::tools::{
    SharedToolRenderKind, explore_summary_expanded, explore_summary_is_running,
    shared_tool_render_kind,
};
use std::collections::HashSet;

#[test]
fn shared_tool_render_kind_routes_structured_tool_cards() {
    assert_eq!(shared_tool_render_kind("tool file_write\n{}"), Some(SharedToolRenderKind::Files));
    assert_eq!(shared_tool_render_kind("tool edit\n{}"), Some(SharedToolRenderKind::Files));
    assert_eq!(
        shared_tool_render_kind("tool notebook_edit\n{}"),
        Some(SharedToolRenderKind::Files)
    );
    assert_eq!(shared_tool_render_kind("tool grep\n{}"), Some(SharedToolRenderKind::Files));
    assert_eq!(shared_tool_render_kind("tool glob\n{}"), Some(SharedToolRenderKind::Files));
    assert_eq!(shared_tool_render_kind("tool lsp\n{}"), Some(SharedToolRenderKind::Lsp));
    assert_eq!(shared_tool_render_kind("tool web_fetch\n{}"), Some(SharedToolRenderKind::Web));
    assert_eq!(shared_tool_render_kind("tool web_search\n{}"), Some(SharedToolRenderKind::Web));
    assert_eq!(shared_tool_render_kind("tool image_info\n{}"), Some(SharedToolRenderKind::Bash));
    assert_eq!(shared_tool_render_kind("tool AgentTool\n{}"), Some(SharedToolRenderKind::Advanced));
    assert_eq!(shared_tool_render_kind("tool browser\n{}"), Some(SharedToolRenderKind::Advanced));
    assert_eq!(
        shared_tool_render_kind("tool browser_open\n{}"),
        Some(SharedToolRenderKind::Advanced)
    );
    assert_eq!(shared_tool_render_kind("tool skill\n{}"), Some(SharedToolRenderKind::Skill));
    assert_eq!(
        shared_tool_render_kind("tool workflow_node\n{}"),
        Some(SharedToolRenderKind::Workflow)
    );
}

#[test]
fn shared_tool_render_kind_keeps_existing_specialized_routes() {
    assert_eq!(shared_tool_render_kind("tool shell\n{}"), Some(SharedToolRenderKind::Bash));
    assert_eq!(shared_tool_render_kind("tool execute\n{}"), Some(SharedToolRenderKind::Bash));
    assert_eq!(
        shared_tool_render_kind("tool AskUserQuestion\n{}"),
        Some(SharedToolRenderKind::Question)
    );
    assert_eq!(
        shared_tool_render_kind("tool apply_patch\n{}"),
        Some(SharedToolRenderKind::ApplyPatch)
    );
    assert_eq!(shared_tool_render_kind("tool unknown_tool\n{}"), Some(SharedToolRenderKind::Text));
    assert_eq!(
        shared_tool_render_kind("tool task_complete\n{}"),
        Some(SharedToolRenderKind::Advanced)
    );
}

#[test]
fn completed_explore_summary_does_not_expand_without_manual_override() {
    let key = 99_u64;

    assert!(!explore_summary_expanded(false, key, &HashSet::new()));
}

#[test]
fn running_explore_summary_requires_manual_expand() {
    let key = 99_u64;
    let mut expanded = HashSet::new();

    assert!(!explore_summary_expanded(true, key, &expanded));

    expanded.insert(key);
    assert!(explore_summary_expanded(true, key, &expanded));
}

#[test]
fn visible_follow_up_block_closes_running_explore_summary() {
    assert!(!explore_summary_is_running(true, false, true));
}

#[test]
fn hidden_think_only_keeps_explore_running_until_visible_boundary() {
    assert!(explore_summary_is_running(false, true, false));
    assert!(!explore_summary_is_running(false, true, true));
}
