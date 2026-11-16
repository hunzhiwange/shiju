//! 工作空间协议解析的宿主边界；提示策略归通道运行库。
use super::*;
pub use channel_runtime::prompt::*;

/// Resolves the current workspace protocol and renders it for a legacy external
/// entrypoint that does not use the session processor directly.
pub fn build_workspace_protocol_messages(
    workspace_dir: &Path,
    skills: &[skills::Skill],
    skill_tool_enabled: bool,
    context_window_tokens: usize,
) -> Vec<ChatMessage> {
    let system = crate::session::prompt::system_with_metadata(None, workspace_dir.to_str());
    build_entrypoint_protocol_messages(
        system.instruction_context(),
        skills,
        skill_tool_enabled,
        context_window_tokens,
    )
}
