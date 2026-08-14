//! 文件视图中的列表项与按钮构建。

use iced::widget::svg;
use iced::widget::tooltip::{Position as TooltipPosition, Tooltip};
use iced::widget::{Space, button, column, container, mouse_area, row, text};
use iced::{Alignment, Background, Border, Color, Element, Length, Theme};
use std::collections::HashMap;

use crate::app::assets::Icon;
use crate::app::components::chat_panel::utils::{
    bold_font, icon_svg, relative_to_project_root, resolve_path,
};
use crate::app::{App, Message, message};

use super::super::diff_utils::file_preview;
use super::super::file_link::{file_name_link, file_name_only, file_path_tooltip};
use super::super::git_diff_view::render_git_diff_content;
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
            file_name_link(display, abs)
        } else {
            let file_name = file_row_label(app, view_ctx, display, abs);
            let light = matches!(view_ctx.verb, "读取" | "编辑");
            tool_file_left(
                view_ctx.verb,
                file_name_link(&file_name, abs),
                light,
                view_ctx.read_range.clone(),
            )
        };

        let open_row = container(open_content).padding([6, 10]).width(Length::Fill);

        if view_ctx.is_edit_like
            && !render_state.is_search
            && let Some(rel) = relative_to_project_root(app, abs)
        {
            if let Some(change) = changes_by_path.get(rel.as_str()) {
                let title = format!("{}  +{}-{}", rel, change.additions, change.deletions);
                column_view = column_view.push(edit_file_change_block(
                    app,
                    view_ctx.msg_idx,
                    view_ctx.tool_idx,
                    abs,
                    rel,
                    edit_result_label(&view_ctx.tool_name, view_ctx.is_running, view_ctx.is_error),
                    file_row_label(app, view_ctx, display, abs),
                    Some((change.additions, change.deletions)),
                    title,
                    change.before.clone(),
                    change.after.clone(),
                ));
                continue;
            }
        }

        if let Some((title, file, after)) = fallback_edit_diff_payload(app, view_ctx, display, abs)
        {
            column_view = column_view.push(edit_file_change_block(
                app,
                view_ctx.msg_idx,
                view_ctx.tool_idx,
                abs,
                file,
                edit_result_label(&view_ctx.tool_name, view_ctx.is_running, view_ctx.is_error),
                file_row_label(app, view_ctx, display, abs),
                None,
                title,
                String::new(),
                after,
            ));
            continue;
        }

        column_view = column_view.push(open_row);
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
    _app: &App,
    _view_ctx: &super::FilesViewContext<'_>,
    display: &str,
    _abs: &str,
) -> String {
    file_name_only(display)
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

pub(super) fn edit_file_expand_key(msg_idx: usize, tool_idx: usize, path: &str) -> String {
    format!("{msg_idx}:{tool_idx}:{path}")
}

pub(super) fn edit_file_change_block<'a>(
    app: &'a App,
    msg_idx: usize,
    tool_idx: usize,
    abs: &str,
    file: String,
    verb: &'static str,
    file_name: String,
    counts: Option<(usize, usize)>,
    title: String,
    before: String,
    after: String,
) -> Element<'a, Message> {
    let expand_key = edit_file_expand_key(msg_idx, tool_idx, &file);
    let file_expanded = app.chat_tool_file_expanded.contains(&expand_key);
    let hover_key = tool_file_hover_key(msg_idx, tool_idx, abs);
    let open_diff_message = Message::Git(message::GitMessage::OpenChatTextDiff {
        title: title.clone(),
        file: file.clone(),
        before: before.clone(),
        after: after.clone(),
    });
    let toggle_message =
        Message::Chat(message::ChatMessage::ToggleToolFile(msg_idx, tool_idx, file.clone()));

    let mut header = row![
        edit_verb_text(verb),
        edit_file_name_link(abs, file_name),
        external_diff_button(open_diff_message),
        container(Space::new()).width(Length::Fill),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    if let Some((adds, dels)) = counts {
        header = header.push(edit_change_counts(adds, dels));
    }

    header = header.push(view_changes_toggle_button(file_expanded, toggle_message));

    let header = container(header).padding([6, 0]).width(Length::Fill);

    let block: Element<'a, Message> = if file_expanded {
        let diff_body = container(render_git_diff_content(app, title, Some(file), before, after))
            .width(Length::Fill)
            .clip(true)
            .padding([4, 0]);
        column![header, diff_body].spacing(4).into()
    } else {
        header.into()
    };

    mouse_area(container(block).width(Length::Fill))
        .on_enter(Message::Chat(message::ChatMessage::ToolFileHover(hover_key)))
        .on_exit(Message::Chat(message::ChatMessage::ToolFileHoverLeave))
        .into()
}

pub(super) fn edit_file_name_link<'a>(abs: &str, file_name: String) -> Element<'a, Message> {
    file_name_link(&file_name, abs)
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

pub(super) fn file_name_tooltip<'a>(label: String) -> Element<'a, Message> {
    file_path_tooltip(label)
}

pub(super) fn external_diff_button<'a>(on_press: Message) -> Element<'a, Message> {
    let eye =
        icon_svg(Icon::ChevronRight).width(Length::Fixed(12.0)).height(Length::Fixed(12.0)).style(
            |theme: &Theme, _status| svg::Style { color: Some(view_changes_secondary_text(theme)) },
        );

    let button: Element<'a, Message> =
        button(eye).padding([2, 4]).style(edit_file_change_button_style).on_press(on_press).into();

    Tooltip::new(button, file_name_tooltip("查找变更".to_string()), TooltipPosition::Top)
        .gap(6)
        .into()
}

pub(super) fn view_changes_toggle_button<'a>(
    expanded: bool,
    on_press: Message,
) -> Element<'a, Message> {
    button(
        row![
            view_changes_pill(),
            icon_svg(if expanded { Icon::ChevronUp } else { Icon::ChevronDown })
                .width(Length::Fixed(10.0))
                .height(Length::Fixed(10.0))
                .style(|theme: &Theme, _status| svg::Style {
                    color: Some(view_changes_secondary_text(theme)),
                })
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    )
    .padding([2, 4])
    .style(edit_file_change_button_style)
    .on_press(on_press)
    .into()
}

pub(super) fn view_changes_pill<'a>() -> Element<'a, Message> {
    container(text("查看变更").size(14).style(|theme: &Theme| {
        let is_dark = theme.palette().background.r
            + theme.palette().background.g
            + theme.palette().background.b
            < 1.5;
        iced::widget::text::Style {
            color: Some(if is_dark {
                theme.palette().text.scale_alpha(0.88)
            } else {
                theme.extended_palette().secondary.base.text
            }),
        }
    }))
    .padding([2, 8])
    .style(|theme: &Theme| {
        let ext = theme.extended_palette();
        let is_dark = theme.palette().background.r
            + theme.palette().background.g
            + theme.palette().background.b
            < 1.5;
        iced::widget::container::Style {
            background: Some(Background::Color(if is_dark {
                ext.background.weak.color.scale_alpha(0.24)
            } else {
                ext.background.base.color.scale_alpha(0.76)
            })),
            border: Border {
                width: 1.0,
                color: ext.background.strong.color.scale_alpha(if is_dark { 0.48 } else { 0.75 }),
                radius: 999.0.into(),
            },
            ..Default::default()
        }
    })
    .into()
}

pub(super) fn view_changes_secondary_text(theme: &Theme) -> Color {
    let is_dark =
        theme.palette().background.r + theme.palette().background.g + theme.palette().background.b
            < 1.5;
    if is_dark {
        theme.palette().text.scale_alpha(0.72)
    } else {
        theme.extended_palette().secondary.base.text.scale_alpha(0.74)
    }
}

pub(super) fn edit_file_change_button_style(
    theme: &Theme,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let ext = theme.extended_palette();
    let is_dark =
        theme.palette().background.r + theme.palette().background.g + theme.palette().background.b
            < 1.5;
    let background = match status {
        iced::widget::button::Status::Hovered | iced::widget::button::Status::Pressed => {
            Some(Background::Color(if is_dark {
                ext.background.weak.color.scale_alpha(0.28)
            } else {
                ext.background.weak.color.scale_alpha(0.55)
            }))
        }
        _ => None,
    };
    iced::widget::button::Style {
        background,
        text_color: theme.palette().text,
        border: Border { width: 0.0, color: Color::TRANSPARENT, radius: 8.0.into() },
        shadow: Default::default(),
        ..Default::default()
    }
}

pub(super) fn tool_file_hover_key(msg_idx: usize, tool_idx: usize, abs: &str) -> String {
    format!("{msg_idx}:{tool_idx}:{abs}")
}

pub(super) fn tool_file_left<'a>(
    verb: &'static str,
    file_name: Element<'a, Message>,
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

    let mut row_view = row![verb_text, file_name].spacing(8).align_y(Alignment::Center);

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
