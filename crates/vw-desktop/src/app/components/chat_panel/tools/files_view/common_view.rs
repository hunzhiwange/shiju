//! read、glob、grep 等通用文件视图布局。

use iced::widget::{Space, button, container, row, scrollable, text, text_input};
use iced::{Alignment, Background, Border, Color, Element, Length, Padding, Theme};

use crate::app::assets::Icon;
use crate::app::components::chat_panel::utils::{
    chat_context_menu, chat_context_target_key, chat_scroll_direction,
    chat_secondary_muted_text_color, eye_icon_button_style, eye_icon_svg_style, icon_svg,
    simplified_block_style, simplified_code_block_style, truncate_chars, truncate_lines_middle,
};
use crate::app::components::overlays::PointBelowOverlay;
use crate::app::components::widgets::RightClickArea;
use crate::app::{Message, message};

use super::super::{
    ToolPermissionState, ToolTextTarget, tool_header_label, tool_header_title, tool_inline_summary,
    tool_permission_error_text, tool_permission_state, tool_permission_title, tool_summary_text,
    tool_text_editor,
};
use super::{FileListState, FilesViewContext};

pub(super) fn common_tool_meta_text(render_state: &FileListState) -> String {
    if !render_state.filter_query.is_empty() {
        format!("{}/{} 项", render_state.display_count, render_state.total_items)
    } else if render_state.truncated_middle {
        format!(
            "{}项 (省略{}项)",
            render_state.total_items.min(render_state.max_items) + render_state.middle_omitted,
            render_state.middle_omitted
        )
    } else if render_state.tail_omitted > 0 {
        format!("{}项 (省略{}项)", render_state.total_items, render_state.tail_omitted)
    } else {
        format!("{}项", render_state.total_items)
    }
}

pub(super) fn common_tool_title(
    tool_name: &str,
    verb: &str,
    is_running: bool,
    permission_state: Option<ToolPermissionState>,
    is_error: bool,
) -> String {
    if is_running {
        format!("{verb}中")
    } else if let Some(permission_state) = permission_state {
        tool_permission_title(tool_header_label(tool_name).as_str(), permission_state)
    } else if is_error {
        format!("{verb}失败")
    } else {
        tool_header_label(tool_name)
    }
}

pub(super) fn common_tool_fallback_text(
    is_error: bool,
    total_items: usize,
    output: &str,
    permission_error_text: Option<String>,
    error_text: Option<&str>,
) -> Option<String> {
    if is_error {
        permission_error_text
            .or_else(|| error_text.map(str::to_string))
            .map(|text| text.trim().to_string())
            .filter(|text| !text.is_empty())
    } else if total_items == 0 {
        let trimmed = output.trim();
        if trimmed.is_empty() { None } else { Some(trimmed.to_string()) }
    } else {
        None
    }
}

pub(super) fn common_tool_scroll_height(display_count: usize, max_items: usize) -> Length {
    const MAX_GREP_HEIGHT: f32 = 220.0;
    const GREP_LINE_HEIGHT: f32 = 32.0;
    const GREP_VERTICAL_PADDING: f32 = 16.0;

    let estimated_height =
        (display_count.min(max_items) as f32 * GREP_LINE_HEIGHT) + GREP_VERTICAL_PADDING;
    if estimated_height >= MAX_GREP_HEIGHT {
        Length::Fixed(MAX_GREP_HEIGHT)
    } else {
        Length::Shrink
    }
}

pub(super) fn common_tool_show_list_view(
    is_running: bool,
    total_items: usize,
    has_fallback_body: bool,
) -> bool {
    is_running || total_items > 0 || !has_fallback_body
}

pub(super) fn common_tool_context_menu_message(
    target: u64,
    point: iced::Point,
    text: String,
) -> Message {
    Message::Chat(message::ChatMessage::OpenMessageContextMenu {
        target,
        x: point.x,
        y: point.y,
        text,
    })
}

pub(crate) fn build_common_tool_view<'a>(
    view_ctx: &FilesViewContext<'a>,
    render_state: &FileListState,
    list_column: iced::widget::Column<'a, Message>,
) -> Element<'a, Message> {
    let context_key = chat_context_target_key(view_ctx.msg_idx, Some(view_ctx.tool_idx));
    let context_menu_open = view_ctx.app.chat_context_menu_target == Some(context_key);
    let context_menu_anchor = view_ctx.app.chat_context_menu_pos.unwrap_or((12.0, 26.0));
    let selected_context_text =
        super::super::selected_chat_text_for_target(view_ctx.app, context_key);

    let meta = common_tool_meta_text(render_state);

    let meta_view: Element<'a, Message> = text(meta)
        .size(14)
        .style(|theme: &Theme| iced::widget::text::Style {
            color: Some(theme.extended_palette().secondary.base.text.scale_alpha(0.70)),
        })
        .into();

    let tool_value = view_ctx
        .visible
        .split_once('\n')
        .and_then(|(_, rest)| serde_json::from_str::<serde_json::Value>(rest.trim()).ok());
    let permission_state =
        tool_value.as_ref().and_then(|value| tool_permission_state(&view_ctx.tool_name, value));
    let mut summary =
        tool_value.as_ref().and_then(|value| tool_summary_text(value)).unwrap_or_default();
    if summary.is_empty() {
        summary = tool_inline_summary(&view_ctx.tool_name, &view_ctx.input).unwrap_or_default();
    }
    let title = common_tool_title(
        &view_ctx.tool_name,
        view_ctx.verb,
        view_ctx.is_running,
        permission_state,
        view_ctx.is_error,
    );

    let detail_btn = button(
        icon_svg(Icon::ChevronRight)
            .width(Length::Fixed(10.0))
            .height(Length::Fixed(10.0))
            .style(eye_icon_svg_style),
    )
    .padding([2, 4])
    .style(|theme: &Theme, status| eye_icon_button_style(theme, status))
    .on_press(Message::Chat(message::ChatMessage::OpenToolDetail(
        view_ctx.msg_idx,
        view_ctx.tool_idx,
        view_ctx.visible.to_string(),
    )));

    let mut title_row = row![tool_header_title(&view_ctx.tool_name, title, view_ctx.is_error)]
        .spacing(10)
        .align_y(Alignment::Center);
    if !summary.trim().is_empty() {
        title_row = title_row.push(text(summary).size(13).style(|theme: &Theme| {
            iced::widget::text::Style { color: Some(chat_secondary_muted_text_color(theme)) }
        }));
    }

    let head: Element<'a, Message> = container(
        row![title_row, container(Space::new()).width(Length::Fill), meta_view, detail_btn]
            .spacing(10)
            .align_y(Alignment::Center),
    )
    .width(Length::Fill)
    .into();

    let filter_row: Element<'a, Message> = if render_state.is_search {
        let filter_input = text_input("筛选文件...", &view_ctx.app.tool_files_filter)
            .on_input(|value| Message::Chat(message::ChatMessage::ToolFilesFilterChanged(value)))
            .size(12)
            .padding([6, 10])
            .style(|theme: &Theme, status| {
                let ext = theme.extended_palette();
                let background = match status {
                    iced::widget::text_input::Status::Focused { .. } => {
                        ext.background.weak.color.scale_alpha(0.5)
                    }
                    _ => ext.background.weak.color.scale_alpha(0.3),
                };
                iced::widget::text_input::Style {
                    background: Background::Color(background),
                    border: Border { width: 0.0, color: Color::TRANSPARENT, radius: 10.0.into() },
                    icon: Color::TRANSPARENT,
                    placeholder: ext.secondary.base.text.scale_alpha(0.7),
                    value: theme.palette().text,
                    selection: theme.palette().primary.scale_alpha(0.25),
                }
            });
        container(filter_input).width(Length::Fill).padding([0, 2]).into()
    } else {
        Space::new().into()
    };

    let list_view: Element<'a, Message> = if view_ctx.is_running {
        container(text(format!("{}中…", view_ctx.verb)).size(14).style(|theme: &Theme| {
            iced::widget::text::Style {
                color: Some(theme.extended_palette().secondary.base.text.scale_alpha(0.85)),
            }
        }))
        .width(Length::Fill)
        .into()
    } else {
        let scroll_height =
            common_tool_scroll_height(render_state.display_count, render_state.max_items);

        let list_view: Element<'a, Message> = scrollable(
            container(list_column).width(Length::Fill).padding([6, 8]).style(|theme: &Theme| {
                let ext = theme.extended_palette();
                let is_dark = theme.palette().background.r
                    + theme.palette().background.g
                    + theme.palette().background.b
                    < 1.5;
                iced::widget::container::Style {
                    background: Some(Background::Color(if is_dark {
                        ext.background.weak.color.scale_alpha(0.34)
                    } else {
                        ext.background.weak.color.scale_alpha(0.9)
                    })),
                    border: Border {
                        width: 1.0,
                        color: ext.background.strong.color.scale_alpha(if is_dark {
                            0.48
                        } else {
                            0.66
                        }),
                        radius: 10.0.into(),
                    },
                    ..Default::default()
                }
            }),
        )
        .direction(chat_scroll_direction())
        .height(scroll_height)
        .into();

        RightClickArea::new(
            list_view,
            Box::new({
                let context_text = selected_context_text
                    .clone()
                    .unwrap_or_else(|| view_ctx.output.trim().to_string());
                move |point| {
                    common_tool_context_menu_message(context_key, point, context_text.clone())
                }
            }),
        )
        .preserve_on_right_click()
        .into()
    };

    let fallback_text = common_tool_fallback_text(
        view_ctx.is_error,
        render_state.total_items,
        &view_ctx.output,
        tool_value
            .as_ref()
            .and_then(|value| tool_permission_error_text(&view_ctx.tool_name, value)),
        view_ctx.error_text.as_deref(),
    );

    let fallback_body = fallback_text.as_ref().map(|body_text| {
        if view_ctx.is_error {
            let body: Element<'a, Message> = tool_text_editor(
                view_ctx.app,
                ToolTextTarget::ToolCardText {
                    msg_idx: view_ctx.msg_idx,
                    tool_idx: view_ctx.tool_idx,
                    text_idx: 0,
                },
                "Noto Sans CJK SC",
                14.0,
                false,
                true,
            )
            .unwrap_or_else(|| {
                container(text(truncate_chars(body_text, 200)).size(14).style(|theme: &Theme| {
                    iced::widget::text::Style {
                        color: Some(theme.extended_palette().danger.base.color.scale_alpha(0.95)),
                    }
                }))
                .width(Length::Fill)
                .into()
            });
            container(body)
                .padding([10, 12])
                .width(Length::Fill)
                .style(|theme: &Theme| {
                    let ext = theme.extended_palette();
                    iced::widget::container::Style {
                        background: Some(Background::Color(
                            ext.danger.base.color.scale_alpha(0.07),
                        )),
                        border: Border {
                            width: 1.0,
                            color: ext.danger.base.color.scale_alpha(0.30),
                            radius: 14.0.into(),
                        },
                        ..Default::default()
                    }
                })
                .into()
        } else {
            let preview = truncate_lines_middle(body_text, 80, 400);
            scrollable(
                container(
                    tool_text_editor(
                        view_ctx.app,
                        ToolTextTarget::ToolCardText {
                            msg_idx: view_ctx.msg_idx,
                            tool_idx: view_ctx.tool_idx,
                            text_idx: 0,
                        },
                        "JetBrains Mono",
                        14.0,
                        false,
                        false,
                    )
                    .unwrap_or_else(|| {
                        text(preview).size(14).font(iced::Font::with_name("JetBrains Mono")).into()
                    }),
                )
                .width(Length::Fill)
                .padding([8, 10])
                .style(simplified_code_block_style),
            )
            .direction(chat_scroll_direction())
            .height(Length::Fixed(180.0))
            .into()
        }
    });

    let show_list_view = common_tool_show_list_view(
        view_ctx.is_running,
        render_state.total_items,
        fallback_body.is_some(),
    );

    let list_view: Element<'a, Message> = if let Some(menu) = chat_context_menu(context_menu_open) {
        PointBelowOverlay::new(list_view, menu)
            .show(true)
            .anchor(iced::Point::new(context_menu_anchor.0, context_menu_anchor.1))
            .gap(0.0)
            .on_close(Message::Chat(message::ChatMessage::CloseMessageContextMenu))
            .into()
    } else {
        list_view
    };

    let mut content = iced::widget::column![head, filter_row].spacing(8);
    if let Some(fallback_body) = fallback_body {
        let wrapped: Element<'a, Message> = RightClickArea::new(
            fallback_body,
            Box::new({
                let context_text = selected_context_text
                    .clone()
                    .unwrap_or_else(|| view_ctx.output.trim().to_string());
                move |point| {
                    common_tool_context_menu_message(context_key, point, context_text.clone())
                }
            }),
        )
        .preserve_on_right_click()
        .into();
        content = content.push(wrapped);
    }
    if show_list_view {
        content = content.push(list_view);
    }

    container(content)
        .padding(Padding { top: 2.0, right: 6.0, bottom: 2.0, left: 0.0 })
        .width(Length::Fill)
        .style(simplified_block_style)
        .into()
}
