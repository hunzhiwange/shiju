//! 工具元数据映射。
//!
//! 本模块集中维护工具名称到图标、动词、标题和摘要的映射，保证聊天工具卡片文案一致。

/// 重新导出 use crate::app::{Message}，让上层模块通过稳定路径访问。
use crate::app::Message;
use crate::app::assets::Icon;
/// 重新导出 use crate::app::components::chat_panel::utils::{，让上层模块通过稳定路径访问。
use crate::app::components::chat_panel::utils::{
    bold_font, chat_secondary_subtle_text_color, chat_secondary_text_color, icon_svg,
    normalize_file_reference_to_path, truncate_chars,
};
/// 重新导出 use iced::widget::{container, row, svg, text}，让上层模块通过稳定路径访问。
use iced::widget::{container, row, svg, text};
/// 重新导出 use iced::{Alignment, Color, Element, Length, Theme}，让上层模块通过稳定路径访问。
use iced::{Alignment, Color, Element, Length, Theme};

/// 重新导出 use super::canonical_tool_name，让上层模块通过稳定路径访问。
use super::canonical_tool_name;

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
pub fn tool_emoji(tool_name: &str) -> &'static str {
    let tool_name = canonical_tool_name(tool_name).to_ascii_lowercase();
    match tool_name.as_str() {
        "config" => "⚙️",
        "brief" => "💬",
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
        | "tool_search"
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
        "image_info" | "view_image" => "🖼️",
        "get_errors" => "🧪",
        "get_changed_files" => "🔀",
        "wasm_module" => "⚙️",
        "composio" => "🔗",
        "pushover" => "🔔",
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
        "brief" => Icon::ChatTextFill,
        "read" | "file_read" | "pdf_read" | "read_file" => Icon::ChevronRight,
        "write" | "file_write" | "file_edit" | "notebook_edit" => Icon::ChevronRight,
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
        "bash" | "shell" => Icon::Terminal,
        "todowrite" | "todoread" => Icon::Clipboard,
        "ls" | "list_dir" => Icon::FolderOpen,
        "webfetch" | "web_fetch" | "http_request" | "fetch_webpage" => Icon::Link,
        "browser" | "browser_open" | "open_browser_page" => Icon::Link,
        "skill" => Icon::Star,
        "task" | "question" => Icon::Check,
        "agenttool"
        | "agent"
        | "mcp"
        | "list_mcp_resources"
        | "list_mcp_resource_templates"
        | "read_mcp_resource" => Icon::Box,
        "screenshot" => Icon::Image,
        "get_errors" => Icon::Code,
        "schedule" | "cron_add" | "cron_list" | "cron_run" | "cron_update" | "cron_remove" => {
            // Icon 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            Icon::Clock
        }
        "memory_store" | "memory_recall" | "memory_forget" => Icon::Box,
        "git_operations" | "git_diff" | "get_changed_files" => Icon::GitBranch,
        "image_info" | "view_image" => Icon::Image,
        "workflow_node" => Icon::Grid1x2,
        name if name.starts_with("mcp_") => Icon::Box,
        _ => Icon::FileText,
    }
}

fn tool_icon_badge_size(tool_name: &str) -> f32 {
    if is_compact_file_tool_icon(tool_name) {
        return 8.0;
    }

    14.0
}

fn is_compact_file_tool_icon(tool_name: &str) -> bool {
    let tool_name = canonical_tool_name(tool_name).to_ascii_lowercase();
    matches!(
        tool_name.as_str(),
        "read"
            | "file_read"
            | "pdf_read"
            | "read_file"
            | "write"
            | "file_write"
            | "file_edit"
            | "notebook_edit"
    )
}

fn tool_icon_badge_color(theme: &Theme, is_compact_file_tool: bool, is_error: bool) -> Color {
    if is_error {
        return theme.extended_palette().danger.base.color;
    }

    if is_compact_file_tool {
        return chat_secondary_subtle_text_color(theme);
    }

    chat_secondary_text_color(theme)
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
        "brief" => "消息",
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
        "tool_search" => "tool_search",
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
        "image_info" | "view_image" => "图片信息",
        "get_errors" => "诊断",
        "wasm_module" => "WASM",
        "composio" => "Composio",
        "pushover" => "通知",
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
pub fn tool_icon_badge<'a>(tool_name: &str, is_error: bool) -> Element<'a, Message> {
    let is_compact_file_tool = is_compact_file_tool_icon(tool_name);
    let icon = icon_svg(tool_icon(tool_name))
        .width(Length::Fixed(tool_icon_badge_size(tool_name)))
        .height(Length::Fixed(tool_icon_badge_size(tool_name)))
        .style(move |theme: &Theme, _status| svg::Style {
            color: Some(tool_icon_badge_color(theme, is_compact_file_tool, is_error)),
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
    // is_error 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    is_error: bool,
) -> Element<'a, Message> {
    let title = title.into();
    row![
        tool_icon_badge(tool_name, is_error),
        text(title).size(13).font(bold_font()).style(move |theme: &Theme| {
            // iced 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            iced::widget::text::Style {
                // color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                color: Some(if is_error {
                    theme.extended_palette().danger.base.color
                } else {
                    chat_secondary_text_color(theme)
                }),
            }
        })
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .into()
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
                        Some(truncate_chars(&format!("{} = {}", setting, display), 80).to_string())
                    } else {
                        Some(truncate_chars(setting, 80).to_string())
                    }
                } else if !section.is_empty() && section != "all" {
                    Some(truncate_chars(section, 80).to_string())
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
                summary.map(|s| truncate_chars(&s, 80).to_string())
            }
            "write" | "file_write" | "file_edit" | "notebook_edit" => {
                let path = v
                    .get("filePath")
                    .or_else(|| v.get("file_path"))
                    .or_else(|| v.get("path"))
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .trim();
                if path.is_empty() { None } else { Some(truncate_chars(path, 80).to_string()) }
            }
            "apply_patch" => {
                let path = v.get("path").and_then(|x| x.as_str()).unwrap_or("").trim();
                if path.is_empty() { None } else { Some(truncate_chars(path, 80).to_string()) }
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
                    (false, true) => Some(truncate_chars(pattern, 80).to_string()),
                    (true, false) => Some(truncate_chars(&path, 80).to_string()),
                    (false, false) => {
                        Some(truncate_chars(&format!("{} in {}", pattern, path), 80).to_string())
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
                if path.is_empty() { None } else { Some(truncate_chars(&path, 80).to_string()) }
            }
            "bash" | "shell" => {
                let cmd = v
                    .get("command")
                    .or_else(|| v.get("cmd"))
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .trim();
                if cmd.is_empty() { None } else { Some(truncate_chars(cmd, 80).to_string()) }
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
                    (false, true) => Some(truncate_chars(url, 80).to_string()),
                    (true, false) => Some(truncate_chars(query, 80).to_string()),
                    (false, false) => {
                        Some(truncate_chars(&format!("{} in {}", query, url), 80).to_string())
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
                    (false, 1) => Some(truncate_chars(label, 80).to_string()),
                    (false, count) => Some(
                        truncate_chars(&format!("{} 等 {} 个问题", label, count), 80).to_string(),
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
            "image_info" | "view_image" => {
                let path = v
                    .get("path")
                    .or_else(|| v.get("filePath"))
                    .or_else(|| v.get("file_path"))
                    .and_then(|x| x.as_str())
                    .and_then(normalize_file_reference_to_path)
                    .unwrap_or_default();
                if path.is_empty() { None } else { Some(truncate_chars(&path, 80).to_string()) }
            }
            "browser_open" | "open_browser_page" => {
                let url = v.get("url").and_then(|x| x.as_str()).unwrap_or("").trim();
                if url.is_empty() { None } else { Some(truncate_chars(url, 80).to_string()) }
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
                    (true, false) => Some(truncate_chars(prompt, 80).to_string()),
                    (false, false) => Some(
                        truncate_chars(&format!("{} · {}", agent, truncate_chars(prompt, 56)), 80)
                            .to_string(),
                    ),
                }
            }
            "browser" => {
                let action = v.get("action").and_then(|x| x.as_str()).unwrap_or("browser").trim();
                let url = v.get("url").and_then(|x| x.as_str()).unwrap_or("").trim();
                let selector = v.get("selector").and_then(|x| x.as_str()).unwrap_or("").trim();
                if !url.is_empty() {
                    Some(truncate_chars(&format!("{} · {}", action, url), 80).to_string())
                } else if !selector.is_empty() {
                    Some(truncate_chars(&format!("{} · {}", action, selector), 80).to_string())
                } else if action.is_empty() {
                    None
                } else {
                    Some(truncate_chars(action, 80).to_string())
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
                        format!("git diff {}", truncate_chars(files, 64))
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
