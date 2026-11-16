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

/// Dispatch and fallback order are shared by all hosts.
pub fn render_shared_tool_view<T>(
    raw: &str,
    mut render: impl FnMut(SharedToolRenderKind) -> Option<T>,
) -> Option<T> {
    let kind = shared_tool_render_kind(raw)?;
    render(kind)
        .or_else(|| {
            (kind == SharedToolRenderKind::TodoCompact)
                .then(|| render(SharedToolRenderKind::TodoRead))
                .flatten()
        })
        .or_else(|| {
            (kind != SharedToolRenderKind::Text)
                .then(|| render(SharedToolRenderKind::Text))
                .flatten()
        })
}

/// Complete local input for the shared tool renderer. Git and stored todos remain injected boundaries.
pub struct ViewInput<'a> {
    pub card: crate::card_input::CardInput<'a>,
    pub result: super::result_input::ResultInput<'a>,
    pub file_filter: &'a str,
    pub file_hovered: Option<&'a str>,
}
#[allow(clippy::too_many_arguments)]
pub fn view<'a, M: Clone + 'static>(
    input: &ViewInput<'a>,
    m: usize,
    t: usize,
    raw: &str,
    diff: &impl Fn(String, Option<String>, String, String) -> iced::Element<'a, M>,
    todos: impl Fn() -> Option<(Vec<todo_types::Todo>, Option<String>)>,
    card: fn(crate::card_input::Message<M>) -> M,
    plain: fn(crate::card_input::Message) -> M,
    advanced: fn(crate::advanced_input::Message) -> M,
) -> Option<iced::Element<'a, M>> {
    render_shared_tool_view(raw, |kind| match kind {
        SharedToolRenderKind::Bash => {
            super::bash_view::tool_bash_view(&input.card, m, t, raw).map(|v| v.map(plain))
        }
        SharedToolRenderKind::Config => {
            super::config_view::tool_config_view(&input.card, m, t, raw).map(|v| v.map(plain))
        }
        SharedToolRenderKind::Read => {
            super::read_view::tool_read_view(&input.card, m, t, raw).map(|v| v.map(plain))
        }
        SharedToolRenderKind::Skill => {
            super::skill_view::tool_skill_view(&input.card, m, t, raw).map(|v| v.map(plain))
        }
        SharedToolRenderKind::Text => {
            super::text_view::tool_text_view(&input.card, m, t, raw).map(|v| v.map(plain))
        }
        SharedToolRenderKind::Web => {
            super::web_view::tool_web_view(&input.card, m, t, raw).map(|v| v.map(plain))
        }
        SharedToolRenderKind::Workflow => {
            super::workflow_view::tool_workflow_view(&input.card, m, t, raw).map(|v| v.map(plain))
        }
        SharedToolRenderKind::Lsp => {
            super::lsp_view::tool_lsp_view(&input.result, m, t, raw).map(|v| v.map(plain))
        }
        SharedToolRenderKind::Mcp => {
            super::mcp_view::tool_mcp_view(&input.result, m, t, raw).map(|v| v.map(plain))
        }
        SharedToolRenderKind::Question => {
            super::question_view::tool_question_view(&input.result, m, t, raw).map(|v| v.map(plain))
        }
        SharedToolRenderKind::TodoCompact => {
            super::todo_views::tool_todowrite_compact_view(&input.card, m, t, raw)
                .map(|v| v.map(plain))
        }
        SharedToolRenderKind::TodoRead => {
            super::todo_views::tool_todos_view(&input.card, m, t, raw, &todos).map(|v| v.map(plain))
        }
        SharedToolRenderKind::ApplyPatch => {
            super::apply_patch_view::tool_apply_patch_view(&input.card, m, t, raw, diff)
                .map(|v| v.map(card))
        }
        SharedToolRenderKind::GitDiff => {
            super::git_diff_view::tool_git_diff_view(&input.result, m, t, raw, diff)
                .map(|v| v.map(card))
        }
        SharedToolRenderKind::Files => super::files_view::tool_files_view(
            &super::files_input::FilesInput {
                card: input.card.clone(),
                filter: input.file_filter,
                file_hovered: input.file_hovered,
            },
            m,
            t,
            raw,
            diff,
        )
        .map(|v| v.map(card)),
        SharedToolRenderKind::Advanced => super::advanced_view::tool_advanced_view(
            crate::advanced_input::AdvancedInput {
                i18n: input.card.i18n,
                hovered: input.card.hovered,
                text: input.card.text.clone(),
            },
            m,
            t,
            raw,
        )
        .map(|v| v.map(advanced)),
    })
}
