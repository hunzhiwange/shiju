//! Git diff 工具卡片视图。
//!
//! 本模块把结构化或文本形式的 Git diff 输出转换为聊天面板中的紧凑预览。

use iced::widget::{Space, button, column, container, mouse_area, row, svg, text};
/// 重新导出 use iced::{Alignment, Background, Border, Color, Element, Length, Theme}，让上层模块通过稳定路径访问。
use iced::{Alignment, Background, Border, Color, Element, Length, Theme};
/// 重新导出 use std::collections::HashMap，让上层模块通过稳定路径访问。
use std::collections::HashMap;

/// 重新导出 use crate::app::assets::Icon，让上层模块通过稳定路径访问。
use crate::app::assets::Icon;
/// 重新导出 use crate::app::components::chat_panel::utils::{，让上层模块通过稳定路径访问。
use crate::app::components::chat_panel::utils::{
    change_pills, eye_icon_button_style, icon_svg, truncate_chars,
};
/// 重新导出 use crate::app::components::git_panel::embedded_custom_text_diff_view，让上层模块通过稳定路径访问。
use crate::app::components::git_panel::embedded_custom_text_diff_view;
/// 重新导出 use crate::app::{App, Message, message}，让上层模块通过稳定路径访问。
use crate::app::{App, Message, message};

/// 重新导出 use super::canonical_tool_name，让上层模块通过稳定路径访问。
use super::canonical_tool_name;
/// 重新导出 use super::tool_meta::{tool_header_title, tool_inline_summary}，让上层模块通过稳定路径访问。
use super::tool_meta::{tool_header_title, tool_inline_summary};
/// 重新导出 use super::tool_parse::{tool_error_text, tool_input, tool_status}，让上层模块通过稳定路径访问。
use super::tool_parse::{tool_error_text, tool_input, tool_status};

/// GitDiffPreview 保存 git_diff_view 模块需要跨函数传递的状态。
///
/// 字段保持贴近调用方的真实数据，避免在 UI 边界处隐藏额外转换。
#[derive(Clone)]
pub(super) struct GitDiffPreview {
    // path 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    pub(super) path: String,
    // before 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    pub(super) before: String,
    // after 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    pub(super) after: String,
    // additions 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    pub(super) additions: usize,
    // deletions 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    pub(super) deletions: usize,
    // cached 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    pub(super) cached: bool,
}

/// 处理 append preview line 对应的局部职责。
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
pub(super) fn append_preview_line(buf: &mut String, line: &str) {
    if !buf.is_empty() {
        buf.push('\n');
    }
    buf.push_str(line);
}

/// 处理 append preview gap 对应的局部职责。
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
pub(super) fn append_preview_gap(buf: &mut String) {
    if !buf.is_empty() && !buf.ends_with("\n\n") {
        buf.push('\n');
    }
}

/// 处理 is git diff tool 对应的局部职责。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// `true` 表示当前输入满足该辅助函数描述的条件。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
pub(super) fn is_git_diff_tool(tool_name: &str, input: &str) -> bool {
    if tool_name == "git_diff" {
        return true;
    }

    if tool_name != "git_operations" {
        return false;
    }

    serde_json::from_str::<serde_json::Value>(input.trim())
        .ok()
        .and_then(|args| args.get("operation").and_then(|v| v.as_str()).map(str::to_string))
        .is_some_and(|operation| operation == "diff")
}

/// 处理 structured git diff output 对应的局部职责。
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
pub(super) fn structured_git_diff_output(value: &serde_json::Value) -> Option<serde_json::Value> {
    value
        .get("result")
        .and_then(|result| result.get("data"))
        .filter(|data| data.get("hunks").is_some())
        .cloned()
        .or_else(|| {
            value
                .get("result")
                .and_then(|result| result.get("content"))
                .and_then(|content| content.as_array())
                .and_then(|blocks| {
                    blocks.iter().find_map(|block| {
                        (block.get("type").and_then(|item| item.as_str()) == Some("json"))
                            .then(|| block.get("value").cloned())
                            .flatten()
                            .filter(|inner| inner.get("hunks").is_some())
                    })
                })
        })
}

/// 解析 git diff previews 的输入文本，返回后续视图可以直接消费的结构化结果。
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
pub(super) fn parse_git_diff_previews(
    input: &str,
    value: &serde_json::Value,
) -> Option<Vec<GitDiffPreview>> {
    let args = serde_json::from_str::<serde_json::Value>(input.trim()).ok()?;
    let output_json = value
        .get("output")
        .and_then(|output| output.as_str())
        .and_then(|output| serde_json::from_str::<serde_json::Value>(output.trim()).ok())
        .or_else(|| structured_git_diff_output(value))?;
    let hunks = output_json.get("hunks").and_then(|v| v.as_array())?;
    let cached = args.get("cached").and_then(|v| v.as_bool()).unwrap_or(false);

    let mut previews: Vec<GitDiffPreview> = Vec::new();
    let mut indices: HashMap<String, usize> = HashMap::new();

    for hunk in hunks {
        let Some(path) = hunk.get("file").and_then(|v| v.as_str()).map(str::trim) else {
            continue;
        };
        if path.is_empty() {
            continue;
        }

        let idx = if let Some(idx) = indices.get(path).copied() {
            idx
        } else {
            let idx = previews.len();
            previews.push(GitDiffPreview {
                // path 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                path: path.to_string(),
                // before 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                before: String::new(),
                // after 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                after: String::new(),
                // additions 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                additions: 0,
                // deletions 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                deletions: 0,
                cached,
            });
            indices.insert(path.to_string(), idx);
            idx
        };

        if hunk.get("header").and_then(|v| v.as_str()).is_some() {
            append_preview_gap(&mut previews[idx].before);
            append_preview_gap(&mut previews[idx].after);
        }

        let Some(lines) = hunk.get("lines").and_then(|v| v.as_array()) else {
            continue;
        };

        for line in lines {
            let Some(raw) = line.get("text").and_then(|v| v.as_str()) else {
                continue;
            };

            if raw.starts_with("diff --git ")
                || raw.starts_with("index ")
                || raw.starts_with("new file mode ")
                || raw.starts_with("deleted file mode ")
                || raw.starts_with("similarity index ")
                || raw.starts_with("rename from ")
                || raw.starts_with("rename to ")
                || raw.starts_with("\\ No newline at end of file")
                || raw.starts_with("@@ ")
            {
                continue;
            }

            if let Some(stripped) = raw.strip_prefix('+') {
                if !raw.starts_with("+++") {
                    append_preview_line(&mut previews[idx].after, stripped);
                    previews[idx].additions = previews[idx].additions.saturating_add(1);
                }
                continue;
            }

            if let Some(stripped) = raw.strip_prefix('-') {
                if !raw.starts_with("---") {
                    append_preview_line(&mut previews[idx].before, stripped);
                    previews[idx].deletions = previews[idx].deletions.saturating_add(1);
                }
                continue;
            }

            if let Some(stripped) = raw.strip_prefix(' ') {
                append_preview_line(&mut previews[idx].before, stripped);
                append_preview_line(&mut previews[idx].after, stripped);
            }
        }
    }

    previews.retain(|preview| {
        !preview.path.is_empty()
            && (!preview.before.is_empty()
                || !preview.after.is_empty()
                || preview.additions > 0
                || preview.deletions > 0)
    });

    if previews.is_empty() { None } else { Some(previews) }
}

/// 根据主题与状态计算 tool card style。
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
pub(super) fn tool_card_style(
    theme: &Theme,
    // is_error 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    is_error: bool,
    // is_hovered 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    is_hovered: bool,
    // expanded 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    expanded: bool,
) -> iced::widget::container::Style {
    let ext = theme.extended_palette();
    let is_dark =
        theme.palette().background.r + theme.palette().background.g + theme.palette().background.b
            < 1.5;
    let base_bg = if is_error {
        ext.danger.base.color.scale_alpha(if is_dark { 0.12 } else { 0.08 })
    } else if is_hovered || expanded {
        ext.background.weak.color.scale_alpha(if is_dark { 0.30 } else { 0.75 })
    } else {
        ext.background.weak.color.scale_alpha(if is_dark { 0.18 } else { 0.52 })
    };
    let border_color = if is_error {
        ext.danger.base.color.scale_alpha(if is_dark { 0.40 } else { 0.32 })
    } else {
        ext.background.strong.color.scale_alpha(if is_dark { 0.60 } else { 0.75 })
    };

    iced::widget::container::Style {
        // background 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        background: Some(Background::Color(base_bg)),
        // border 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        border: Border { width: 1.0, color: border_color, radius: 14.0.into() },
        ..Default::default()
    }
}

/// 根据主题与状态计算 tool content style。
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
pub(super) fn tool_content_style(theme: &Theme, is_error: bool) -> iced::widget::container::Style {
    let ext = theme.extended_palette();
    let is_dark =
        theme.palette().background.r + theme.palette().background.g + theme.palette().background.b
            < 1.5;
    let background = if is_error {
        ext.danger.base.color.scale_alpha(if is_dark { 0.10 } else { 0.07 })
    } else {
        ext.background.base.color.scale_alpha(if is_dark { 0.40 } else { 0.92 })
    };
    let border = if is_error {
        ext.danger.base.color.scale_alpha(if is_dark { 0.35 } else { 0.28 })
    } else {
        ext.background.strong.color.scale_alpha(if is_dark { 0.55 } else { 0.75 })
    };

    iced::widget::container::Style {
        // background 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        background: Some(Background::Color(background)),
        // border 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        border: Border { width: 1.0, color: border, radius: 12.0.into() },
        ..Default::default()
    }
}

/// 构建 tool item button style 控件，并绑定既有消息或样式。
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
pub(super) fn tool_item_button_style(
    theme: &Theme,
    // status 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let ext = theme.extended_palette();
    let is_dark =
        theme.palette().background.r + theme.palette().background.g + theme.palette().background.b
            < 1.5;
    let background = match status {
        // iced 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        iced::widget::button::Status::Pressed => {
            Some(ext.background.strong.color.scale_alpha(if is_dark { 0.42 } else { 0.35 }))
        }
        iced::widget::button::Status::Hovered => {
            Some(ext.background.weak.color.scale_alpha(if is_dark { 0.38 } else { 0.78 }))
        }
        _ => Some(ext.background.base.color.scale_alpha(if is_dark { 0.16 } else { 0.96 })),
    };

    iced::widget::button::Style {
        // background 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        background: background.map(Background::Color),
        // border 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        border: Border {
            // width 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            width: 1.0,
            // color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            color: ext.background.strong.color.scale_alpha(if is_dark { 0.55 } else { 0.82 }),
            // radius 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            radius: 11.0.into(),
        },
        // text_color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        text_color: theme.palette().text,
        ..Default::default()
    }
}

/// 处理 secondary text 对应的局部职责。
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
pub(super) fn secondary_text(theme: &Theme, dark_alpha: f32, light_alpha: f32) -> Color {
    let is_dark =
        theme.palette().background.r + theme.palette().background.g + theme.palette().background.b
            < 1.5;
    if is_dark {
        theme.palette().text.scale_alpha(dark_alpha)
    } else {
        theme.extended_palette().secondary.base.text.scale_alpha(light_alpha)
    }
}

/// 处理 diff pill 对应的局部职责。
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
fn diff_pill<'a>() -> Element<'a, Message> {
    container(text("查看 Diff").size(14).style(|theme: &Theme| iced::widget::text::Style {
        // color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        color: Some(secondary_text(theme, 0.9, 0.86)),
    }))
    .padding([3, 8])
    .style(|theme: &Theme| {
        let ext = theme.extended_palette();
        let is_dark = theme.palette().background.r
            + theme.palette().background.g
            + theme.palette().background.b
            < 1.5;
        iced::widget::container::Style {
            // background 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            background: Some(Background::Color(if is_dark {
                ext.background.weak.color.scale_alpha(0.28)
            } else {
                ext.background.base.color.scale_alpha(0.35)
            })),
            // border 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            border: Border {
                // width 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                width: 1.0,
                // color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                color: ext.background.strong.color.scale_alpha(if is_dark { 0.48 } else { 0.72 }),
                // radius 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                radius: 999.0.into(),
            },
            // text_color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            text_color: Some(secondary_text(theme, 0.9, 0.86)),
            ..Default::default()
        }
    })
    .into()
}

/// 处理 git diff title 对应的局部职责。
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
pub(super) fn git_diff_title(preview: &GitDiffPreview) -> String {
    let mut title = preview.path.clone();
    if preview.additions > 0 || preview.deletions > 0 {
        title.push_str(&format!("  +{}-{}", preview.additions, preview.deletions));
    }
    if preview.cached {
        title.push_str("  cached");
    }
    title
}

/// 渲染 git diff content 对应的 diff 行、工具卡片或控件内容。
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
pub(super) fn render_git_diff_content<'a>(
    app: &'a App,
    // title 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    title: String,
    // path 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    path: Option<String>,
    // before 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    before: String,
    // after 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    after: String,
) -> Element<'a, Message> {
    container(embedded_custom_text_diff_view(app, title, path, before, after, None))
        .width(Length::Fill)
        .style(|theme: &Theme| tool_content_style(theme, false))
        .padding(6)
        .into()
}

/// 处理 git diff body 对应的局部职责。
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
pub(super) fn git_diff_body<'a>(
    app: &'a App,
    // previews 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    previews: &[GitDiffPreview],
    // expanded 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    expanded: bool,
) -> Option<Element<'a, Message>> {
    if previews.is_empty() {
        return None;
    }

    let visible_count = if expanded { previews.len() } else { previews.len().min(3) };
    let mut list = column![].spacing(8);

    for preview in previews.iter().take(visible_count) {
        let title = git_diff_title(preview);
        let path = preview.path.clone();
        let before = preview.before.clone();
        let after = preview.after.clone();
        let label = if preview.cached {
            format!("{} · cached", truncate_chars(&preview.path, 88))
        } else {
            truncate_chars(&preview.path, 88).to_string()
        };

        let button_row = row![
            row![
                icon_svg(Icon::GitBranch).width(Length::Fixed(12.0)).height(Length::Fixed(12.0)),
                text(label).size(14).style(|theme: &Theme| iced::widget::text::Style {
                    // color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                    color: Some(secondary_text(theme, 0.94, 0.92)),
                })
            ]
            .spacing(8)
            .align_y(Alignment::Center),
            container(Space::new()).width(Length::Fill),
            change_pills(preview.additions, preview.deletions),
            diff_pill()
        ]
        .spacing(10)
        .align_y(Alignment::Center);

        let open_diff = Message::Git(message::GitMessage::OpenChatTextDiff {
            // title 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            title: title.clone(),
            // file 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            file: path.clone(),
            // before 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            before: before.clone(),
            // after 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            after: after.clone(),
        });
        let header_button: Element<'a, Message> = button(button_row)
            .padding([8, 10])
            .width(Length::Fill)
            .style(tool_item_button_style)
            .on_press(open_diff)
            .into();

        if expanded {
            let embedded = render_git_diff_content(app, title, Some(path), before, after);
            list = list.push(column![header_button, embedded].spacing(6));
        } else {
            list = list.push(header_button);
        }
    }

    if previews.len() > visible_count {
        list = list.push(
            container(
                text(format!("还有 {} 个文件", previews.len() - visible_count)).size(14).style(
                    |theme: &Theme| iced::widget::text::Style {
                        // color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                        color: Some(secondary_text(theme, 0.78, 0.72)),
                    },
                ),
            )
            .padding([2, 4])
            .width(Length::Fill),
        );
    }

    Some(list.into())
}

/// 处理 tool git diff view 对应的局部职责。
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
pub fn tool_git_diff_view<'a>(
    app: &'a App,
    // msg_idx 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    msg_idx: usize,
    // tool_idx 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    tool_idx: usize,
    // visible 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    visible: &str,
) -> Option<Element<'a, Message>> {
    let (first, rest) = visible.split_once('\n')?;
    let tool_name = canonical_tool_name(first.trim().strip_prefix("tool ")?.trim());

    let v = serde_json::from_str::<serde_json::Value>(rest.trim()).ok()?;
    let status = tool_status(&v);
    let input = tool_input(&v).trim();
    if !is_git_diff_tool(tool_name, input) {
        return None;
    }

    let is_error = matches!(status, "error" | "denied");
    let is_running = status == "running";
    let err_text = tool_error_text(&v).unwrap_or_default();
    let previews = if is_error { None } else { parse_git_diff_previews(input, &v) };
    if previews.is_none() && err_text.is_empty() && !is_running {
        return None;
    }

    let expanded = true;
    let key = ((msg_idx as u64) << 32) | (tool_idx as u64);
    let is_hovered = app.chat_tool_hovered_idx == Some(key);

    let title = if is_running {
        "Git Diff 中"
    } else if is_error {
        "Git Diff 失败"
    } else {
        "Git Diff"
    };

    let mut summary = tool_inline_summary(tool_name, input).unwrap_or_default();
    if summary.is_empty()
        && let Some(previews) = previews.as_ref()
    {
        summary = if previews.len() == 1 {
            previews[0].path.clone()
        } else {
            format!("{} 个文件", previews.len())
        };
    }

    let header_meta: Option<Element<'a, Message>> = if let Some(previews) = previews.as_ref() {
        let adds = previews.iter().map(|item| item.additions).sum::<usize>();
        let dels = previews.iter().map(|item| item.deletions).sum::<usize>();
        Some(change_pills(adds, dels))
    } else {
        None
    };

    let detail_btn: Element<'a, Message> = button(
        icon_svg(Icon::ChevronRight).width(Length::Fixed(10.0)).height(Length::Fixed(10.0)).style(
            |theme: &Theme, _status| {
                let is_dark = theme.palette().background.r
                    + theme.palette().background.g
                    + theme.palette().background.b
                    < 1.5;
                svg::Style {
                    // color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                    color: Some(if is_dark {
                        theme.palette().text.scale_alpha(0.92)
                    } else {
                        theme.extended_palette().secondary.base.text.scale_alpha(0.90)
                    }),
                }
            },
        ),
    )
    .padding([2, 4])
    .style(|theme: &Theme, status| eye_icon_button_style(theme, status))
    .on_press(Message::Chat(message::ChatMessage::OpenToolDetail(
        msg_idx,
        tool_idx,
        visible.to_string(),
    )))
    .into();

    let mut head_row = row![
        tool_header_title("git_diff", title, is_error),
        container(icon_svg(Icon::GitBranch).width(Length::Fixed(13.0)).height(Length::Fixed(13.0)))
            .padding([5, 6])
            .style(|theme: &Theme| {
                let ext = theme.extended_palette();
                let is_dark = theme.palette().background.r
                    + theme.palette().background.g
                    + theme.palette().background.b
                    < 1.5;
                iced::widget::container::Style {
                    // background 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                    background: Some(Background::Color(if is_dark {
                        ext.background.weak.color.scale_alpha(0.30)
                    } else {
                        ext.background.base.color.scale_alpha(0.28)
                    })),
                    // border 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                    border: Border {
                        // width 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                        width: 1.0,
                        // color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                        color: ext.background.strong.color.scale_alpha(if is_dark {
                            0.5
                        } else {
                            0.64
                        }),
                        // radius 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                        radius: 10.0.into(),
                    },
                    ..Default::default()
                }
            }),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    if !summary.is_empty() {
        head_row = head_row.push(text(summary).size(13).style(|theme: &Theme| {
            // iced 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            iced::widget::text::Style { color: Some(secondary_text(theme, 0.72, 0.72)) }
        }));
    }

    head_row = head_row.push(container(Space::new()).width(Length::Fill));

    if let Some(meta) = header_meta {
        head_row = head_row.push(meta);
    }

    head_row = head_row.push(detail_btn);

    let head = mouse_area(container(head_row).width(Length::Fill).padding([2, 2]))
        .on_enter(Message::Chat(message::ChatMessage::ToolHover(msg_idx, tool_idx)))
        .on_exit(Message::Chat(message::ChatMessage::ToolHoverLeave));

    let body: Element<'a, Message> = if let Some(previews) = previews.as_ref() {
        let git_body = git_diff_body(app, previews, expanded)?;
        container(git_body).width(Length::Fill).into()
    } else {
        let err = if err_text.is_empty() { "Git diff 处理中…".to_string() } else { err_text };
        container(text(err).size(13).style(move |theme: &Theme| iced::widget::text::Style {
            // color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            color: Some(if is_error {
                theme.extended_palette().danger.base.color
            } else {
                secondary_text(theme, 0.82, 0.82)
            }),
        }))
        .padding([10, 12])
        .width(Length::Fill)
        .style(move |theme: &Theme| tool_content_style(theme, is_error))
        .into()
    };

    Some(
        container(column![head, body].spacing(10))
            .padding([10, 12])
            .width(Length::Fill)
            .style(move |theme: &Theme| tool_card_style(theme, is_error, is_hovered, expanded))
            .into(),
    )
}
