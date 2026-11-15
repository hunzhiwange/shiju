//! 探索工具摘要视图模块
//!
//! 本模块提供了用于展示代码库探索操作摘要的 UI 组件。
//! 主要功能包括：
//! - 统计和展示读取、搜索、列出文件等操作的次数
//! - 提供可展开/折叠的交互式摘要视图
//! - 支持运行中状态和已完成状态的视觉区分
//! - 为不同类型的探索工具提供后备视图

use std::collections::HashMap;

use iced::widget::{Space, button, column, container, mouse_area, row, svg, text};
use iced::{Alignment, Element, Length, Theme};
use std::collections::HashSet;

use desktop_assets::assets::{self, Icon};
use desktop_ui_controls::animated_text::neutral_sweep_text_color;
use crate::tool_text_support::{chat_text_font, tool_text_key};
use crate::card_style::{
    chat_secondary_subtle_text_color, chat_secondary_text_color, icon_svg, truncate_chars,
};
use desktop_ui_controls::status_animation::{
    EXPLORE_SUMMARY_FLIP_DURATION_MS, activity_spinner,
};
use crate::card_input::Message;
use super::explore_input::ExploreInput;

use super::tool_meta::tool_header_label;
use super::types::{EXPLORE_GROUP_TOOL_IDX, ExploreItem};
use super::tool_parse::{ExploreToolKind, explore_item_dedupe_key, explore_tool_kind, tool_name_from_raw, tool_status_from_raw};
use crate::tool_text_support::{ToolTextTarget, tool_inline_text_editor};
use super::canonical_tool_name;
#[cfg(test)]
pub(super) use super::summary_animation::right_aligned_slot_char;
use super::summary_animation::flip_number_view;

pub(super) const EXPANDED_LIST_TOP_SPACING: f32 = 10.0;
const EXPLORE_TOGGLE_SLOT_WIDTH: f32 = 12.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SummarySegmentKind {
    Text,
    Number,
}

pub(super) fn summary_animation_key(msg_idx: usize, group_idx: usize) -> u128 {
    ((msg_idx as u128) << 64) | (group_idx as u128)
}

pub fn explore_summary_is_running(
    has_running_tool: bool,
    force_running: bool,
    closed_by_following_block: bool,
) -> bool {
    !closed_by_following_block && (has_running_tool || force_running)
}

pub fn explore_summary_expanded(
    _has_running_tool: bool,
    key: u64,
    expanded_groups: &HashSet<u64>,
) -> bool {
    expanded_groups.contains(&key)
}

pub(super) fn split_summary_segments(input: &str) -> Vec<(SummarySegmentKind, &str)> {
    let mut segments = Vec::new();
    let mut start = 0usize;
    let mut in_number = None::<bool>;

    for (idx, ch) in input.char_indices() {
        let is_number = ch.is_ascii_digit();
        match in_number {
            None => in_number = Some(is_number),
            Some(current_kind) if current_kind != is_number => {
                segments.push((
                    if current_kind {
                        SummarySegmentKind::Number
                    } else {
                        SummarySegmentKind::Text
                    },
                    &input[start..idx],
                ));
                start = idx;
                in_number = Some(is_number);
            }
            _ => {}
        }
    }

    if let Some(current_kind) = in_number {
        segments.push((
            if current_kind { SummarySegmentKind::Number } else { SummarySegmentKind::Text },
            &input[start..],
        ));
    }

    segments
}

fn plain_summary_text<'a>(content: &str) -> Element<'a, Message> {
    text(content.to_string())
        .size(13)
        .font(chat_text_font())
        .style(|theme: &Theme| iced::widget::text::Style {
            color: Some(chat_secondary_subtle_text_color(theme)),
        })
        .into()
}

fn running_explore_title<'a>(
    title: &str,
    now_ms: u64,
    animation_frame: usize,
) -> Element<'a, Message> {
    let char_count = title.chars().count().max(1);
    let spinner = activity_spinner(animation_frame, 13.0).style(|theme: &Theme, _status| {
        svg::Style { color: Some(chat_secondary_text_color(theme)) }
    });
    let mut content = row![spinner].spacing(4).align_y(Alignment::Center);

    for (char_idx, character) in title.chars().enumerate() {
        content = content.push(text(character.to_string()).size(13).font(chat_text_font()).style(
            move |theme: &Theme| iced::widget::text::Style {
                color: Some(neutral_sweep_text_color(
                    theme,
                    chat_secondary_text_color(theme),
                    now_ms,
                    char_idx,
                    char_count,
                    true,
                )),
            },
        ));
    }

    content.into()
}

fn completed_explore_title<'a>(title: &str) -> Element<'a, Message> {
    text(title.to_string())
        .size(13)
        .font(chat_text_font())
        .style(|theme: &Theme| iced::widget::text::Style {
            color: Some(chat_secondary_subtle_text_color(theme)),
        })
        .into()
}

fn compact_eye_button_style(
    theme: &Theme,
    _status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    iced::widget::button::Style {
        background: None,
        border: iced::Border { width: 0.0, color: iced::Color::TRANSPARENT, radius: 0.0.into() },
        text_color: chat_secondary_subtle_text_color(theme),
        shadow: iced::Shadow::default(),
        ..Default::default()
    }
}

fn animated_summary_slot<'a>(
    app: &ExploreInput<'a>,
    msg_idx: usize,
    group_idx: usize,
    summary_text: &str,
) -> Option<Element<'a, Message>> {
    let state =
        app.chat_explore_summary_animations.get(&summary_animation_key(msg_idx, group_idx))?;
    let changed_at_ms = state.changed_at_ms?;
    if state.current_summary_text != summary_text {
        return None;
    }

    let elapsed_ms = app.now_ms.saturating_sub(changed_at_ms);
    if elapsed_ms >= EXPLORE_SUMMARY_FLIP_DURATION_MS {
        return None;
    }

    let previous_numbers = split_summary_segments(&state.previous_summary_text)
        .into_iter()
        .filter_map(|(kind, segment)| (kind == SummarySegmentKind::Number).then_some(segment))
        .collect::<Vec<_>>();

    let progress = (elapsed_ms as f32 / EXPLORE_SUMMARY_FLIP_DURATION_MS as f32).clamp(0.0, 1.0);
    let mut has_animated_number = false;
    let mut number_idx = 0usize;
    let mut summary_row = row![].spacing(0).align_y(Alignment::Center);

    for (kind, segment) in split_summary_segments(summary_text) {
        match kind {
            SummarySegmentKind::Text => {
                summary_row = summary_row.push(plain_summary_text(segment));
            }
            SummarySegmentKind::Number => {
                let previous = previous_numbers.get(number_idx).copied().unwrap_or("0");
                if previous != segment {
                    has_animated_number = true;
                    summary_row =
                        summary_row.push(flip_number_view(previous, segment, progress, 13.0));
                } else {
                    summary_row = summary_row.push(plain_summary_text(segment));
                }
                number_idx += 1;
            }
        }
    }

    has_animated_number.then_some(summary_row.into())
}

fn explore_item_compact_view<'a>(
    app: &ExploreInput<'a>,
    msg_idx: usize,
    item: &ExploreItem,
) -> Option<Element<'a, Message>> {
    let (first, rest) = item.raw.split_once('\n')?;
    let tool_name = canonical_tool_name(first.trim().strip_prefix("tool ")?.trim());
    if tool_name.is_empty() {
        return None;
    }
    let v = serde_json::from_str::<serde_json::Value>(rest.trim()).ok()?;
    let input = v.get("input").and_then(|v| v.as_str()).unwrap_or("").trim();
    let summary = crate::tools::localized_tool_inline_summary(
        app.i18n,
        tool_name,
        input,
    )
    .unwrap_or_default();
    let summary = truncate_chars(summary.replace(['\n', '\r'], " ").trim(), 64);

    let key = ((msg_idx as u64) << 32) | (item.tool_idx as u64);
    let is_hovered = app.hovered == Some(key);
    let eye_icon = iced::widget::svg::Svg::new(assets::get_icon(Icon::ChevronRight))
        .width(Length::Fixed(5.0))
        .height(Length::Fixed(5.0))
        .style(|theme: &Theme, _status| iced::widget::svg::Style {
            color: Some(chat_secondary_subtle_text_color(theme)),
        });
    let detail_btn = button(eye_icon)
        .width(Length::Fixed(10.0))
        .height(Length::Fixed(10.0))
        .padding(0)
        .style(compact_eye_button_style)
        .on_press(Message::OpenToolDetail(
            msg_idx,
            item.tool_idx,
            item.raw.clone(),
        ));
    let detail_slot: Element<'a, Message> = if is_hovered {
        detail_btn.into()
    } else {
        Space::new().width(Length::Fixed(10.0)).height(Length::Fixed(10.0)).into()
    };

    let row_container = container(
        row![
            text(
                app.i18n
                    .__(&tool_header_label(tool_name))
                    .to_string()
            )
            .size(13)
            .font(chat_text_font())
            .style(|theme: &Theme| {
                iced::widget::text::Style {
                    color: Some(chat_secondary_text_color(theme).scale_alpha(0.82)),
                }
            },),
            text(summary).size(13).font(chat_text_font()).style(|theme: &Theme| {
                iced::widget::text::Style { color: Some(chat_secondary_subtle_text_color(theme)) }
            }),
            detail_slot,
            container(Space::new()).width(Length::Fill)
        ]
        .spacing(6)
        .align_y(Alignment::Center),
    )
    .width(Length::Fill)
    .padding([1, 0]);
    Some(
        mouse_area(row_container)
            .on_enter(Message::ToolHover(msg_idx, item.tool_idx))
            .on_exit(Message::ToolHoverLeave)
            .into(),
    )
}

pub(super) fn latest_explore_items(items: &[ExploreItem]) -> Vec<&ExploreItem> {
    let item_keys = items.iter().map(|item| explore_item_dedupe_key(&item.raw)).collect::<Vec<_>>();
    let mut last_index_by_key: HashMap<&str, usize> = HashMap::new();

    for (idx, key) in item_keys.iter().enumerate() {
        if let Some(key) = key.as_deref() {
            last_index_by_key.insert(key, idx);
        }
    }

    items
        .iter()
        .enumerate()
        .filter(|(idx, _)| {
            item_keys[*idx]
                .as_deref()
                .and_then(|key| last_index_by_key.get(key))
                .is_none_or(|last_idx| *last_idx == *idx)
        })
        .map(|(_, item)| item)
        .collect()
}

/// 创建探索工具的摘要视图
///
/// 该函数用于创建一个可交互的探索工具摘要视图，展示代码库探索操作的统计信息。
/// 支持展开/折叠操作，显示读取、搜索、列出文件等操作的次数统计，
/// 并根据工具的运行状态（运行中/已完成）提供不同的视觉反馈。
///
/// # 参数
///
/// * `app` - 应用程序状态引用，用于访问展开状态和悬停状态
/// * `msg_idx` - 消息索引，用于标识消息在消息列表中的位置
/// * `group_idx` - 分组索引，用于计算工具的唯一键值
/// * `items` - 探索项列表，包含所有需要展示的探索工具数据
///
/// # 返回值
///
/// 如果 `items` 非空，返回 `Some(Element)` 包含完整的摘要视图；
/// 如果 `items` 为空，返回 `None`。
///
/// # 视图结构
///
/// 视图包含以下部分：
/// 1. 标题行：显示"正在探索"或"已探索"，以及操作统计信息
/// 2. 详细列表：显示每个探索工具的具体视图
///
/// # 状态管理
///
/// - 悬停状态通过 `app.hovered` 管理
pub fn tool_explore_summary_view<'a>(
    app: &ExploreInput<'a>,
    msg_idx: usize,
    group_idx: usize,
    items: &[ExploreItem],
    force_running: bool,
    closed_by_following_block: bool,
) -> Option<Element<'a, Message>> {
    // 空列表直接返回 None
    if items.is_empty() {
        return None;
    }

    let latest_items = latest_explore_items(items);

    // 统计各类操作的次数
    let mut read_count = 0usize; // 读取文件次数
    let mut search_count = 0usize; // 搜索操作次数
    let mut ls_count = 0usize; // 列出目录次数
    let has_running_tool = latest_items
        .iter()
        .any(|item| tool_status_from_raw(&item.raw).as_deref() == Some("running"));
    let has_running =
        explore_summary_is_running(has_running_tool, force_running, closed_by_following_block);

    // 遍历所有探索项，统计操作次数和运行状态
    for item in &latest_items {
        if let Some(name) = tool_name_from_raw(&item.raw) {
            match explore_tool_kind(&name) {
                Some(ExploreToolKind::Read) => read_count += 1,
                Some(ExploreToolKind::Search) => search_count += 1,
                Some(ExploreToolKind::List) => ls_count += 1,
                None => {}
            }
        }
    }

    let summary_text = desktop_i18n::chat::ExploreCounts {
        reads: read_count,
        searches: search_count,
        listings: ls_count,
        others: 0,
    }
    .render(app.i18n);
    let summary_text = truncate_chars(summary_text.replace(['\n', '\r'], " ").trim(), 64);

    // 计算工具的唯一标识键
    // 键的计算基于消息索引和分组索引，运行中和已完成状态使用不同的键
    let group_base_idx = EXPLORE_GROUP_TOOL_IDX.saturating_sub(group_idx.saturating_mul(2));
    let group_running_idx = group_base_idx.saturating_sub(1);
    // 后续可见块会结束摘要文案上的运行态，但仍沿用运行态槽位来保留展开状态与文本编辑器映射。
    let use_running_group_slot = has_running_tool || force_running;
    let group_tool_idx = if use_running_group_slot { group_running_idx } else { group_base_idx };
    let key = ((msg_idx as u64) << 32) | (group_tool_idx as u64);

    let expanded = explore_summary_expanded(has_running, key, &app.chat_explore_expanded);

    // 获取当前时间，用于动画效果
    let now_ms = app.now_ms;

    let title = if has_running {
        app.i18n.__("正在探索")
    } else {
        app.i18n.__("已探索")
    };
    let summary_slot: Element<'a, Message> =
        animated_summary_slot(app, msg_idx, group_idx, &summary_text).unwrap_or_else(|| {
            let cached_summary_matches = app.text
                .chat_tool_text_editors
                .get(&tool_text_key(msg_idx, group_tool_idx, 0))
                .is_some_and(|editor| editor.text() == summary_text);

            if cached_summary_matches {
                tool_inline_text_editor(
                    &app.text,
                    ToolTextTarget::ToolCardText { msg_idx, tool_idx: group_tool_idx, text_idx: 0 },
                    crate::tool_text_support::chat_text_font_name(),
                    13.0,
                    chat_secondary_subtle_text_color,
                )
                .map(|e| e.map(Message::Text))
                .unwrap_or_else(|| plain_summary_text(&summary_text))
            } else {
                plain_summary_text(&summary_text)
            }
        });
    let title_view = if has_running {
        running_explore_title(title, now_ms, app.status_animation_frame)
    } else {
        completed_explore_title(title)
    };
    let toggle_btn = button(
        icon_svg(if expanded { Icon::ChevronDown } else { Icon::ChevronRight })
            .width(Length::Fixed(10.0))
            .height(Length::Fixed(10.0))
            .style(|theme: &Theme, _status| iced::widget::svg::Style {
                color: Some(chat_secondary_subtle_text_color(theme)),
            }),
    )
    .padding([0, 1])
    .style(|theme: &Theme, _status| iced::widget::button::Style {
        background: None,
        border: iced::Border { width: 0.0, color: iced::Color::TRANSPARENT, radius: 0.0.into() },
        text_color: chat_secondary_subtle_text_color(theme),
        ..Default::default()
    })
    .on_press(Message::ToggleExploreSummary(msg_idx, group_tool_idx));
    let toggle_slot: Element<'a, Message> = if app.hovered == Some(key) {
        toggle_btn.into()
    } else {
        Space::new().width(Length::Fixed(EXPLORE_TOGGLE_SLOT_WIDTH)).into()
    };
    let head_row =
        row![title_view, summary_slot, toggle_slot, container(Space::new()).width(Length::Fill)]
            .spacing(7)
            .align_y(Alignment::Center);

    // 创建可交互的头部区域
    let head = mouse_area(container(head_row).width(Length::Fill))
        .on_enter(Message::ToolHover(msg_idx, group_tool_idx))
        .on_exit(Message::ToolHoverLeave);

    let mut content = column![head].spacing(EXPANDED_LIST_TOP_SPACING);
    if expanded {
        let mut list = column![].spacing(3);
        let mut has_visible_items = false;
        for item in latest_items {
            if let Some(view) = explore_item_compact_view(app, msg_idx, item) {
                list = list.push(view);
                has_visible_items = true;
            }
        }
        if has_visible_items {
            content = content.push(container(list).width(Length::Fill));
        }
    }

    Some(container(content).padding(0).width(Length::Fill).into())
}
