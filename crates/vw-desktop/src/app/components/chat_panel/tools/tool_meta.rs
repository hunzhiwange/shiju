//! 工具元数据映射。
//!
//! 本模块集中维护工具名称到图标、动词、标题和摘要的映射，保证聊天工具卡片文案一致。

/// 重新导出 use crate::app::{Message}，让上层模块通过稳定路径访问。
use crate::app::Message;
use crate::app::assets::Icon;
use crate::app::components::chat_panel::tool_text_support::content_safe_font;
/// 重新导出 use crate::app::components::chat_panel::utils::{，让上层模块通过稳定路径访问。
use crate::app::components::chat_panel::utils::{
    bold_font, chat_secondary_muted_text_color, chat_secondary_subtle_text_color,
    chat_secondary_text_color, icon_svg, is_dark_theme,
};
/// 重新导出 use iced::widget::{container, row, svg, text}，让上层模块通过稳定路径访问。
use iced::widget::text::Wrapping;
use iced::widget::{Space, container, row, svg, text};
/// 重新导出 use iced::{Alignment, Color, Element, Length, Theme}，让上层模块通过稳定路径访问。
use iced::{Alignment, Color, Element, Length, Theme};

/// 重新导出 use super::canonical_tool_name，让上层模块通过稳定路径访问。
use super::canonical_tool_name;

/// 工具生命周期在 UI 中使用的稳定语义。
pub use desktop_chat_view::tool_presentation::{
    ToolVisualState, ToolVisualTone, ToolVisual, tool_visual_state, tool_verb,
    tool_inline_summary, tool_inline_summary_unbounded,
};
pub(crate) use desktop_chat_view::tool_presentation::{localized_tool_inline_summary, localized_tool_inline_summary_unbounded};

/// 处理 tool emoji 对应的局部职责。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 返回值保持当前模块的领域语义，供相邻视图或测试继续使用。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
#[cfg(test)]
pub fn tool_emoji(tool_name: &str) -> &'static str {
    let tool_name = canonical_tool_name(tool_name).to_ascii_lowercase();
    match tool_name.as_str() {
        "config" => "⚙️",
        "read" | "file_read" | "pdf_read" | "read_file" => "📖",
        "write" | "file_write" | "file_edit" | "notebook_edit" => "👁️",
        "apply_patch" => "📝",
        "lsp"
        | "codesearch"
        | "grep"
        | "glob"
        | "glob_search"
        | "content_search"
        | "searchcodebase"
        | "grep_search"
        | "file_search"
        | "semantic_search"
        | "web_search"
        | "github_repo"
        | "copilot_getnotebooksummary"
        | "vscode_listcodeusages" => "🔍",
        "bash" | "shell" => "⚡",
        "ls" | "list_dir" => "📁",
        "webfetch" | "web_fetch" | "http_request" | "fetch_webpage" => "🌐",
        "browser" | "browser_open" | "open_browser_page" => "🖥️",
        "skill" => "🎯",
        "task" => "📋",
        "question" => "❓",
        "screenshot" => "📸",
        "agenttool" | "agent" => "🛰️",
        "todowrite" | "todoread" => "✅",
        "schedule" | "cron_add" | "cron_list" | "cron_run" | "cron_update" | "cron_remove" => "⏰",
        "memory_store" | "memory_recall" | "memory_forget" => "🧠",
        "git_operations" | "git_diff" => "🔀",
        "view_image" => "🖼️",
        "get_errors" => "🧪",
        "get_changed_files" => "🔀",
        "process" => "💻",
        "workflow_node" => "🔁",
        _ => "🔧",
    }
}

/// 处理 tool icon 对应的局部职责。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 返回值保持当前模块的领域语义，供相邻视图或测试继续使用。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
pub fn tool_icon(tool_name: &str) -> Icon {
    let tool_name = canonical_tool_name(tool_name).to_ascii_lowercase();
    match tool_name.as_str() {
        "config" => Icon::Sliders,
        "read" | "file_read" | "pdf_read" | "read_file" => Icon::FileText,
        "write" | "file_write" | "file_edit" | "notebook_edit" => Icon::Pencil,
        "apply_patch" => Icon::Pencil,
        "lsp"
        | "codesearch"
        | "grep"
        | "glob"
        | "glob_search"
        | "content_search"
        | "searchcodebase"
        | "grep_search"
        | "file_search"
        | "semantic_search"
        | "web_search"
        | "github_repo"
        | "copilot_getnotebooksummary"
        | "vscode_listcodeusages" => Icon::Search,
        "bash" | "shell" | "process" => Icon::Terminal,
        "todowrite" | "todoread" => Icon::Clipboard,
        "ls" | "list_dir" => Icon::FolderOpen,
        "webfetch" | "web_fetch" | "http_request" | "fetch_webpage" => Icon::Globe,
        "browser" | "browser_open" | "open_browser_page" => Icon::Link,
        "skill" => Icon::Star,
        "task" => Icon::Clipboard,
        "question" => Icon::QuestionCircle,
        "agenttool" | "agent" => Icon::Robot,
        "mcp" | "list_mcp_resources" | "list_mcp_resource_templates" | "read_mcp_resource" => {
            Icon::Box
        }
        "screenshot" => Icon::Image,
        "get_errors" => Icon::Code,
        "schedule" | "cron_add" | "cron_list" | "cron_run" | "cron_update" | "cron_remove" => {
            // Icon 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            Icon::Clock
        }
        "memory_store" | "memory_recall" | "memory_forget" => Icon::Box,
        "git_operations" | "git_diff" | "get_changed_files" => Icon::GitBranch,
        "view_image" => Icon::Image,
        "workflow_node" => Icon::Grid1x2,
        name if name.starts_with("mcp_") => Icon::Box,
        _ => Icon::Wrench,
    }
}

fn tool_icon_badge_size(_tool_name: &str) -> f32 {
    14.0
}

fn tool_icon_badge_color(theme: &Theme, is_compact_file_tool: bool, visual: ToolVisual) -> Color {
    if visual.state == ToolVisualState::Unknown && is_compact_file_tool {
        chat_secondary_subtle_text_color(theme)
    } else {
        visual.color(theme)
    }
}

/// 处理 tool verb 对应的局部职责。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 返回值保持当前模块的领域语义，供相邻视图或测试继续使用。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。


/// 处理 tool header label 对应的局部职责。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 返回字符串已经按界面展示或比较需求做过必要整理。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
pub fn tool_header_label(tool_name: &str) -> String {
    let verb = tool_verb(tool_name);
    verb.to_string()
}

/// 处理 tool icon badge 对应的局部职责。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 返回值是 Iced `Element`，调用方继续组合到当前界面树中。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
pub fn tool_icon_badge<'a>(tool_name: &str, visual: ToolVisual) -> Element<'a, Message> {
    let icon = icon_svg(tool_icon(tool_name))
        .width(Length::Fixed(tool_icon_badge_size(tool_name)))
        .height(Length::Fixed(tool_icon_badge_size(tool_name)))
        .style(move |theme: &Theme, _status| svg::Style {
            color: Some(tool_icon_badge_color(theme, false, visual)),
        });

    // 徽标只保留居中的图标，去掉背景、边框，保持与标题对齐的固定尺寸。
    // 星形图标视觉重心偏下（下方两尖更重），几何居中后看起来比文字略低，
    // 用底部 padding 把它向上抬一点，与文字光学居中对齐。
    let optical_bottom = if matches!(tool_icon(tool_name), Icon::Star) { 2.0 } else { 0.0 };
    container(icon)
        .width(Length::Fixed(20.0))
        .height(Length::Fixed(20.0))
        .padding(iced::Padding { top: 0.0, right: 0.0, bottom: optical_bottom, left: 0.0 })
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center)
        .into()
}

/// 处理 tool header title 对应的局部职责。
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
pub fn tool_header_title<'a>(
    tool_name: &str,
    // title 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    title: impl Into<String>,
    visual: ToolVisual,
) -> Element<'a, Message> {
    let title = title.into().replace(['\r', '\n'], " ");
    row![
        tool_icon_badge(tool_name, visual),
        text(title).size(13).font(bold_font()).wrapping(Wrapping::None).style(
            move |theme: &Theme| iced::widget::text::Style {
                color: Some(if visual.state == ToolVisualState::Failed {
                    theme.extended_palette().danger.base.color
                } else {
                    chat_secondary_text_color(theme)
                }),
            }
        )
    ]
    .spacing(4)
    .align_y(Alignment::Center)
    .into()
}

/// 构建固定 16px 的工具生命周期状态槽。
pub fn tool_status_badge<'a>(visual: ToolVisual) -> Element<'a, Message> {
    let icon = icon_svg(visual.icon)
        .width(Length::Fixed(11.0))
        .height(Length::Fixed(11.0))
        .style(move |theme: &Theme, _status| svg::Style { color: Some(visual.color(theme)) });

    container(icon)
        .width(Length::Fixed(16.0))
        .height(Length::Fixed(16.0))
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center)
        .into()
}

/// 构建基础工具统一活动头部：类型、动作、摘要、状态与动作入口。
pub fn tool_activity_header<'a>(
    tool_name: &str,
    title: impl Into<String>,
    summary: impl Into<String>,
    visual: ToolVisual,
    action: Element<'a, Message>,
) -> iced::widget::Row<'a, Message> {
    // 禁止自动折行仍会渲染文本里的显式换行。
    let summary = summary.into().replace(['\r', '\n'], " ");
    let summary_view: Element<'a, Message> = if summary.trim().is_empty() {
        Space::new().width(Length::Shrink).into()
    } else {
        let font = content_safe_font(&summary, "JetBrains Mono");
        text(summary)
            .size(13)
            .font(font)
            .width(Length::Shrink)
            .wrapping(Wrapping::None)
            .style(|theme: &Theme| iced::widget::text::Style {
                color: Some(chat_secondary_muted_text_color(theme)),
            })
            .into()
    };

    row![
        tool_header_title(tool_name, title, visual),
        summary_view,
        tool_status_badge(visual),
        action,
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .width(Length::Fill)
}

/// 生成 tool inline summary，用于工具卡片或状态行的简短说明。
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
#[cfg(test)]
#[path = "tool_meta_tests.rs"]
mod tool_meta_tests;
