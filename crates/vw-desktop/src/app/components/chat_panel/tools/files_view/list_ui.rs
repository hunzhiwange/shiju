//! 文件视图中的列表项与按钮构建。

use std::collections::HashMap;
use std::path::Path;

use iced::widget::tooltip::{Position as TooltipPosition, Tooltip};
use iced::widget::{Space, button, column, container, mouse_area, row, text};
use iced::{Alignment, Background, Border, Color, Element, Length, Theme};

use crate::app::assets::Icon;
use crate::app::components::chat_panel::utils::{
    bold_font, icon_svg, relative_to_project_root, resolve_path, truncate_chars,
    weak_file_button_style,
};
use crate::app::{App, Message, message};

use super::super::diff_utils::file_preview;
use super::super::tool_parse::tool_input_path;
use super::super::types::ChangeFile;

pub(crate) fn build_file_list_column<'a>(
    app: &'a App,
    render_state: &super::FileListState,
    view_ctx: &super::FilesViewContext<'a>,
    changes_by_path: &HashMap<String, ChangeFile>,
) -> iced::widget::Column<'a, Message> {
    let mut column_view = column![].spacing(6);

    for (index, (display, abs)) in render_state.items_for_display.iter().enumerate() {
        if render_state.truncated_middle && index == (render_state.max_items / 2) {
            column_view = column_view.push(
                container(text("…").size(14).style(|theme: &Theme| iced::widget::text::Style {
                    color: Some(theme.extended_palette().secondary.base.text.scale_alpha(0.9)),
                }))
                .padding([2, 6])
                .width(Length::Fill),
            );
        }

        let open_content: Element<'a, Message> = if render_state.is_search {
            let label = truncate_chars(display, 80);
            row![text(label).size(14).style(|theme: &Theme| iced::widget::text::Style {
                color: Some(theme.extended_palette().secondary.base.text.scale_alpha(0.85)),
            })]
            .into()
        } else {
            let file_name = file_row_label(app, view_ctx, display, abs);
            let light = matches!(view_ctx.verb, "读取" | "编辑");
            tool_file_left(view_ctx.verb, file_name, light, view_ctx.read_range.clone())
        };

        let open_button = button(open_content)
            .padding([6, 10])
            .width(Length::Fill)
            .style(weak_file_button_style)
            .on_press(Message::Preview(message::PreviewMessage::Open(abs.clone())));

        if view_ctx.is_edit_like
            && !render_state.is_search
            && let Some(rel) = relative_to_project_root(app, abs)
        {
            if let Some(change) = changes_by_path.get(rel.as_str()) {
                let hover_key = tool_file_hover_key(view_ctx.msg_idx, view_ctx.tool_idx, abs);
                let is_hovered = app.chat_tool_file_hovered.as_deref() == Some(hover_key.as_str());
                let title = format!("{}  +{}-{}", rel, change.additions, change.deletions);
                let change_message = Message::Git(message::GitMessage::OpenChatTextDiff {
                    title,
                    file: rel.clone(),
                    before: change.before.clone(),
                    after: change.after.clone(),
                });
                let file_name = file_row_label(app, view_ctx, display, abs);
                let row_content = edit_file_row_content(
                    view_ctx.msg_idx,
                    view_ctx.tool_idx,
                    abs,
                    edit_result_label(&view_ctx.tool_name, view_ctx.is_running, view_ctx.is_error),
                    file_name,
                    Message::Preview(message::PreviewMessage::Open(abs.clone())),
                    is_hovered,
                    Some((change.additions, change.deletions)),
                    change_message,
                );

                column_view = column_view.push(row_content);
                continue;
            }
        }

        if let Some((title, file, after)) = fallback_edit_diff_payload(app, view_ctx, display, abs)
        {
            let hover_key = tool_file_hover_key(view_ctx.msg_idx, view_ctx.tool_idx, abs);
            let is_hovered = app.chat_tool_file_hovered.as_deref() == Some(hover_key.as_str());
            let change_message = Message::Git(message::GitMessage::OpenChatTextDiff {
                title,
                file,
                before: String::new(),
                after,
            });
            let row_content = edit_file_row_content(
                view_ctx.msg_idx,
                view_ctx.tool_idx,
                abs,
                edit_result_label(&view_ctx.tool_name, view_ctx.is_running, view_ctx.is_error),
                file_row_label(app, view_ctx, display, abs),
                Message::Preview(message::PreviewMessage::Open(abs.clone())),
                is_hovered,
                None,
                change_message,
            );

            column_view = column_view.push(row_content);
            continue;
        }

        column_view = column_view.push(open_button);
    }

    if render_state.tail_omitted > 0 {
        column_view = column_view.push(
            container(text(format!("… 省略{}条", render_state.tail_omitted)).size(14).style(
                |theme: &Theme| iced::widget::text::Style {
                    color: Some(theme.extended_palette().secondary.base.text.scale_alpha(0.8)),
                },
            ))
            .padding([2, 6])
            .width(Length::Fill),
        );
    }

    if !render_state.filter_query.is_empty() && render_state.is_empty_filtered {
        column_view = column_view.push(
            container(text("没有匹配的文件").size(14).style(|theme: &Theme| {
                iced::widget::text::Style {
                    color: Some(theme.extended_palette().secondary.base.text.scale_alpha(0.65)),
                }
            }))
            .padding([2, 6])
            .width(Length::Fill),
        );
    } else if render_state.total_items == 0 {
        column_view = column_view.push(
            container(
                text(if render_state.is_search {
                    "未返回可定位的文件结果"
                } else {
                    "没有可展示的文件"
                })
                .size(14)
                .style(|theme: &Theme| iced::widget::text::Style {
                    color: Some(theme.extended_palette().secondary.base.text.scale_alpha(0.65)),
                }),
            )
            .padding([2, 6])
            .width(Length::Fill),
        );
    }

    column_view
}

pub(super) fn file_row_label(
    app: &App,
    view_ctx: &super::FilesViewContext<'_>,
    display: &str,
    abs: &str,
) -> String {
    if view_ctx.is_edit_like {
        return relative_to_project_root(app, abs).unwrap_or_else(|| display.to_string());
    }

    Path::new(display).file_name().and_then(|value| value.to_str()).unwrap_or(display).to_string()
}

pub(super) fn fallback_edit_diff_payload(
    app: &App,
    view_ctx: &super::FilesViewContext<'_>,
    display: &str,
    abs: &str,
) -> Option<(String, String, String)> {
    if !view_ctx.is_edit_like {
        return None;
    }

    let input_path = tool_input_path(&view_ctx.input).and_then(|path| resolve_path(app, &path))?;
    if input_path != abs {
        return None;
    }

    let after = file_preview(&view_ctx.tool_name, &view_ctx.input, &view_ctx.output)?;
    let after = after.trim().to_string();
    if after.is_empty() {
        return None;
    }

    let file = relative_to_project_root(app, abs).unwrap_or_else(|| display.to_string());
    let title = format!("{}  写入内容", file);
    Some((title, file, after))
}

pub(super) fn edit_file_row_content<'a>(
    msg_idx: usize,
    tool_idx: usize,
    abs: &str,
    verb: &'static str,
    file_name: String,
    open_message: Message,
    is_hovered: bool,
    counts: Option<(usize, usize)>,
    diff_message: Message,
) -> Element<'a, Message> {
    let mut content = row![
        edit_verb_text(verb),
        edit_file_name_link(msg_idx, tool_idx, abs, file_name, open_message, is_hovered),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    if let Some((adds, dels)) = counts {
        content = content.push(edit_change_counts(adds, dels));
    }

    content = content.push(diff_eye_button_slot(is_hovered, diff_message));
    let hover_key = tool_file_hover_key(msg_idx, tool_idx, abs);

    mouse_area(container(content).width(Length::Fill))
        .on_enter(Message::Chat(message::ChatMessage::ToolFileHover(hover_key)))
        .on_exit(Message::Chat(message::ChatMessage::ToolFileHoverLeave))
        .into()
}

pub(super) fn edit_file_name_link<'a>(
    msg_idx: usize,
    tool_idx: usize,
    abs: &str,
    file_name: String,
    open_message: Message,
    is_hovered: bool,
) -> Element<'a, Message> {
    let hover_key = tool_file_hover_key(msg_idx, tool_idx, abs);
    let label = file_name.clone();
    let file_text = text(file_name).size(14).style(move |theme: &Theme| {
        iced::widget::text::Style { color: Some(edit_file_name_color(theme, is_hovered)) }
    });

    let link: Element<'a, Message> = mouse_area(file_text)
        .on_enter(Message::Chat(message::ChatMessage::ToolFileHover(hover_key)))
        .on_press(open_message)
        .into();

    Tooltip::new(link, file_name_tooltip(label), TooltipPosition::Top).gap(6).into()
}

pub(super) fn edit_verb_text<'a>(verb: &'static str) -> Element<'a, Message> {
    text(verb)
        .size(14)
        .style(move |theme: &Theme| {
            let is_dark = theme.palette().background.r
                + theme.palette().background.g
                + theme.palette().background.b
                < 1.5;
            iced::widget::text::Style {
                color: Some(if is_dark {
                    theme.palette().text.scale_alpha(0.76)
                } else {
                    theme.extended_palette().secondary.base.text.scale_alpha(0.65)
                }),
            }
        })
        .into()
}

pub(super) fn edit_file_name_color(theme: &Theme, is_hovered: bool) -> Color {
    let base = theme.extended_palette().primary.base.color;
    if is_hovered { base } else { base.scale_alpha(0.86) }
}

pub(super) fn file_name_tooltip<'a>(label: String) -> Element<'a, Message> {
    container(text(label).size(12))
        .padding([6, 8])
        .style(|_theme: &Theme| iced::widget::container::Style {
            text_color: Some(Color::WHITE),
            background: Some(Background::Color(Color::from_rgba8(0, 0, 0, 0.94))),
            border: Border { radius: 6.0.into(), ..Default::default() },
            ..Default::default()
        })
        .into()
}

pub(super) fn diff_eye_button<'a>(on_press: Message) -> Element<'a, Message> {
    let eye = icon_svg(Icon::ChevronRight)
        .width(Length::Fixed(12.0))
        .height(Length::Fixed(12.0))
        .style(|theme: &Theme, _status| {
            let is_dark = theme.palette().background.r
                + theme.palette().background.g
                + theme.palette().background.b
                < 1.5;
            iced::widget::svg::Style {
                color: Some(if is_dark {
                    theme.palette().text.scale_alpha(0.9)
                } else {
                    theme.extended_palette().secondary.base.text
                }),
            }
        });

    let button: Element<'a, Message> = mouse_area(
        container(eye).style(|_theme: &Theme| iced::widget::container::Style::default()),
    )
    .on_press(on_press)
    .into();

    Tooltip::new(button, file_name_tooltip("查找变更".to_string()), TooltipPosition::Top)
        .gap(6)
        .into()
}

pub(super) fn diff_eye_button_slot<'a>(
    is_hovered: bool,
    on_press: Message,
) -> Element<'a, Message> {
    if is_hovered {
        diff_eye_button(on_press)
    } else {
        Space::new().width(Length::Fixed(12.0)).into()
    }
}

pub(super) fn tool_file_hover_key(msg_idx: usize, tool_idx: usize, abs: &str) -> String {
    format!("{msg_idx}:{tool_idx}:{abs}")
}

pub(super) fn tool_file_left<'a>(
    verb: &'static str,
    file_name: String,
    light: bool,
    read_range: Option<String>,
) -> Element<'a, Message> {
    let verb_text = text(verb).size(14).style(move |theme: &Theme| {
        let is_dark = theme.palette().background.r
            + theme.palette().background.g
            + theme.palette().background.b
            < 1.5;
        iced::widget::text::Style {
            color: Some(if light {
                if is_dark {
                    theme.palette().text.scale_alpha(0.76)
                } else {
                    theme.extended_palette().secondary.base.text.scale_alpha(0.65)
                }
            } else {
                theme.extended_palette().secondary.base.text
            }),
        }
    });

    let file_text = text(file_name).size(14).style(move |theme: &Theme| {
        let is_dark = theme.palette().background.r
            + theme.palette().background.g
            + theme.palette().background.b
            < 1.5;
        iced::widget::text::Style {
            color: Some(if light {
                if is_dark {
                    theme.extended_palette().primary.base.color.scale_alpha(0.95)
                } else {
                    theme.extended_palette().primary.base.color.scale_alpha(0.86)
                }
            } else {
                theme.palette().text
            }),
        }
    });

    let mut row_view = row![verb_text, file_text].spacing(8).align_y(Alignment::Center);

    if let Some(range) = read_range {
        row_view = row_view.push(container(Space::new()).width(Length::Fill)).push(
            text(range).size(14).style(|theme: &Theme| {
                let is_dark = theme.palette().background.r
                    + theme.palette().background.g
                    + theme.palette().background.b
                    < 1.5;
                iced::widget::text::Style {
                    color: Some(if is_dark {
                        theme.palette().text.scale_alpha(0.72)
                    } else {
                        theme.extended_palette().secondary.base.text.scale_alpha(0.65)
                    }),
                }
            }),
        );
    }

    row_view.into()
}

pub(super) fn edit_result_label(tool_name: &str, is_running: bool, is_error: bool) -> &'static str {
    if matches!(tool_name, "write" | "file_write") {
        if is_running {
            "写入中"
        } else if is_error {
            "写入失败"
        } else {
            "已写入"
        }
    } else if is_running {
        "编辑中"
    } else if is_error {
        "编辑失败"
    } else {
        "已编辑"
    }
}

pub(super) fn edit_change_counts<'a>(adds: usize, dels: usize) -> Element<'a, Message> {
    row![
        text(format!("+{}", adds)).size(14).font(bold_font()).style(|theme: &Theme| {
            iced::widget::text::Style { color: Some(theme.extended_palette().success.base.color) }
        }),
        text(format!("-{}", dels)).size(14).font(bold_font()).style(|theme: &Theme| {
            iced::widget::text::Style { color: Some(theme.extended_palette().danger.base.color) }
        })
    ]
    .spacing(4)
    .align_y(Alignment::Center)
    .into()
}
