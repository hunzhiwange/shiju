//! 共享工具渲染入口。
//!
//! 本模块识别可复用的工具展示类型，并把工具调用分派到对应的紧凑视图。

use iced::Element;

/// 重新导出 use crate::app::{App, Message}，让上层模块通过稳定路径访问。
use crate::app::{App, Message};

/// 重新导出 use super::{，让上层模块通过稳定路径访问。
use super::{
    tool_advanced_view, tool_apply_patch_view, tool_bash_view, tool_config_view,
    tool_files_view, tool_git_diff_view, tool_lsp_view, tool_mcp_view, tool_name_from_raw,
    tool_question_view, tool_read_view, tool_search_view, tool_skill_view,
    tool_text_view, tool_todos_view, tool_todowrite_compact_view, tool_web_view,
    tool_workflow_view,
};

/// SharedToolRenderKind 描述 tool_renderer 模块支持的离散状态。
///
/// 新增变体时需要同步检查显式分支，避免未知状态被静默吞掉。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SharedToolRenderKind {
    Bash,
    Config,
    TodoCompact,
    TodoRead,
    ApplyPatch,
    Read,
    Files,
    Lsp,
    Mcp,
    ToolSearch,
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
pub(crate) fn shared_tool_render_kind(raw: &str) -> Option<SharedToolRenderKind> {
    let tool_name = tool_name_from_raw(raw)?;
    if super::is_mcp_tool_raw(raw) {
        return Some(SharedToolRenderKind::Mcp);
    }

    Some(match tool_name.as_str() {
        "bash" | "shell" => SharedToolRenderKind::Bash,
        "image_info" => SharedToolRenderKind::Bash,
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
        "tool_search" => SharedToolRenderKind::ToolSearch,
        _ => SharedToolRenderKind::Text,
    })
}

/// 渲染 shared tool view 对应的 diff 行、工具卡片或控件内容。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 无返回值时，函数通过发布消息或更新局部状态完成交互。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
pub(crate) fn render_shared_tool_view<'a>(
    app: &'a App,
    // msg_idx 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    msg_idx: usize,
    // tool_idx 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    tool_idx: usize,
    // visible 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    visible: &str,
) -> Option<Element<'a, Message>> {
    match shared_tool_render_kind(visible)? {
        // SharedToolRenderKind 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        SharedToolRenderKind::Bash => tool_bash_view(app, msg_idx, tool_idx, visible)
            .or_else(|| tool_text_view(app, msg_idx, tool_idx, visible)),
        SharedToolRenderKind::Config => tool_config_view(app, msg_idx, tool_idx, visible)
            .or_else(|| tool_text_view(app, msg_idx, tool_idx, visible)),
        SharedToolRenderKind::TodoCompact => {
            tool_todowrite_compact_view(app, msg_idx, tool_idx, visible)
                .or_else(|| tool_todos_view(app, msg_idx, tool_idx, visible))
                .or_else(|| tool_text_view(app, msg_idx, tool_idx, visible))
        }
        SharedToolRenderKind::TodoRead => tool_todos_view(app, msg_idx, tool_idx, visible)
            .or_else(|| tool_text_view(app, msg_idx, tool_idx, visible)),
        // SharedToolRenderKind 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        SharedToolRenderKind::ApplyPatch => tool_apply_patch_view(app, msg_idx, tool_idx, visible)
            .or_else(|| tool_text_view(app, msg_idx, tool_idx, visible)),
        SharedToolRenderKind::Read => tool_read_view(app, msg_idx, tool_idx, visible)
            .or_else(|| tool_text_view(app, msg_idx, tool_idx, visible)),
        SharedToolRenderKind::Files => tool_files_view(app, msg_idx, tool_idx, visible)
            .or_else(|| tool_text_view(app, msg_idx, tool_idx, visible)),
        SharedToolRenderKind::Lsp => tool_lsp_view(app, msg_idx, tool_idx, visible)
            .or_else(|| tool_text_view(app, msg_idx, tool_idx, visible)),
        SharedToolRenderKind::Mcp => tool_mcp_view(app, msg_idx, tool_idx, visible)
            .or_else(|| tool_text_view(app, msg_idx, tool_idx, visible)),
        SharedToolRenderKind::ToolSearch => tool_search_view(app, msg_idx, tool_idx, visible)
            .or_else(|| tool_text_view(app, msg_idx, tool_idx, visible)),
        // SharedToolRenderKind 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        SharedToolRenderKind::GitDiff => tool_git_diff_view(app, msg_idx, tool_idx, visible)
            .or_else(|| tool_text_view(app, msg_idx, tool_idx, visible)),
        SharedToolRenderKind::Question => tool_question_view(app, msg_idx, tool_idx, visible)
            .or_else(|| tool_text_view(app, msg_idx, tool_idx, visible)),
        // SharedToolRenderKind 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        SharedToolRenderKind::Web => tool_web_view(app, msg_idx, tool_idx, visible)
            .or_else(|| tool_text_view(app, msg_idx, tool_idx, visible)),
        SharedToolRenderKind::Advanced => tool_advanced_view(app, msg_idx, tool_idx, visible)
            .or_else(|| tool_text_view(app, msg_idx, tool_idx, visible)),
        SharedToolRenderKind::Skill => tool_skill_view(app, msg_idx, tool_idx, visible)
            .or_else(|| tool_text_view(app, msg_idx, tool_idx, visible)),
        SharedToolRenderKind::Workflow => tool_workflow_view(app, msg_idx, tool_idx, visible)
            .or_else(|| tool_text_view(app, msg_idx, tool_idx, visible)),
        SharedToolRenderKind::Text => tool_text_view(app, msg_idx, tool_idx, visible),
    }
}
