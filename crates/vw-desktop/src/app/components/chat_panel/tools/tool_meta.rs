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
    chat_secondary_text_color, icon_svg, is_dark_theme, normalize_file_reference_to_path,
    truncate_chars,
};
/// 重新导出 use iced::widget::{container, row, svg, text}，让上层模块通过稳定路径访问。
use iced::widget::text::Wrapping;
use iced::widget::{Space, container, row, svg, text};
/// 重新导出 use iced::{Alignment, Color, Element, Length, Theme}，让上层模块通过稳定路径访问。
use iced::{Alignment, Color, Element, Length, Theme};

/// 重新导出 use super::canonical_tool_name，让上层模块通过稳定路径访问。
use super::canonical_tool_name;

/// 工具生命周期在 UI 中使用的稳定语义。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolVisualState {
    Waiting,
    Running,
    Completed,
    Failed,
    Cancelled,
    Unknown,
}

/// 工具生命周期颜色的语义角色。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolVisualTone {
    Neutral,
    Waiting,
    Accent,
    Success,
    Danger,
    Muted,
}

/// 工具生命周期的完整 UI 展示结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToolVisual {
    pub state: ToolVisualState,
    pub icon: Icon,
    pub label: &'static str,
    pub is_animated: bool,
    pub tone: ToolVisualTone,
}

impl ToolVisual {
    /// 根据当前主题解析生命周期色调。
    pub fn color(self, theme: &Theme) -> Color {
        match self.tone {
            ToolVisualTone::Neutral => chat_secondary_text_color(theme),
            ToolVisualTone::Waiting => {
                if is_dark_theme(theme) {
                    Color::from_rgb8(0xD6, 0xA8, 0x43)
                } else {
                    Color::from_rgb8(0x97, 0x68, 0x14)
                }
            }
            ToolVisualTone::Accent => theme.extended_palette().primary.base.color,
            ToolVisualTone::Success => {
                let alpha = if is_dark_theme(theme) { 0.78 } else { 0.82 };
                theme.extended_palette().success.base.color.scale_alpha(alpha)
            }
            ToolVisualTone::Danger => theme.extended_palette().danger.base.color,
            ToolVisualTone::Muted => chat_secondary_subtle_text_color(theme),
        }
    }
}

/// 将协议中出现的生命周期字符串映射为纯 UI 展示语义。
pub fn tool_visual_state(raw_state: &str) -> ToolVisual {
    match raw_state.trim().to_ascii_lowercase().as_str() {
        "waiting" | "queued" => ToolVisual {
            state: ToolVisualState::Waiting,
            icon: Icon::Clock,
            label: "等待中",
            is_animated: true,
            tone: ToolVisualTone::Waiting,
        },
        "running" => ToolVisual {
            state: ToolVisualState::Running,
            icon: Icon::ArrowRepeat,
            label: "运行中",
            is_animated: true,
            tone: ToolVisualTone::Accent,
        },
        "completed" | "success" => ToolVisual {
            state: ToolVisualState::Completed,
            icon: Icon::Check,
            label: "已完成",
            is_animated: false,
            tone: ToolVisualTone::Success,
        },
        "error" | "failed" | "denied" => ToolVisual {
            state: ToolVisualState::Failed,
            icon: Icon::X,
            label: "失败",
            is_animated: false,
            tone: ToolVisualTone::Danger,
        },
        "cancelled" => ToolVisual {
            state: ToolVisualState::Cancelled,
            icon: Icon::X,
            label: "已取消",
            is_animated: false,
            tone: ToolVisualTone::Muted,
        },
        _ => ToolVisual {
            state: ToolVisualState::Unknown,
            icon: Icon::Clock,
            label: "状态未知",
            is_animated: false,
            tone: ToolVisualTone::Neutral,
        },
    }
}

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

fn tool_icon_badge_color(
    theme: &Theme,
    is_compact_file_tool: bool,
    visual: ToolVisual,
) -> Color {
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
pub fn tool_verb(tool_name: &str) -> &'static str {
    let tool_name = canonical_tool_name(tool_name).to_ascii_lowercase();
    match tool_name.as_str() {
        "config" => "配置",
        "read" | "file_read" | "pdf_read" | "read_file" => "读取",
        "write" | "file_write" => "写入",
        "file_edit" | "notebook_edit" | "apply_patch" => "编辑",
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
        | "vscode_listcodeusages" => "搜索",
        "bash" | "shell" => "运行",
        "ls" | "list_dir" => "列出",
        "webfetch" | "web_fetch" | "http_request" | "fetch_webpage" => "请求",
        "browser" | "browser_open" | "open_browser_page" => "浏览",
        "skill" => "技能",
        "task" => "任务",
        "question" => "提问",
        "screenshot" => "截图",
        "agenttool" | "agent" => "AgentTool",
        "mcp" | "list_mcp_resources" | "list_mcp_resource_templates" | "read_mcp_resource" => "MCP",
        "todowrite" => "写任务",
        "todoread" => "读任务",
        "schedule" | "cron_add" | "cron_list" | "cron_run" | "cron_update" | "cron_remove" => {
            "调度"
        }
        "memory_store" => "存储记忆",
        "memory_recall" => "回忆记忆",
        "memory_forget" => "遗忘记忆",
        "git_operations" | "git_diff" | "get_changed_files" => "Git Diff",
        "view_image" => "图片信息",
        "get_errors" => "诊断",
        "process" => "进程",
        "workflow_node" => "工作流",
        name if name.starts_with("mcp_") => "MCP",
        _ => "工具",
    }
}

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
pub fn tool_icon_badge<'a>(
    tool_name: &str,
    visual: ToolVisual,
) -> Element<'a, Message> {
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
    let title = title.into();
    row![
        tool_icon_badge(tool_name, visual),
        text(title).size(13).font(bold_font()).style(move |theme: &Theme| {
            iced::widget::text::Style {
                color: Some(if visual.state == ToolVisualState::Failed {
                    theme.extended_palette().danger.base.color
                } else {
                    chat_secondary_text_color(theme)
                }),
            }
        })
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
    let summary = summary.into();
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
pub fn tool_inline_summary(tool_name: &str, input: &str) -> Option<String> {
    tool_inline_summary_with_max_chars(tool_name, input, 80)
}

/// 返回不预先按字符数截短的工具摘要，供按真实布局宽度裁切的状态栏使用。
pub fn tool_inline_summary_unbounded(tool_name: &str, input: &str) -> Option<String> {
    tool_inline_summary_with_max_chars(tool_name, input, usize::MAX)
}

fn tool_inline_summary_with_max_chars(
    tool_name: &str,
    input: &str,
    max_chars: usize,
) -> Option<String> {
    let tool_name = canonical_tool_name(tool_name).to_ascii_lowercase();
    if !input.trim_start().starts_with('{') {
        let t = input.trim();
        if t.is_empty() { None } else { Some(t.to_string()) }
    } else {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(input.trim()) else { return None };
        match tool_name.as_str() {
            "config" => {
                let setting = v.get("setting").and_then(|x| x.as_str()).unwrap_or("").trim();
                let section = v.get("section").and_then(|x| x.as_str()).unwrap_or("").trim();
                if !setting.is_empty() {
                    if let Some(value) = v.get("value") {
                        let display = match value {
                            // serde_json 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                            serde_json::Value::String(text) => text.to_string(),
                            other => {
                                serde_json::to_string(other).unwrap_or_else(|_| other.to_string())
                            }
                        };
                        Some(
                            truncate_chars(&format!("{} = {}", setting, display), max_chars)
                                .to_string(),
                        )
                    } else {
                        Some(truncate_chars(setting, max_chars).to_string())
                    }
                } else if !section.is_empty() && section != "all" {
                    Some(truncate_chars(section, max_chars).to_string())
                } else {
                    None
                }
            }
            "read" | "file_read" | "pdf_read" | "read_file" => {
                let path = v
                    .get("filePath")
                    .or_else(|| v.get("file_path"))
                    .or_else(|| v.get("path"))
                    .and_then(|x| x.as_str())
                    .and_then(normalize_file_reference_to_path)
                    .unwrap_or_default();
                let mut parts = Vec::new();
                if let Some(offset) = v.get("offset").and_then(|x| x.as_u64()) {
                    parts.push(format!("offset={}", offset.max(1)));
                }
                if let Some(limit) = v.get("limit").and_then(|x| x.as_u64()) {
                    parts.push(format!("limit={limit}"));
                }
                let summary = if path.is_empty() {
                    if parts.is_empty() { None } else { Some(parts.join(", ")) }
                } else if parts.is_empty() {
                    Some(path)
                } else {
                    Some(format!("{path} ({})", parts.join(", ")))
                };
                summary.map(|s| truncate_chars(&s, max_chars).to_string())
            }
            "write" | "file_write" | "file_edit" | "notebook_edit" => {
                let path = v
                    .get("filePath")
                    .or_else(|| v.get("file_path"))
                    .or_else(|| v.get("path"))
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .trim();
                if path.is_empty() {
                    None
                } else {
                    Some(truncate_chars(path, max_chars).to_string())
                }
            }
            "apply_patch" => {
                let path = v.get("path").and_then(|x| x.as_str()).unwrap_or("").trim();
                if path.is_empty() {
                    None
                } else {
                    Some(truncate_chars(path, max_chars).to_string())
                }
            }
            "grep"
            | "glob"
            | "lsp"
            | "glob_search"
            | "codesearch"
            | "content_search"
            | "searchcodebase"
            | "grep_search"
            | "file_search"
            | "semantic_search"
            | "web_search"
            | "github_repo"
            | "copilot_getnotebooksummary"
            | "vscode_listcodeusages" => {
                let pattern = v
                    .get("pattern")
                    .or_else(|| v.get("query"))
                    .or_else(|| v.get("information_request"))
                    .or_else(|| v.get("symbol"))
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .trim();
                let path = v
                    .get("path")
                    .or_else(|| v.get("includePattern"))
                    .or_else(|| v.get("filePath"))
                    .and_then(|x| x.as_str())
                    .and_then(normalize_file_reference_to_path)
                    .unwrap_or_default();
                match (pattern.is_empty(), path.is_empty()) {
                    (true, true) => None,
                    (false, true) => Some(truncate_chars(pattern, max_chars).to_string()),
                    (true, false) => Some(truncate_chars(&path, max_chars).to_string()),
                    (false, false) => {
                        Some(
                            truncate_chars(&format!("{} in {}", pattern, path), max_chars)
                                .to_string(),
                        )
                    }
                }
            }
            "ls" | "list_dir" => {
                let path = v
                    .get("path")
                    .or_else(|| v.get("filePath"))
                    .or_else(|| v.get("file_path"))
                    .and_then(|x| x.as_str())
                    .and_then(normalize_file_reference_to_path)
                    .unwrap_or_default();
                if path.is_empty() {
                    None
                } else {
                    Some(truncate_chars(&path, max_chars).to_string())
                }
            }
            "bash" | "shell" => {
                let cmd = v
                    .get("command")
                    .or_else(|| v.get("cmd"))
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .trim();
                if cmd.is_empty() {
                    None
                } else {
                    Some(truncate_chars(cmd, max_chars).to_string())
                }
            }
            "webfetch" | "web_fetch" | "fetch_webpage" => {
                let url = v
                    .get("urls")
                    .and_then(|x| x.as_array())
                    .and_then(|x| x.first())
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .trim();
                let query = v.get("query").and_then(|x| x.as_str()).unwrap_or("").trim();
                match (url.is_empty(), query.is_empty()) {
                    (true, true) => None,
                    (false, true) => Some(truncate_chars(url, max_chars).to_string()),
                    (true, false) => Some(truncate_chars(query, max_chars).to_string()),
                    (false, false) => {
                        Some(
                            truncate_chars(&format!("{} in {}", query, url), max_chars).to_string(),
                        )
                    }
                }
            }
            "question" => {
                let questions =
                    v.get("questions").and_then(|x| x.as_array()).cloned().unwrap_or_default();
                let count = questions.len();
                let first = questions.first();
                let label = first
                    .and_then(|q| q.get("header").and_then(|x| x.as_str()))
                    .filter(|s| !s.trim().is_empty())
                    .or_else(|| first.and_then(|q| q.get("question").and_then(|x| x.as_str())))
                    .unwrap_or("")
                    .trim();

                match (label.is_empty(), count) {
                    (true, 0) => None,
                    (true, 1) => Some("1 个问题".to_string()),
                    (true, count) => Some(format!("{} 个问题", count)),
                    (false, 1) => Some(truncate_chars(label, max_chars).to_string()),
                    (false, count) => Some(
                        truncate_chars(&format!("{} 等 {} 个问题", label, count), max_chars)
                            .to_string(),
                    ),
                }
            }
            "get_errors" => v
                .get("filePaths")
                .and_then(|x| x.as_array())
                .and_then(|x| x.first())
                .and_then(|x| x.as_str())
                .and_then(normalize_file_reference_to_path)
                .or_else(|| Some("当前诊断".to_string())),
            "get_changed_files" => {
                let path = v
                    .get("repositoryPath")
                    .and_then(|x| x.as_str())
                    .and_then(normalize_file_reference_to_path)
                    .unwrap_or_default();
                if path.is_empty() { Some("当前仓库".to_string()) } else { Some(path) }
            }
            "view_image" => {
                let path = v
                    .get("path")
                    .or_else(|| v.get("filePath"))
                    .or_else(|| v.get("file_path"))
                    .and_then(|x| x.as_str())
                    .and_then(normalize_file_reference_to_path)
                    .unwrap_or_default();
                if path.is_empty() {
                    None
                } else {
                    Some(truncate_chars(&path, max_chars).to_string())
                }
            }
            "browser_open" | "open_browser_page" => {
                let url = v.get("url").and_then(|x| x.as_str()).unwrap_or("").trim();
                if url.is_empty() {
                    None
                } else {
                    Some(truncate_chars(url, max_chars).to_string())
                }
            }
            "agenttool" | "agent" => {
                let agent = v
                    .get("agent")
                    .or_else(|| v.get("subagent_type"))
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .trim();
                let prompt = v
                    .get("prompt")
                    .or_else(|| v.get("task"))
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .trim();
                match (agent.is_empty(), prompt.is_empty()) {
                    (true, true) => None,
                    (false, true) => Some(format!("调用 {}", agent)),
                    (true, false) => Some(truncate_chars(prompt, max_chars).to_string()),
                    (false, false) => Some(
                        truncate_chars(
                            &format!(
                                "{} · {}",
                                agent,
                                truncate_chars(prompt, max_chars.saturating_sub(24))
                            ),
                            max_chars,
                        )
                        .to_string(),
                    ),
                }
            }
            "browser" => {
                let action = v.get("action").and_then(|x| x.as_str()).unwrap_or("browser").trim();
                let url = v.get("url").and_then(|x| x.as_str()).unwrap_or("").trim();
                let selector = v.get("selector").and_then(|x| x.as_str()).unwrap_or("").trim();
                if !url.is_empty() {
                    Some(
                        truncate_chars(&format!("{} · {}", action, url), max_chars).to_string(),
                    )
                } else if !selector.is_empty() {
                    Some(
                        truncate_chars(&format!("{} · {}", action, selector), max_chars).to_string(),
                    )
                } else if action.is_empty() {
                    None
                } else {
                    Some(truncate_chars(action, max_chars).to_string())
                }
            }
            "git_operations" | "git_diff" => {
                let operation = v.get("operation").and_then(|x| x.as_str()).unwrap_or("").trim();
                if tool_name == "git_operations" && operation.is_empty() {
                    return None;
                }
                if tool_name == "git_diff" || operation == "diff" {
                    let files = v.get("files").and_then(|x| x.as_str()).unwrap_or(".").trim();
                    let cached = v.get("cached").and_then(|x| x.as_bool()).unwrap_or(false);
                    let mut summary = if files.is_empty() || files == "." {
                        "git diff".to_string()
                    } else {
                        format!(
                            "git diff {}",
                            truncate_chars(files, max_chars.saturating_sub(16))
                        )
                    };
                    if cached {
                        summary.push_str(" --cached");
                    }
                    Some(summary)
                } else {
                    Some(format!("git {}", operation))
                }
            }
            _ => None,
        }
    }
}

/// tests 子模块承载当前组件的一部分独立职责。
#[cfg(test)]
#[path = "tool_meta_tests.rs"]
mod tool_meta_tests;
