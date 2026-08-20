//! LSP 工具结果视图。
//!
//! 本模块解析语言服务返回的位置、符号和调用层级数据，并渲染可跳转的结果列表。

use iced::widget::{Space, button, column, container, mouse_area, scrollable, text};
/// 重新导出 use iced::{Alignment, Background, Border, Element, Length, Theme}，让上层模块通过稳定路径访问。
use iced::{Background, Border, Element, Length, Theme};
/// 重新导出 use serde::Deserialize，让上层模块通过稳定路径访问。
use serde::Deserialize;

/// 重新导出 use crate::app::assets::Icon，让上层模块通过稳定路径访问。
use crate::app::assets::Icon;
/// 重新导出 use crate::app::components::chat_panel::utils::{，让上层模块通过稳定路径访问。
use crate::app::components::chat_panel::utils::{
    chat_scroll_direction, eye_icon_button_style, eye_icon_svg_style, icon_svg,
    simplified_block_style, simplified_code_block_style, truncate_chars, truncate_lines_middle,
    weak_file_button_style,
};
/// 重新导出 use crate::app::{App, Message, message}，让上层模块通过稳定路径访问。
use crate::app::{App, Message, message};

/// 重新导出 use super::tool_meta::tool_header_title，让上层模块通过稳定路径访问。
use super::tool_meta::{tool_activity_header, tool_visual_state};
/// 重新导出 use super::tool_parse::{，让上层模块通过稳定路径访问。
use super::tool_parse::{
    tool_error_text, tool_input, tool_output_text, tool_result_data, tool_status, tool_summary_text,
};

/// LspToolData 保存 lsp_view 模块需要跨函数传递的状态。
///
/// 字段保持贴近调用方的真实数据，避免在 UI 边界处隐藏额外转换。
#[derive(Debug, Clone, Deserialize)]
pub(super) struct LspToolData {
    #[serde(default)]
    success: bool,
    #[serde(default)]
    operation: String,
    #[serde(default)]
    file_path: Option<String>,
    #[serde(default)]
    query: Option<String>,
    #[serde(default)]
    result_count: usize,
    #[serde(default)]
    file_count: usize,
    // payload 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    payload: LspPayload,
    #[serde(default)]
    error: Option<String>,
}

/// LspPayload 描述 lsp_view 模块支持的离散状态。
///
/// 新增变体时需要同步检查显式分支，避免未知状态被静默吞掉。
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(super) enum LspPayload {
    Locations {
        items: Vec<LspLocationItem>,
    },
    Hover {
        // contents 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        contents: String,
        #[serde(default)]
        line: Option<u32>,
        #[serde(default)]
        character: Option<u32>,
    },
    DocumentSymbols {
        items: Vec<LspDocumentSymbolItem>,
    },
    CallHierarchyItems {
        items: Vec<LspCallHierarchyItem>,
    },
    CallHierarchyCalls {
        items: Vec<LspCallHierarchyItem>,
    },
    Message {
        message: String,
    },
}

/// LspLocationItem 保存 lsp_view 模块需要跨函数传递的状态。
///
/// 字段保持贴近调用方的真实数据，避免在 UI 边界处隐藏额外转换。
#[derive(Debug, Clone, Deserialize)]
pub(super) struct LspLocationItem {
    // path 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    path: String,
    // absolute_path 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    absolute_path: String,
    // line 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    line: u32,
    // character 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    character: u32,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    kind: Option<String>,
    #[serde(default)]
    detail: Option<String>,
    #[serde(default)]
    container_name: Option<String>,
}

/// LspDocumentSymbolItem 保存 lsp_view 模块需要跨函数传递的状态。
///
/// 字段保持贴近调用方的真实数据，避免在 UI 边界处隐藏额外转换。
#[derive(Debug, Clone, Deserialize)]
pub(super) struct LspDocumentSymbolItem {
    // name 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    name: String,
    // kind 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    kind: String,
    #[serde(default)]
    detail: Option<String>,
    // path 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    path: String,
    // absolute_path 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    absolute_path: String,
    // line 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    line: u32,
    // character 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    character: u32,
    // depth 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    depth: usize,
}

/// LspCallHierarchyItem 保存 lsp_view 模块需要跨函数传递的状态。
///
/// 字段保持贴近调用方的真实数据，避免在 UI 边界处隐藏额外转换。
#[derive(Debug, Clone, Deserialize)]
pub(super) struct LspCallHierarchyItem {
    // name 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    name: String,
    // kind 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    kind: String,
    // path 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    path: String,
    // absolute_path 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    absolute_path: String,
    // line 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    line: u32,
    // character 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    character: u32,
    #[serde(default)]
    call_sites: Vec<LspCallSite>,
}

/// LspCallSite 保存 lsp_view 模块需要跨函数传递的状态。
///
/// 字段保持贴近调用方的真实数据，避免在 UI 边界处隐藏额外转换。
#[derive(Debug, Clone, Deserialize)]
struct LspCallSite {
    // line 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    line: u32,
    // character 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    character: u32,
}

/// 处理 tool lsp view 对应的局部职责。
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
pub fn tool_lsp_view<'a>(
    app: &'a App,
    // msg_idx 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    msg_idx: usize,
    // tool_idx 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    tool_idx: usize,
    // visible 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    visible: &str,
) -> Option<Element<'a, Message>> {
    let (first, rest) = visible.split_once('\n')?;
    let tool_name = first.trim().strip_prefix("tool ")?.trim();
    if super::canonical_tool_name(tool_name) != "lsp" {
        return None;
    }

    // LSP 工具输出可能来自不同后端版本，解析失败时保留原始文本展示更稳妥。
    let value = serde_json::from_str::<serde_json::Value>(rest.trim()).ok()?;
    let status = tool_status(&value);
    let is_error = matches!(status, "error" | "failed" | "denied");
    let is_running = status == "running";
    let is_waiting = matches!(status, "waiting" | "queued");
    let visual = tool_visual_state(status);
    let data = tool_result_data(&value)
        .cloned()
        .and_then(|data| serde_json::from_value::<LspToolData>(data).ok());
    let output_text = tool_output_text(&value).unwrap_or_default();
    let error_text = tool_error_text(&value)
        .or_else(|| data.as_ref().and_then(|data| data.error.clone()))
        .unwrap_or_default();
    let summary = tool_summary_text(&value).unwrap_or_else(|| {
        data.as_ref().map(summary_from_data).filter(|summary| !summary.is_empty()).unwrap_or_else(
            || {
                // super 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                super::tool_inline_summary("lsp", tool_input(&value))
                    .unwrap_or_else(|| truncate_chars(output_text.trim(), 96).to_string())
            },
        )
    });

    let title = if is_running || is_waiting {
        "LSP 中".to_string()
    } else if is_error || data.as_ref().is_some_and(|data| !data.success) {
        "LSP 失败".to_string()
    } else if let Some(data) = data.as_ref() {
        action_title(&data.operation).to_string()
    } else {
        "LSP".to_string()
    };

    let detail_btn = button(
        icon_svg(Icon::ChevronRight)
            .width(Length::Fixed(10.0))
            .height(Length::Fixed(10.0))
            .style(eye_icon_svg_style),
    )
    .padding([2, 4])
    .style(|theme: &Theme, status| eye_icon_button_style(theme, status))
    .on_press(Message::Chat(message::ChatMessage::OpenToolDetail(
        msg_idx,
        tool_idx,
        visible.to_string(),
    )));
    let key = ((msg_idx as u64) << 32) | tool_idx as u64;
    let detail_slot: Element<'a, Message> = if app.chat_tool_hovered_idx == Some(key) {
        detail_btn.into()
    } else {
        Space::new().width(Length::Fixed(22.0)).into()
    };

    let meta_label = data.as_ref().map(meta_text).unwrap_or_default();

    let header_summary = if meta_label.is_empty() {
        summary.clone()
    } else if summary.is_empty() || summary == meta_label {
        meta_label
    } else {
        format!("{summary} · {meta_label}")
    };
    let head = container(tool_activity_header(
        "lsp",
        title,
        header_summary,
        visual,
        detail_slot,
    ))
    .width(Length::Fill);

    let body: Element<'a, Message> = if is_running || is_waiting {
        let pending = if is_waiting { "等待 LSP 响应..." } else { "正在等待 LSP 响应..." };
        container(text(pending).size(14))
            .padding([10, 12])
            .width(Length::Fill)
            .into()
    } else if is_error || data.as_ref().is_some_and(|data| !data.success) {
        build_error_body(&error_text)
    } else if let Some(data) = data.as_ref() {
        build_success_body(data)
    } else {
        build_text_body(output_text.trim())
    };

    Some(
        mouse_area(
            container(column![head, body].spacing(8))
                .padding([2, 6])
                .width(Length::Fill)
                .style(simplified_block_style),
        )
        .on_enter(Message::Chat(message::ChatMessage::ToolHover(msg_idx, tool_idx)))
        .on_exit(Message::Chat(message::ChatMessage::ToolHoverLeave))
        .into(),
    )
}

/// 生成 summary from data，用于工具卡片或状态行的简短说明。
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
pub(super) fn summary_from_data(data: &LspToolData) -> String {
    if data.result_count > 0 {
        return meta_text(data);
    }
    if let Some(query) = data.query.as_deref().filter(|query| !query.trim().is_empty()) {
        return query.to_string();
    }
    data.file_path.clone().unwrap_or_default()
}

/// 处理 meta text 对应的局部职责。
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
pub(super) fn meta_text(data: &LspToolData) -> String {
    match data.operation.as_str() {
        "hover" => {
            if data.result_count > 0 {
                "悬停信息可用".to_string()
            } else {
                "无悬停信息".to_string()
            }
        }
        "documentSymbol" => format!("{} 个文档符号", data.result_count),
        "workspaceSymbol" => {
            format!("{} 个工作区符号 / {} 个文件", data.result_count, data.file_count)
        }
        "prepareCallHierarchy" => format!("{} 个调用层级项", data.result_count),
        "incomingCalls" => format!("{} 个入向调用 / {} 个文件", data.result_count, data.file_count),
        "outgoingCalls" => format!("{} 个出向调用 / {} 个文件", data.result_count, data.file_count),
        _ => format!("{} 个结果 / {} 个文件", data.result_count, data.file_count),
    }
}

/// 处理 action title 对应的局部职责。
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
pub(super) fn action_title(operation: &str) -> &'static str {
    match operation {
        "goToDefinition" => "跳转定义",
        "findReferences" => "查找引用",
        "hover" => "悬停信息",
        "documentSymbol" => "文档符号",
        "workspaceSymbol" => "工作区符号",
        "goToImplementation" => "跳转实现",
        "prepareCallHierarchy" => "调用层级",
        "incomingCalls" => "入向调用",
        "outgoingCalls" => "出向调用",
        _ => "LSP",
    }
}

/// 构建 error body 对应的 Iced 界面片段或中间数据。
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
fn build_error_body<'a>(error_text: &str) -> Element<'a, Message> {
    container(text(truncate_chars(error_text.trim(), 400)).size(14).style(|theme: &Theme| {
        // iced 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        iced::widget::text::Style {
            // color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            color: Some(theme.extended_palette().danger.base.color),
        }
    }))
    .padding([10, 12])
    .width(Length::Fill)
    .style(|theme: &Theme| {
        let ext = theme.extended_palette();
        iced::widget::container::Style {
            // background 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            background: Some(Background::Color(ext.danger.base.color.scale_alpha(0.07))),
            // border 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            border: Border {
                // width 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                width: 1.0,
                // color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                color: ext.danger.base.color.scale_alpha(0.30),
                // radius 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                radius: 14.0.into(),
            },
            ..Default::default()
        }
    })
    .into()
}

/// 构建 success body 对应的 Iced 界面片段或中间数据。
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
fn build_success_body<'a>(data: &LspToolData) -> Element<'a, Message> {
    match &data.payload {
        // LspPayload 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        LspPayload::Locations { items } => build_location_list(items),
        // LspPayload 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        LspPayload::DocumentSymbols { items } => build_symbol_list(items),
        // LspPayload 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        LspPayload::CallHierarchyItems { items } | LspPayload::CallHierarchyCalls { items } => {
            build_call_list(items)
        }
        LspPayload::Hover { contents, line, character } => {
            let mut body = String::new();
            if let (Some(line), Some(character)) = (*line, *character) {
                body.push_str(&format!("位置: {}:{}\n\n", line, character));
            }
            body.push_str(contents.trim());
            build_text_body(&body)
        }
        LspPayload::Message { message } => build_text_body(message),
    }
}

/// 构建 location list 对应的 Iced 界面片段或中间数据。
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
fn build_location_list<'a>(items: &[LspLocationItem]) -> Element<'a, Message> {
    if items.is_empty() {
        return build_text_body("未返回结构化位置结果。");
    }
    let mut column_view = column![].spacing(6);
    for item in items {
        let primary =
            item.name.clone().or_else(|| item.kind.clone()).unwrap_or_else(|| item.path.clone());
        let mut secondary = format!("{}:{}:{}", item.path, item.line, item.character);
        if let Some(detail) = item.detail.as_deref().filter(|detail| !detail.trim().is_empty()) {
            secondary.push_str(&format!(" · {}", detail));
        }
        if let Some(container_name) = item
            .container_name
            .as_deref()
            .filter(|container_name| !container_name.trim().is_empty())
        {
            secondary.push_str(&format!(" · {}", container_name));
        }
        column_view =
            column_view.push(open_item_button(item.absolute_path.clone(), primary, secondary));
    }
    build_scroll_box(column_view)
}

/// 构建 symbol list 对应的 Iced 界面片段或中间数据。
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
fn build_symbol_list<'a>(items: &[LspDocumentSymbolItem]) -> Element<'a, Message> {
    if items.is_empty() {
        return build_text_body("文档中未返回结构化符号结果。");
    }
    let mut column_view = column![].spacing(6);
    for item in items {
        let indent = "  ".repeat(item.depth);
        let primary = format!("{}{} ({})", indent, item.name, item.kind);
        let secondary = item
            .detail
            .as_deref()
            .filter(|detail| !detail.trim().is_empty())
            .map(|detail| format!("{}:{}:{} · {}", item.path, item.line, item.character, detail))
            .unwrap_or_else(|| format!("{}:{}:{}", item.path, item.line, item.character));
        column_view =
            column_view.push(open_item_button(item.absolute_path.clone(), primary, secondary));
    }
    build_scroll_box(column_view)
}

/// 构建 call list 对应的 Iced 界面片段或中间数据。
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
fn build_call_list<'a>(items: &[LspCallHierarchyItem]) -> Element<'a, Message> {
    if items.is_empty() {
        return build_text_body("未返回结构化调用层级结果。");
    }
    let mut column_view = column![].spacing(6);
    for item in items {
        let primary = format!("{} ({})", item.name, item.kind);
        let secondary = if item.call_sites.is_empty() {
            format!("{}:{}:{}", item.path, item.line, item.character)
        } else {
            let call_sites = item
                .call_sites
                .iter()
                .map(|site| format!("{}:{}", site.line, site.character))
                .collect::<Vec<_>>()
                .join(", ");
            format!("{}:{}:{} · 调用点 {}", item.path, item.line, item.character, call_sites)
        };
        column_view =
            column_view.push(open_item_button(item.absolute_path.clone(), primary, secondary));
    }
    build_scroll_box(column_view)
}

/// 构建 open item button 控件，并绑定既有消息或样式。
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
fn open_item_button<'a>(
    absolute_path: String,
    primary: String,
    secondary: String,
) -> Element<'a, Message> {
    button(
        column![
            text(truncate_chars(&primary, 120)).size(14),
            text(truncate_chars(&secondary, 180)).size(12).style(|theme: &Theme| {
                // iced 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                iced::widget::text::Style {
                    // color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                    color: Some(theme.extended_palette().secondary.base.text.scale_alpha(0.72)),
                }
            })
        ]
        .spacing(4)
        .width(Length::Fill),
    )
    .padding([8, 10])
    .width(Length::Fill)
    .style(weak_file_button_style)
    .on_press(Message::Preview(message::PreviewMessage::Open(absolute_path)))
    .into()
}

/// 构建 scroll box 对应的 Iced 界面片段或中间数据。
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
fn build_scroll_box<'a>(content: iced::widget::Column<'a, Message>) -> Element<'a, Message> {
    scrollable(
        container(content).width(Length::Fill).padding([8, 10]).style(simplified_code_block_style),
    )
    .direction(chat_scroll_direction())
    .height(Length::Fixed(220.0))
    .into()
}

/// 构建 text body 对应的 Iced 界面片段或中间数据。
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
fn build_text_body<'a>(body: &str) -> Element<'a, Message> {
    let preview = truncate_lines_middle(body.trim(), 80, 500);
    scrollable(
        container(text(preview).size(14))
            .width(Length::Fill)
            .padding([8, 10])
            .style(simplified_code_block_style),
    )
    .direction(chat_scroll_direction())
    .height(Length::Fixed(180.0))
    .into()
}
