//! 思考块视图组件。

use iced::widget::svg::{self};
use iced::widget::{column, container, mouse_area, row, text, text_editor};
use iced::{Alignment, Border, Color, Element, Length, Theme};
use std::collections::HashSet;

use desktop_assets::assets::Icon;
use desktop_ui_controls::text_editor_scroll_panel::pass_wheel_to_parent;
use session_ui_types as models;
#[derive(Debug, Clone)]
pub enum Message {
    ThinkHover(usize, usize), ThinkHoverLeave, ToggleThink(usize, usize, bool),
    ThinkEditorAction(usize, usize, text_editor::Action),
}
pub struct ThinkBlockState<'a> {
    pub show_reasoning_summary: bool,
    pub expanded: &'a HashSet<u64>,
    pub collapsed: &'a HashSet<u64>,
    pub hovered: Option<u64>,
    pub editors: &'a std::collections::HashMap<u64, text_editor::Content>,
    pub i18n: desktop_i18n::I18n,
    pub now_ms: u64,
    pub animation_frame: usize,
}
fn map_text(message: super::text::Message) -> Message {
    match message {
        super::text::Message::ThinkEditorAction(msg, idx, action) => Message::ThinkEditorAction(msg, idx, action),
        super::text::Message::MessageEditorAction(_, _) => unreachable!("think body only emits think actions"),
    }
}

use super::super::global_status::COMPACT_ACTIVITY_SLOT_SIZE;
use super::super::selectable_text::{SelectableTextPolicy, selectable_text_policy};
use desktop_chat_tools::tool_text_support::{
    READ_ONLY_EDITOR_MAX_HEIGHT, chat_text_font, chat_text_line_height, read_only_text_style,
};
use desktop_chat_tools::card_style::{
    chat_secondary_muted_text_color, chat_secondary_subtle_text_color, icon_svg,
};
use super::styles::{
    THINK_BLOCK_CONTENT_SPACING, THINK_META_TEXT_SIZE, THINK_STATUS_TEXT_SIZE,
    think_block_text_color, thinking_status_text,
};
use super::text_normalize::normalize_display_text;
use super::text::{should_segment_text_block, think_text_body};

pub fn think_block_view_with_streaming<'a>(
    state: ThinkBlockState<'a>,
    msg_idx: usize,
    think_idx: usize,
    content: String,
    open: bool,
    timing: Option<&'a models::ThinkTiming>,
    is_streaming_msg: bool,
) -> Element<'a, Message> {
    let key = ((msg_idx as u64) << 32) | (think_idx as u64);
    let is_thinking = think_block_is_running(open, timing);
    let default_expanded =
        think_block_default_expanded(state.show_reasoning_summary, open, timing);
    let expanded = think_block_resolved_expanded(
        default_expanded,
        key,
        state.expanded,
        state.collapsed,
    );
    let now_ms = state.now_ms;

    let (status_label, duration_label) = if let Some(timing) = timing {
        let end_ms = if is_thinking { now_ms } else { timing.end_ms.unwrap_or(now_ms) };
        let duration_secs = ((end_ms.saturating_sub(timing.start_ms)) as f64 / 1000.0) as u64;
        let status = if is_thinking {
            state.i18n.__("思考中")
        } else {
            state.i18n.__("思考")
        };
        (
            status.to_string(),
            Some(
                state.i18n
                    .format("%u 秒", &[duration_secs.into()])
                    .expect("valid thinking duration template"),
            ),
        )
    } else if is_thinking {
        (state.i18n.__("思考中").to_string(), None)
    } else {
        (state.i18n.__("思考").to_string(), None)
    };

    let is_hovered = state.hovered == Some(key);
    let toggle_icon: Element<'_, Message> = if is_hovered {
        let icon_kind = if expanded { Icon::ChevronDown } else { Icon::ChevronRight };
        icon_svg(icon_kind)
            .width(Length::Fixed(10.0))
            .height(Length::Fixed(10.0))
            .style(move |theme: &Theme, _status| svg::Style {
                color: Some(chat_secondary_muted_text_color(theme)),
            })
            .into()
    } else {
        container(iced::widget::Space::new())
            .width(Length::Fixed(10.0))
            .height(Length::Fixed(10.0))
            .into()
    };

    let status_text: Element<'_, Message> = if is_thinking {
        thinking_status_text(&status_label, now_ms, state.animation_frame).map(map_text)
    } else {
        text(status_label)
            .size(THINK_STATUS_TEXT_SIZE)
            .font(chat_text_font())
            .style(move |theme: &Theme| iced::widget::text::Style {
                color: Some(chat_secondary_subtle_text_color(theme)),
            })
            .into()
    };

    let mut head_row = row![status_text]
        .spacing(5)
        .height(Length::Fixed(COMPACT_ACTIVITY_SLOT_SIZE))
        .align_y(Alignment::Center);

    if let Some(duration) = duration_label {
        head_row = head_row.push(
            text(format!("· {duration}")).size(THINK_META_TEXT_SIZE).font(chat_text_font()).style(
                |theme: &Theme| iced::widget::text::Style {
                    color: Some(chat_secondary_subtle_text_color(theme)),
                },
            ),
        );
    }

    head_row = head_row.push(container(toggle_icon).padding([1, 2]));

    let head = mouse_area(container(head_row).width(Length::Fill).padding([1, 0]))
        .on_enter(Message::ThinkHover(msg_idx, think_idx))
        .on_exit(Message::ThinkHoverLeave);

    let head = head.on_press(Message::ToggleThink(
        msg_idx,
        think_idx,
        default_expanded,
    ));

    let mut think_col = column![head].spacing(THINK_BLOCK_CONTENT_SPACING);

    if expanded {
        let text_content = normalize_display_text(content.trim()).into_owned();
        if !text_content.is_empty() {
            let scroll_key = ((msg_idx as u64) << 32) | (think_idx as u64);
            let use_segmented_body = should_segment_text_block(&text_content);
            let prefer_plain_text =
                super::text::should_prefer_plain_think_body(is_streaming_msg, is_thinking);

            if use_segmented_body || prefer_plain_text {
                think_col = think_col.push(think_text_body(
                    state.editors,
                    msg_idx,
                    think_idx,
                    text_content.clone(),
                    prefer_plain_text,
                ).map(map_text));
            } else if !prefer_plain_text
                && selectable_text_policy(&text_content, false)
                    != SelectableTextPolicy::PreviewWithCopyFallback
            {
                if let Some(editor_content) = state.editors.get(&scroll_key) {
                    let on_action = move |action| {
                        Message::ThinkEditorAction(
                            msg_idx, think_idx, action,
                        )
                    };

                    let editor = text_editor(editor_content)
                        .on_action(on_action)
                        .size(14.0)
                        .line_height(chat_text_line_height())
                        .padding(0)
                        .height(Length::Shrink)
                        .max_height(READ_ONLY_EDITOR_MAX_HEIGHT)
                        .font(chat_text_font())
                        .style(move |theme: &Theme, _status: text_editor::Status| {
                            let value = think_block_text_color(theme);
                            read_only_text_style(theme, value)
                        });

                    let body = container(pass_wheel_to_parent(editor)).width(Length::Fill).style(
                        |_theme: &Theme| iced::widget::container::Style {
                            background: None,
                            border: Border {
                                width: 0.0,
                                color: Color::TRANSPARENT,
                                radius: 10.0.into(),
                            },
                            ..Default::default()
                        },
                    );
                    think_col = think_col.push(body);
                } else {
                    let body = container(
                        text(text_content.clone())
                            .size(14)
                            .font(chat_text_font())
                            .line_height(chat_text_line_height())
                            .style(|theme: &Theme| iced::widget::text::Style {
                                color: Some(think_block_text_color(theme)),
                            }),
                    )
                    .width(Length::Fill)
                    .style(|_theme: &Theme| iced::widget::container::Style {
                        background: None,
                        border: Border {
                            width: 0.0,
                            color: Color::TRANSPARENT,
                            radius: 10.0.into(),
                        },
                        ..Default::default()
                    });
                    think_col = think_col.push(body);
                }
            } else {
                think_col = think_col.push(think_text_body(
                    state.editors,
                    msg_idx,
                    think_idx,
                    text_content.clone(),
                    prefer_plain_text,
                ).map(map_text));
            }
        }
    }

    let think_box = container(think_col);

    think_box.width(Length::Fill).into()
}

pub fn think_block_is_running(open: bool, timing: Option<&models::ThinkTiming>) -> bool {
    open || timing.is_some_and(|think_timing| think_timing.end_ms.is_none())
}

pub fn think_block_default_expanded(
    _show_reasoning_summary: bool,
    _open: bool,
    _timing: Option<&models::ThinkTiming>,
) -> bool {
    false
}

pub fn think_block_resolved_expanded(
    default_expanded: bool,
    key: u64,
    manual_expanded: &HashSet<u64>,
    manual_collapsed: &HashSet<u64>,
) -> bool {
    if manual_collapsed.contains(&key) {
        false
    } else if manual_expanded.contains(&key) {
        true
    } else {
        default_expanded
    }
}

pub fn should_render_think_block(
    show_reasoning_summary: bool,
    total_think_blocks: usize,
    open: bool,
    timing: Option<&models::ThinkTiming>,
) -> bool {
    let _ = total_think_blocks;
    think_block_is_running(open, timing) || show_reasoning_summary
}
