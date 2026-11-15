//! read、glob、grep 等通用文件视图布局。

use iced::widget::{Space, button, container, mouse_area, scrollable, text, text_input};
use iced::{Background, Border, Color, Element, Length, Padding, Theme};

use desktop_assets::assets::Icon;
use crate::card_style::{
    chat_context_target_key, chat_scroll_direction, eye_icon_button_style, eye_icon_svg_style,
    icon_svg, simplified_block_style, simplified_code_block_style, truncate_chars,
    truncate_lines_middle,
};
use desktop_ui_controls::right_click_area::RightClickArea;
use crate::card_input::Message;

use super::super::tool_meta::{tool_activity_header, tool_visual_state};
use super::super::tool_permission::{ToolPermissionState, tool_permission_error_text, tool_permission_state};
use super::super::tool_meta::tool_header_label;
use super::super::tool_parse::{tool_status, tool_summary_text};
use crate::tool_text_support::{ToolTextTarget, tool_text_editor};
use super::{FileListState, FilesViewContext};

pub(super) fn common_tool_meta_text(
    i18n: desktop_i18n::I18n,
    render_state: &FileListState,
) -> String {
    if !render_state.filter_query.is_empty() {
        i18n.format(
            "%u/%u 项",
            &[render_state.display_count.into(), render_state.total_items.into()],
        )
    } else if render_state.truncated_middle {
        i18n.format(
            "%u项 (省略%u项)",
            &[
                (render_state.total_items.min(render_state.max_items)
                    + render_state.middle_omitted)
                    .into(),
                render_state.middle_omitted.into(),
            ],
        )
    } else if render_state.tail_omitted > 0 {
        i18n.format(
            "%u项 (省略%u项)",
            &[render_state.total_items.into(), render_state.tail_omitted.into()],
        )
    } else {
        i18n.format("%u项", &[render_state.total_items.into()])
    }
    .expect("valid file metadata template")
}

pub(super) fn common_tool_title(
    i18n: desktop_i18n::I18n,
    tool_name: &str,
    verb: &str,
    is_running: bool,
    permission_state: Option<ToolPermissionState>,
    is_error: bool,
) -> String {
    if is_running {
        i18n.format("%s中", &[i18n.__(verb).into()]).expect("valid file activity template")
    } else if let Some(permission_state) = permission_state {
        crate::tools::localized_tool_permission_title(
            i18n,
            tool_header_label(tool_name).as_str(),
            permission_state,
        )
    } else if is_error {
        i18n.format("%s失败", &[i18n.__(verb).into()]).expect("valid file failure template")
    } else {
        i18n.__(&tool_header_label(tool_name)).to_string()
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

pub(super) fn common_tool_context_menu_message<Host: Clone>(
    target: u64,
    point: iced::Point,
    text: String,
) -> Message<Host> {
    Message::OpenMessageContextMenu {
        target,
        x: point.x,
        y: point.y,
        text,
    }
}

pub(super) fn build_common_tool_view<'a, Host: Clone + 'a>(
    view_ctx: &FilesViewContext<'a>,
    render_state: &FileListState,
    list_column: iced::widget::Column<'a, Message<Host>>,
) -> Element<'a, Message<Host>> {
    let context_key = chat_context_target_key(view_ctx.msg_idx, Some(view_ctx.tool_idx));
    let selected_context_text =
        crate::tool_text_support::selected_chat_text_for_target(&view_ctx.app.text, context_key);

    let meta = common_tool_meta_text(
        view_ctx.app.i18n,
        render_state,
    );

    let tool_value = view_ctx
        .visible
        .split_once('\n')
        .and_then(|(_, rest)| serde_json::from_str::<serde_json::Value>(rest.trim()).ok());
    let permission_state =
        tool_value.as_ref().and_then(|value| tool_permission_state(&view_ctx.tool_name, value));
    let visual = tool_value
        .as_ref()
        .map(|value| tool_visual_state(tool_status(value)))
        .unwrap_or_else(|| tool_visual_state(""));
    let mut summary =
        tool_value.as_ref().and_then(|value| tool_summary_text(value)).unwrap_or_default();
    if summary.is_empty() {
        summary = crate::tools::localized_tool_inline_summary(
            view_ctx.app.i18n,
            &view_ctx.tool_name,
            &view_ctx.input,
        )
        .unwrap_or_default();
    }
    let title = common_tool_title(
        view_ctx.app.i18n,
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
    .on_press(Message::OpenToolDetail(
        view_ctx.msg_idx,
        view_ctx.tool_idx,
        view_ctx.visible.to_string(),
    ));

    let key = ((view_ctx.msg_idx as u64) << 32) | view_ctx.tool_idx as u64;
    let detail_slot: Element<'a, Message<Host>> = if view_ctx.app.hovered == Some(key) {
        detail_btn.into()
    } else {
        Space::new().width(Length::Fixed(22.0)).into()
    };
    let header_summary =
        if summary.trim().is_empty() { meta } else { format!("{summary} · {meta}") };
    let head =
        tool_activity_header(&view_ctx.tool_name, title, header_summary, visual, detail_slot);

    let filter_row: Element<'a, Message<Host>> = if render_state.is_search {
        let filter_input = text_input(
            view_ctx.app.i18n.__("筛选文件..."),
            &view_ctx.app.filter,
        )
        .on_input(|value| Message::ToolFilesFilterChanged(value))
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

    let list_view: Element<'a, Message<Host>> = if view_ctx.is_running {
        container(
            text(
                view_ctx.app.i18n
                    .format(
                        "%s中…",
                        &[view_ctx.app.i18n
                            .__(view_ctx.verb)
                            .into()],
                    )
                    .expect("valid file progress template"),
            )
            .size(14)
            .style(|theme: &Theme| iced::widget::text::Style {
                color: Some(theme.extended_palette().secondary.base.text.scale_alpha(0.85)),
            }),
        )
        .width(Length::Fill)
        .into()
    } else {
        let scroll_height =
            common_tool_scroll_height(render_state.display_count, render_state.max_items);

        let list_view: Element<'a, Message<Host>> = scrollable(
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
        .use_global_position()
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
            let body: Element<'a, Message<Host>> = tool_text_editor(
                &view_ctx.app.text,
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
            .map(|e| e.map(Message::Text))
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
                        &view_ctx.app.text,
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
                    .map(|e| e.map(Message::Text))
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

    let mut content = iced::widget::column![head, filter_row].spacing(8);
    if let Some(fallback_body) = fallback_body {
        let wrapped: Element<'a, Message<Host>> = RightClickArea::new(
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
        .use_global_position()
        .preserve_on_right_click()
        .into();
        content = content.push(wrapped);
    }
    if show_list_view {
        content = content.push(list_view);
    }

    mouse_area(
        container(content)
            .padding(Padding { top: 2.0, right: 6.0, bottom: 2.0, left: 0.0 })
            .width(Length::Fill)
            .style(simplified_block_style),
    )
    .on_enter(Message::ToolHover(view_ctx.msg_idx, view_ctx.tool_idx))
    .on_exit(Message::ToolHoverLeave)
    .into()
}
