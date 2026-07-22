//! 提问工具结果视图。
//!
//! 本模块关联待回答问题与聊天消息，并把用户答案或等待状态展示在工具卡片中。

use std::collections::BTreeMap;

/// 重新导出 use iced::widget::{Space, button, column, container, mouse_area, row, text}，让上层模块通过稳定路径访问。
use iced::widget::{Space, button, column, container, mouse_area, row, text};
/// 重新导出 use iced::{Alignment, Element, Length, Theme}，让上层模块通过稳定路径访问。
use iced::{Alignment, Element, Length, Theme};

/// 重新导出 use crate::app::assets::Icon，让上层模块通过稳定路径访问。
use crate::app::assets::Icon;
/// 重新导出 use crate::app::components::chat_panel::utils::{，让上层模块通过稳定路径访问。
use crate::app::components::chat_panel::utils::{
    bold_font, chat_secondary_muted_text_color, chat_secondary_text_color, eye_icon_button_style,
    eye_icon_svg_style, icon_svg, simplified_block_style, truncate_chars,
};
/// 重新导出 use crate::app::{App, Message, message}，让上层模块通过稳定路径访问。
use crate::app::{App, Message, message};

/// 重新导出 use super::canonical_tool_name，让上层模块通过稳定路径访问。
use super::canonical_tool_name;
/// 重新导出 use super::tool_meta::tool_header_title，让上层模块通过稳定路径访问。
use super::tool_meta::tool_header_title;
/// 重新导出 use super::tool_parse::{tool_input, tool_output_text, tool_result_data, tool_status, tool_summary_text}，让上层模块通过稳定路径访问。
use super::tool_parse::{
    tool_input, tool_output_text, tool_result_data, tool_status, tool_summary_text,
};

/// 处理 question request targets message 对应的局部职责。
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
pub(crate) fn question_request_targets_message(
    request: Option<&vw_shared::question::Request>,
    // message_id 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    message_id: Option<&str>,
) -> bool {
    let Some(request) = request else {
        return false;
    };
    let Some(tool_meta) = request.tool.as_ref() else {
        return true;
    };
    let Some(message_id) = message_id else {
        return false;
    };

    tool_meta.message_id == message_id
}

/// 处理 question request matches message 对应的局部职责。
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
fn question_request_matches_message(app: &App, msg_idx: usize) -> bool {
    question_request_targets_message(
        app.question_modal_request.as_ref(),
        app.chat_message_ids.get(msg_idx).and_then(|value| value.as_deref()),
    )
}

/// 解析 questions 的输入文本，返回后续视图可以直接消费的结构化结果。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 返回集合保持输入顺序或界面展示顺序，空集合表示没有可展示项。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
pub(super) fn parse_questions(input: &str) -> Vec<vw_shared::question::Info> {
    let trimmed = input.trim();
    if !(trimmed.starts_with('{') || trimmed.starts_with('[')) {
        return Vec::new();
    }

    let Ok(value) = serde_json::from_str::<serde_json::Value>(trimmed) else {
        return Vec::new();
    };

    let questions =
        if value.is_array() { value } else { value.get("questions").cloned().unwrap_or_default() };

    serde_json::from_value::<Vec<vw_shared::question::Info>>(questions).ok().unwrap_or_default()
}

/// 解析 answers 的输入文本，返回后续视图可以直接消费的结构化结果。
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
pub(super) fn parse_answers(value: &serde_json::Value, output: &str) -> BTreeMap<String, String> {
    if let Some(data) = tool_result_data(value)
        && let Some(answers) = data.get("answers")
        && let Ok(map) = serde_json::from_value::<BTreeMap<String, String>>(answers.clone())
    {
        return map;
    }

    let legacy = serde_json::from_str::<Vec<Vec<String>>>(output.trim()).unwrap_or_default();
    legacy
        .into_iter()
        .enumerate()
        .map(|(index, answers)| {
            let text = answers
                .into_iter()
                .map(|answer| {
                    answer.strip_prefix("__custom__:").unwrap_or(answer.as_str()).to_string()
                })
                .collect::<Vec<_>>()
                .join(" / ");
            (index.to_string(), text)
        })
        .collect()
}

/// 生成 derived summary，用于工具卡片或状态行的简短说明。
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
pub(super) fn derived_summary(
    app: &App,
    // questions 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    questions: &[vw_shared::question::Info],
    // answers 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    answers: &BTreeMap<String, String>,
    // is_running 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    is_running: bool,
    // summary 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    summary: &str,
) -> String {
    if !summary.trim().is_empty() {
        return summary.trim().to_string();
    }

    if is_running {
        if app.question_modal_request.is_some() {
            return "等待你的回答".to_string();
        }
        return match questions.len() {
            0 => "等待回答".to_string(),
            1 => "等待 1 个问题的回答".to_string(),
            count => format!("等待 {} 个问题的回答", count),
        };
    }

    if !answers.is_empty() {
        return match answers.len() {
            1 => "已回答 1 个问题".to_string(),
            count => format!("已回答 {} 个问题", count),
        };
    }

    match questions.len() {
        0 => String::new(),
        1 => "1 个问题".to_string(),
        count => format!("{} 个问题", count),
    }
}

/// 处理 tool question view 对应的局部职责。
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
pub fn tool_question_view<'a>(
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
    if tool_name != "question" {
        return None;
    }

    let value = serde_json::from_str::<serde_json::Value>(rest.trim()).ok()?;
    let status = tool_status(&value);
    let is_running = status == "running";
    let is_error = matches!(status, "error" | "denied");
    let input = tool_input(&value);
    let output = tool_output_text(&value).unwrap_or_default();
    let questions = parse_questions(input);
    let answers = if is_error || is_running {
        // BTreeMap 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        BTreeMap::new()
    } else {
        parse_answers(&value, &output)
    };
    let is_active_request = question_request_matches_message(app, msg_idx);
    let summary = derived_summary(
        app,
        &questions,
        &answers,
        is_running && is_active_request,
        tool_summary_text(&value).as_deref().unwrap_or_default(),
    );

    if questions.is_empty() && summary.is_empty() {
        return None;
    }

    let key = ((msg_idx as u64) << 32) | (tool_idx as u64);
    let is_hovered = app.chat_tool_hovered_idx == Some(key);

    let title = if is_running && is_active_request {
        "等待回答"
    } else if is_error {
        "提问失败"
    } else {
        "提问"
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
    let detail_slot: Element<'a, Message> =
        if is_hovered { detail_btn.into() } else { Space::new().width(Length::Fixed(22.0)).into() };

    let head_row = row![
        row![
            tool_header_title(tool_name, title, is_error),
            text(summary).size(13).style(|theme: &Theme| iced::widget::text::Style {
                // color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                color: Some(chat_secondary_muted_text_color(theme)),
            })
        ]
        .spacing(10)
        .align_y(Alignment::Center),
        container(Space::new()).width(Length::Fill),
        detail_slot,
    ]
    .align_y(Alignment::Center);

    let mut body = column![].spacing(8);
    for (index, question) in questions.iter().take(3).enumerate() {
        let label = if question.header.trim().is_empty() {
            format!("问题 {}", index + 1)
        } else {
            truncate_chars(question.header.trim(), 96)
        };
        let prompt = truncate_chars(question.question.trim(), 180);
        let answer_text = answers
            .get(question.question.as_str())
            .or_else(|| answers.get(index.to_string().as_str()))
            .filter(|value| !value.trim().is_empty())
            .cloned()
            .unwrap_or_default();

        let mut item = column![
            text(label).size(13).font(bold_font()).style(|theme: &Theme| {
                iced::widget::text::Style {
                    // color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                    color: Some(chat_secondary_text_color(theme)),
                }
            }),
            text(prompt).size(13).style(|theme: &Theme| iced::widget::text::Style {
                // color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                color: Some(chat_secondary_muted_text_color(theme)),
            })
        ]
        .spacing(4);

        if !answer_text.trim().is_empty() {
            item = item.push(
                text(format!("回答: {}", truncate_chars(answer_text.trim(), 160))).size(12).style(
                    |theme: &Theme| iced::widget::text::Style {
                        // color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                        color: Some(chat_secondary_text_color(theme)),
                    },
                ),
            );
        }

        body = body.push(item);
    }

    if questions.len() > 3 {
        body = body.push(text(format!("还有 {} 个问题", questions.len() - 3)).size(12).style(
            |theme: &Theme| iced::widget::text::Style {
                // color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                color: Some(chat_secondary_muted_text_color(theme)),
            },
        ));
    }

    let card = mouse_area(
        container(column![head_row, body].spacing(10))
            .width(Length::Fill)
            .style(simplified_block_style),
    )
    .on_enter(Message::Chat(message::ChatMessage::ToolHover(msg_idx, tool_idx)))
    .on_exit(Message::Chat(message::ChatMessage::ToolHoverLeave));

    Some(card.into())
}
