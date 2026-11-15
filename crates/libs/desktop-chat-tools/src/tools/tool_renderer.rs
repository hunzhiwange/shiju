//! 共享工具卡片分类。
use super::tool_parse::tool_name_from_raw;
/// SharedToolRenderKind 描述 tool_renderer 模块支持的离散状态。
///
/// 新增变体时需要同步检查显式分支，避免未知状态被静默吞掉。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SharedToolRenderKind {
    Bash,
    Config,
    TodoCompact,
    TodoRead,
    ApplyPatch,
    Read,
    Files,
    Lsp,
    Mcp,
    GitDiff,
    Question,
    Web,
    Advanced,
    Skill,
    Workflow,
    Text,
}

/// 处理 shared tool render kind 对应的局部职责。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// `None` 表示输入缺少必要字段、当前状态不匹配，或该视图片段不需要展示。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
pub fn shared_tool_render_kind(raw: &str) -> Option<SharedToolRenderKind> {
    let tool_name = tool_name_from_raw(raw)?;
    if super::mcp_view::is_mcp_tool_raw(raw) {
        return Some(SharedToolRenderKind::Mcp);
    }

    Some(match tool_name.as_str() {
        "bash" | "shell" => SharedToolRenderKind::Bash,
        "Config" | "config" => SharedToolRenderKind::Config,
        "todowrite" => SharedToolRenderKind::TodoCompact,
        "todoread" => SharedToolRenderKind::TodoRead,
        "apply_patch" => SharedToolRenderKind::ApplyPatch,
        "read" | "file_read" | "pdf_read" | "read_file" => SharedToolRenderKind::Read,
        "write" | "file_write" | "file_edit" | "notebook_edit" | "glob" | "glob_search"
        | "grep" | "content_search" | "codesearch" => SharedToolRenderKind::Files,
        "lsp" => SharedToolRenderKind::Lsp,
        "git_diff" | "git_operations" => SharedToolRenderKind::GitDiff,
        "question" | "AskUserQuestion" => SharedToolRenderKind::Question,
        "web_fetch" | "fetch_webpage" | "http_request" | "web_search" => {
            // SharedToolRenderKind 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            SharedToolRenderKind::Web
        }
        "skill" => SharedToolRenderKind::Skill,
        "workflow_node" => SharedToolRenderKind::Workflow,
        "AgentTool" | "Agent" | "browser" | "browser_open" | "open_browser_page"
        | "enter_worktree" | "exit_worktree" | "task_complete" => SharedToolRenderKind::Advanced,
        _ => SharedToolRenderKind::Text,
    })
}
