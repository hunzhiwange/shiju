use super::tool_renderer::{SharedToolRenderKind, shared_tool_render_kind};
#[test]
fn shared_tool_render_kind_routes_known_tools() {
    assert_eq!(shared_tool_render_kind("tool bash\n{}"), Some(SharedToolRenderKind::Bash));
    assert_eq!(shared_tool_render_kind("tool skill\n{}"), Some(SharedToolRenderKind::Skill));
    assert_eq!(
        shared_tool_render_kind("tool workflow_node\n{}"),
        Some(SharedToolRenderKind::Workflow)
    );
    assert_eq!(shared_tool_render_kind("tool custom_tool\n{}"), Some(SharedToolRenderKind::Text));
}

#[test]
fn shared_tool_render_kind_routes_aliases_and_special_tools() {
    assert_eq!(shared_tool_render_kind("tool glob_search\n{}"), Some(SharedToolRenderKind::Files));
    assert_eq!(
        shared_tool_render_kind("tool browser_open\n{}"),
        Some(SharedToolRenderKind::Advanced)
    );
    assert_eq!(shared_tool_render_kind("tool mcp_demo\n{}"), Some(SharedToolRenderKind::Mcp));
    assert_eq!(shared_tool_render_kind("tool MCP\n{}"), Some(SharedToolRenderKind::Mcp));
    assert_eq!(shared_tool_render_kind("invalid"), None);
}

#[test]
fn shared_tool_render_kind_routes_all_supported_kinds() {
    for (raw, expected) in [
        ("tool shell\n{}", SharedToolRenderKind::Bash),
        ("tool Config\n{}", SharedToolRenderKind::Config),
        ("tool todowrite\n{}", SharedToolRenderKind::TodoCompact),
        ("tool todoread\n{}", SharedToolRenderKind::TodoRead),
        ("tool apply_patch\n{}", SharedToolRenderKind::ApplyPatch),
        ("tool file_read\n{}", SharedToolRenderKind::Read),
        ("tool pdf_read\n{}", SharedToolRenderKind::Read),
        ("tool read_file\n{}", SharedToolRenderKind::Read),
        ("tool write\n{}", SharedToolRenderKind::Files),
        ("tool file_write\n{}", SharedToolRenderKind::Files),
        ("tool file_edit\n{}", SharedToolRenderKind::Files),
        ("tool notebook_edit\n{}", SharedToolRenderKind::Files),
        ("tool glob\n{}", SharedToolRenderKind::Files),
        ("tool grep\n{}", SharedToolRenderKind::Files),
        ("tool content_search\n{}", SharedToolRenderKind::Files),
        ("tool codesearch\n{}", SharedToolRenderKind::Files),
        ("tool lsp\n{}", SharedToolRenderKind::Lsp),
        ("tool git_operations\n{}", SharedToolRenderKind::GitDiff),
        ("tool AskUserQuestion\n{}", SharedToolRenderKind::Question),
        ("tool fetch_webpage\n{}", SharedToolRenderKind::Web),
        ("tool http_request\n{}", SharedToolRenderKind::Web),
        ("tool web_search\n{}", SharedToolRenderKind::Web),
        ("tool AgentTool\n{}", SharedToolRenderKind::Advanced),
        ("tool Agent\n{}", SharedToolRenderKind::Advanced),
        ("tool browser\n{}", SharedToolRenderKind::Advanced),
        ("tool open_browser_page\n{}", SharedToolRenderKind::Advanced),
        ("tool enter_worktree\n{}", SharedToolRenderKind::Advanced),
        ("tool exit_worktree\n{}", SharedToolRenderKind::Advanced),
        ("tool task_complete\n{}", SharedToolRenderKind::Advanced),
        ("tool mcp_linear\n{}", SharedToolRenderKind::Mcp),
        ("tool custom\n{}", SharedToolRenderKind::Text),
    ] {
        assert_eq!(shared_tool_render_kind(raw), Some(expected), "{raw}");
    }
}

#[test]
fn shared_tool_render_kind_detects_mcp_from_generic_tool_payload() {
    let raw = r#"tool tools
{"output":"[{\"server\":{\"name\":\"rustcodegraph\"},\"resources\":[]}]"}"#;

    assert_eq!(shared_tool_render_kind(raw), Some(SharedToolRenderKind::Mcp));
}

#[test]
fn shared_tool_render_kind_uses_render_hint_canonical_name() {
    assert_eq!(
        shared_tool_render_kind(
            r#"tool unknown
{"renderHint":{"metadata":{"canonical_tool_id":"git_diff"}}}"#
        ),
        Some(SharedToolRenderKind::GitDiff)
    );
}

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
