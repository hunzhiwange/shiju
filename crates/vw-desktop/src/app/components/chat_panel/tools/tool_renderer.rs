//! 共享工具渲染入口。
//!
//! 本模块识别可复用的工具展示类型，并把工具调用分派到对应的紧凑视图。

use iced::Element;

/// 重新导出 use crate::app::{App, Message}，让上层模块通过稳定路径访问。
use crate::app::{App, Message};

/// 重新导出 use super::{，让上层模块通过稳定路径访问。
use super::{
    tool_advanced_view, tool_apply_patch_view, tool_bash_view, tool_config_view, tool_files_view,
    tool_git_diff_view, tool_lsp_view, tool_mcp_view, tool_question_view, tool_read_view,
    tool_skill_view, tool_text_view, tool_todos_view, tool_todowrite_compact_view, tool_web_view,
    tool_workflow_view,
};

pub(crate) use desktop_chat_tools::tools::tool_renderer::{
    SharedToolRenderKind, shared_tool_render_kind,
};

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
    desktop_chat_tools::tools::tool_renderer::render_shared_tool_view(visible, |kind| match kind {
        SharedToolRenderKind::Bash => tool_bash_view(app, msg_idx, tool_idx, visible),
        SharedToolRenderKind::Config => tool_config_view(app, msg_idx, tool_idx, visible),
        SharedToolRenderKind::TodoCompact => {
            tool_todowrite_compact_view(app, msg_idx, tool_idx, visible)
        }
        SharedToolRenderKind::TodoRead => tool_todos_view(app, msg_idx, tool_idx, visible),
        SharedToolRenderKind::ApplyPatch => tool_apply_patch_view(app, msg_idx, tool_idx, visible),
        SharedToolRenderKind::Read => tool_read_view(app, msg_idx, tool_idx, visible),
        SharedToolRenderKind::Files => tool_files_view(app, msg_idx, tool_idx, visible),
        SharedToolRenderKind::Lsp => tool_lsp_view(app, msg_idx, tool_idx, visible),
        SharedToolRenderKind::Mcp => tool_mcp_view(app, msg_idx, tool_idx, visible),
        SharedToolRenderKind::GitDiff => tool_git_diff_view(app, msg_idx, tool_idx, visible),
        SharedToolRenderKind::Question => tool_question_view(app, msg_idx, tool_idx, visible),
        SharedToolRenderKind::Web => tool_web_view(app, msg_idx, tool_idx, visible),
        SharedToolRenderKind::Advanced => tool_advanced_view(app, msg_idx, tool_idx, visible),
        SharedToolRenderKind::Skill => tool_skill_view(app, msg_idx, tool_idx, visible),
        SharedToolRenderKind::Workflow => tool_workflow_view(app, msg_idx, tool_idx, visible),
        SharedToolRenderKind::Text => tool_text_view(app, msg_idx, tool_idx, visible),
    })
}
