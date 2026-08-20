//! 渲染 apply_patch 工具结果。
//! 视图将补丁摘要、文件列表和差异预览组合呈现，帮助用户审查自动修改。

use iced::widget::svg;
use iced::widget::tooltip::{Position as TooltipPosition, Tooltip};
use iced::widget::{Space, button, column, container, mouse_area, row, scrollable, text};
use iced::{Alignment, Background, Border, Color, Element, Length, Theme};
use std::path::Path;

use crate::app::assets::{self, Icon};
use crate::app::components::widgets::RightClickArea;
use crate::app::{App, Message, message};

#[cfg(not(target_arch = "wasm32"))]
use super::apply_patch_preview::parse_unified_diff_change_files;
use super::apply_patch_preview::{collect_apply_patch_changes, find_apply_patch_change};
use super::changes::parse_changes_file_summaries;
use super::diff_utils::{
    count_apply_patch_format_changes, count_unified_diff_changes, extract_diff_block,
    parse_apply_patch_line_changes, parse_apply_patch_summary,
};
use super::file_link::{file_name_link, file_name_only, file_path_tooltip};
use super::git_diff_view::render_git_diff_content;
use super::tool_meta::{tool_activity_header, tool_visual_state};
use super::tool_parse::{
    resolve_output_path, tool_change_file_summaries, tool_change_files, tool_error_text,
    tool_input, tool_output_path, tool_output_text, tool_status, tool_structured_diff_text,
};
use super::types::ChangeFile;
use super::{
    ToolTextTarget, canonical_tool_name, selected_chat_text_for_target, tool_permission_error_text,
    tool_permission_state, tool_permission_target_summary, tool_permission_title, tool_text_editor,
};
use crate::app::components::chat_panel::utils::{
    bold_font, change_pills, chat_context_target_key, chat_scroll_direction, deletions_pill,
    eye_icon_button_style, eye_icon_svg_style, file_button_style, icon_svg, resolve_path,
    simplified_block_style, simplified_code_block_style, truncate_chars, truncate_lines_middle,
};

/// 执行 tool_apply_patch_view 对应的模块功能。
///
/// 参数含义保持与函数签名一致；返回值用于调用方继续组合处理，错误由返回类型显式表达。
pub fn tool_apply_patch_view<'a>(
    app: &'a App,
    msg_idx: usize,
    tool_idx: usize,
    visible: &str,
) -> Option<Element<'a, Message>> {
    let (first, rest) = visible.split_once('\n')?;
    let tool_name = canonical_tool_name(first.trim().strip_prefix("tool ")?.trim());
    if tool_name != "apply_patch" {
        return None;
    }
    let v = serde_json::from_str::<serde_json::Value>(rest.trim()).ok()?;
    let status = tool_status(&v);
    let visual = tool_visual_state(status);
    let is_error = matches!(status, "error" | "failed" | "denied");
    let is_running = status == "running";

    let input = tool_input(&v);
    let output = tool_output_text(&v).unwrap_or_default();
    let expanded = true;
    let structured_summaries = tool_change_file_summaries(&v);
    let mut files = if structured_summaries.is_empty() {
        parse_apply_patch_summary(&output)
    } else {
        structured_summaries.iter().map(|c| (c.kind, c.path.clone())).collect()
    };
    let diff = tool_structured_diff_text(&v).or_else(|| extract_diff_block(&output));
    let change_summaries = if structured_summaries.is_empty() {
        parse_changes_file_summaries(&output)
    } else {
        structured_summaries
    };
    let summaries_by_path = change_summaries
        .iter()
        .map(|c| (c.path.clone(), c.clone()))
        .collect::<std::collections::HashMap<String, super::types::ChangeFileSummary>>(
    );
    if !summaries_by_path.is_empty() {
        files.retain(|(_, path)| summaries_by_path.contains_key(path));
        if files.is_empty() {
            files = change_summaries.iter().map(|c| (c.kind, c.path.clone())).collect();
        }
    }
    let changes = if !summaries_by_path.is_empty() {
        let structured_changes = tool_change_files(&v);
        if structured_changes.is_empty() {
            collect_apply_patch_changes(&output, input)
        } else {
            structured_changes
        }
    } else {
        Vec::new()
    };
    let output_path = tool_output_path(&v);

    let (adds, mods, dels) = files.iter().fold((0usize, 0usize, 0usize), |acc, (k, _)| match k {
        'A' => (acc.0 + 1, acc.1, acc.2),
        'M' => (acc.0, acc.1 + 1, acc.2),
        'D' => (acc.0, acc.1, acc.2 + 1),
        _ => acc,
    });
    let (mut line_adds, mut line_dels) = parse_apply_patch_line_changes(&output);
    if line_adds == 0 && line_dels == 0 {
        if let Some(d) = diff.as_deref() {
            (line_adds, line_dels) = count_unified_diff_changes(d);
        } else if diff_utils_looks_like_unified_diff(input) {
            (line_adds, line_dels) = count_unified_diff_changes(input);
        } else if input.contains("*** Begin Patch") {
            (line_adds, line_dels) = count_apply_patch_format_changes(input);
        }
    }

    let key = ((msg_idx as u64) << 32) | (tool_idx as u64);
    let _is_hovered = app.chat_tool_hovered_idx == Some(key);
    let context_key = chat_context_target_key(msg_idx, Some(tool_idx));
    let selected_context_text = selected_chat_text_for_target(app, context_key);
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

    let (primary_name, _primary_dir, trailing_summary) = apply_patch_header_summary(&files);
    let permission_target = tool_permission_target_summary(tool_name, &v);
    let title = if let Some(permission_state) = tool_permission_state(tool_name, &v) {
        tool_permission_title("补丁", permission_state)
    } else if is_error {
        "补丁失败".to_string()
    } else if is_running {
        "应用补丁中".to_string()
    } else {
        "补丁".to_string()
    };
    let header_icon = apply_patch_header_file_icon(
        files.first().map(|(_, path)| path.as_str()).unwrap_or(primary_name.as_str()),
    );

    let mut header_summary = if primary_name.is_empty() {
        permission_target.unwrap_or_default()
    } else {
        primary_name
    };
    if let Some(extra) = trailing_summary {
        if !header_summary.is_empty() {
            header_summary.push_str(" · ");
        }
        header_summary.push_str(&extra);
    }

    let mut actions = row![container(header_file_type_icon(header_icon)).width(Length::Fixed(16.0))]
        .spacing(6)
        .align_y(Alignment::Center);

    if line_adds + line_dels > 0 {
        actions = actions.push(apply_patch_change_totals(line_adds, line_dels));
    } else if adds + mods + dels > 0 {
        actions = actions.push(text(format!("+{}  ~{}  -{}", adds, mods, dels)).size(14).style(
            |theme: &Theme| iced::widget::text::Style {
                color: Some(apply_patch_secondary_text(theme, 0.64, 0.72)),
            },
        ));
    }

    actions = actions.push(detail_btn);
    let head_row = tool_activity_header(
        "apply_patch",
        title,
        header_summary,
        visual,
        actions.into(),
    );

    let head_with_hover = mouse_area(container(head_row).width(Length::Fill).padding([1, 0]))
        .on_enter(Message::Chat(message::ChatMessage::ToolHover(msg_idx, tool_idx)))
        .on_exit(Message::Chat(message::ChatMessage::ToolHoverLeave));

    let mut content = column![head_with_hover].spacing(10);
    if is_running {
        content = content.push(
            container(text("应用补丁中…").size(14).style(|theme: &Theme| {
                iced::widget::text::Style {
                    color: Some(theme.extended_palette().secondary.base.text.scale_alpha(0.85)),
                }
            }))
            .width(Length::Fill),
        );
        return Some(
            container(content)
                .padding([2, 0])
                .width(Length::Fill)
                .style(simplified_block_style)
                .into(),
        );
    }

    if !is_error || expanded {
        if is_error {
            let err_full = tool_permission_error_text(tool_name, &v)
                .or_else(|| tool_error_text(&v))
                .unwrap_or_default();
            if !err_full.trim().is_empty() {
                let err_view: Element<'a, Message> = tool_text_editor(
                    app,
                    ToolTextTarget::ToolCardText { msg_idx, tool_idx, text_idx: 0 },
                    "Noto Sans CJK SC",
                    14.0,
                    false,
                    true,
                )
                .unwrap_or_else(|| {
                    let short = truncate_chars(err_full.trim(), 160);
                    container(text(short).size(14).style(|theme: &Theme| {
                        iced::widget::text::Style {
                            color: Some(
                                theme.extended_palette().danger.base.color.scale_alpha(0.9),
                            ),
                        }
                    }))
                    .width(Length::Fill)
                    .into()
                });
                let err_view: Element<'a, Message> = RightClickArea::new(
                    err_view,
                    Box::new({
                        let text = selected_context_text
                            .clone()
                            .unwrap_or_else(|| err_full.trim().to_string());
                        move |point| {
                            Message::Chat(message::ChatMessage::OpenMessageContextMenu {
                                target: context_key,
                                x: point.x,
                                y: point.y,
                                text: text.clone(),
                            })
                        }
                    }),
                )
                .use_global_position()
                .preserve_on_right_click()
                .into();
                content = content.push(err_view);
            }
        }

        if !files.is_empty() {
            let mut list = column![].spacing(6);
            for (kind, path) in files.iter() {
                let icon = match kind {
                    'D' => Icon::Trash,
                    _ => Icon::File,
                };
                let abs_for_preview = if *kind != 'D' { resolve_path(app, path) } else { None };
                let full_path = resolve_path(app, path).unwrap_or_else(|| path.clone());
                let file_ref: Element<'a, Message> = if let Some(abs) = abs_for_preview.as_deref() {
                    file_name_link(path, abs)
                } else {
                    Tooltip::new(
                        text(file_name_only(path)).size(14).style(|theme: &Theme| {
                            iced::widget::text::Style {
                                color: Some(theme.palette().text.scale_alpha(0.92)),
                            }
                        }),
                        file_path_tooltip(full_path),
                        TooltipPosition::Top,
                    )
                    .gap(6)
                    .into()
                };
                let eye_btn: Element<'a, Message> = abs_for_preview
                    .as_ref()
                    .cloned()
                    .map(preview_eye_button)
                    .unwrap_or_else(|| Space::new().width(Length::Fixed(20.0)).into());
                let count_meta: Element<'a, Message> = if let Some(c) = summaries_by_path.get(path)
                {
                    if *kind == 'D' {
                        deletions_pill(c.deletions)
                    } else {
                        change_pills(c.additions, c.deletions)
                    }
                } else {
                    Space::new().into()
                };

                if let Some(c) = summaries_by_path.get(path) {
                    let expanded_key = apply_patch_file_expand_key(msg_idx, tool_idx, path);
                    let file_expanded = app.chat_tool_file_expanded.contains(&expanded_key);
                    let title = if *kind == 'D' {
                        format!("{}  -{}", path, c.deletions)
                    } else {
                        format!("{}  +{}-{}", path, c.additions, c.deletions)
                    };
                    let resolved_change =
                        resolve_apply_patch_change_preview(app, &changes, *kind, path);
                    let external_diff_btn: Element<'a, Message> = if let Some(change) =
                        resolved_change.clone()
                    {
                        external_diff_button(Message::Git(message::GitMessage::OpenChatTextDiff {
                            title: title.clone(),
                            file: path.clone(),
                            before: change.before,
                            after: change.after,
                        }))
                    } else {
                        Space::new().width(Length::Fixed(20.0)).into()
                    };
                    let view_changes_btn = view_changes_toggle_button(
                        file_expanded,
                        Message::Chat(message::ChatMessage::ToggleToolFile(
                            msg_idx,
                            tool_idx,
                            path.clone(),
                        )),
                    );
                    let header_row = row![
                        row![apply_patch_file_icon(icon), file_ref]
                            .spacing(10)
                            .align_y(Alignment::Center),
                        eye_btn,
                        external_diff_btn,
                        container(Space::new()).width(Length::Fill),
                        count_meta,
                        view_changes_btn,
                    ]
                    .align_y(Alignment::Center)
                    .spacing(10);
                    let header = container(header_row)
                        .padding([8, 0])
                        .width(Length::Fill)
                        .style(apply_patch_inner_card_style);

                    if file_expanded {
                        let diff_body: Element<'a, Message> = if let Some(change) = resolved_change
                        {
                            container(render_git_diff_content(
                                app,
                                title,
                                Some(path.clone()),
                                change.before,
                                change.after,
                            ))
                            .width(Length::Fill)
                            .clip(true)
                            .padding([4, 0])
                            .into()
                        } else if *kind == 'D' {
                            container(
                                text("文件已删除，但当前记录未包含删除前内容预览").size(14).style(
                                    |theme: &Theme| iced::widget::text::Style {
                                        color: Some(apply_patch_secondary_text(theme, 0.72, 0.74)),
                                    },
                                ),
                            )
                            .width(Length::Fill)
                            .clip(true)
                            .padding([10, 0])
                            .style(apply_patch_inner_card_style)
                            .into()
                        } else {
                            container(text("未能解析文件变更").size(14).style(|theme: &Theme| {
                                iced::widget::text::Style {
                                    color: Some(apply_patch_secondary_text(theme, 0.72, 0.74)),
                                }
                            }))
                            .width(Length::Fill)
                            .clip(true)
                            .padding([10, 0])
                            .style(apply_patch_inner_card_style)
                            .into()
                        };
                        list = list.push(
                            container(column![header, diff_body].spacing(4))
                                .width(Length::Fill)
                                .clip(true)
                                .style(apply_patch_outer_card_style),
                        );
                    } else {
                        list = list.push(header);
                    }
                } else {
                    let open_row = row![
                        row![apply_patch_file_icon(icon), file_ref]
                            .spacing(10)
                            .align_y(Alignment::Center),
                        eye_btn,
                        container(Space::new()).width(Length::Fill),
                        count_meta
                    ]
                    .align_y(Alignment::Center)
                    .spacing(10);
                    list = list.push(
                        container(open_row)
                            .padding([8, 0])
                            .width(Length::Fill)
                            .style(apply_patch_inner_card_style),
                    );
                }
            }
            content = content.push(container(list).width(Length::Fill));
        }
    }

    let has_inline_file_diffs = !summaries_by_path.is_empty();

    if expanded && !has_inline_file_diffs {
        if let Some(op) = output_path.as_ref() {
            let open_path = resolve_output_path(app, op);
            let btn = button(
                row![
                    icon_svg(Icon::ChevronRight).style(|theme: &Theme, _status| {
                        svg::Style { color: Some(apply_patch_secondary_text(theme, 0.92, 0.90)) }
                    }),
                    text("打开完整输出").size(14)
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            )
            .padding([6, 10])
            .style(file_button_style)
            .on_press(Message::Preview(message::PreviewMessage::Open(open_path)));
            content = content.push(container(btn).width(Length::Fill));
        }

        if let Some(d) = diff.as_ref().map(|s| s.trim()).filter(|s| !s.is_empty()) {
            let out = truncate_lines_middle(d, 200, 2000);
            let code = tool_text_editor(
                app,
                ToolTextTarget::ToolCardText { msg_idx, tool_idx, text_idx: 0 },
                "JetBrains Mono",
                14.0,
                false,
                false,
            )
            .unwrap_or_else(|| {
                text(out)
                    .size(14)
                    .font(iced::Font::with_name("JetBrains Mono"))
                    .style(|theme: &Theme| iced::widget::text::Style {
                        color: Some(theme.palette().text),
                    })
                    .into()
            });
            let body: Element<'a, Message> = scrollable(
                container(code)
                    .width(Length::Fill)
                    .padding([6, 8])
                    .style(simplified_code_block_style),
            )
            .direction(chat_scroll_direction())
            .height(Length::Fixed(240.0))
            .into();
            let body: Element<'a, Message> = RightClickArea::new(
                body,
                Box::new({
                    let text = selected_context_text.clone().unwrap_or_else(|| d.to_string());
                    move |point| {
                        Message::Chat(message::ChatMessage::OpenMessageContextMenu {
                            target: context_key,
                            x: point.x,
                            y: point.y,
                            text: text.clone(),
                        })
                    }
                }),
            )
            .use_global_position()
            .preserve_on_right_click()
            .into();
            content = content.push(body);
        }
    }

    let content: Element<'a, Message> =
        container(content).padding([2, 0]).width(Length::Fill).style(simplified_block_style).into();

    Some(content)
}

fn view_changes_pill<'a>() -> Element<'a, Message> {
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

fn view_changes_toggle_button<'a>(expanded: bool, on_press: Message) -> Element<'a, Message> {
    button(
        row![
            view_changes_pill(),
            icon_svg(if expanded { Icon::ChevronUp } else { Icon::ChevronDown })
                .width(Length::Fixed(10.0))
                .height(Length::Fixed(10.0))
                .style(|theme: &Theme, _status| svg::Style {
                    color: Some(apply_patch_secondary_text(theme, 0.72, 0.74)),
                })
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    )
    .padding([2, 4])
    .style(apply_patch_file_button_style)
    .on_press(on_press)
    .into()
}

fn external_diff_button<'a>(on_press: Message) -> Element<'a, Message> {
    let eye = icon_svg(Icon::ChevronRight)
        .width(Length::Fixed(12.0))
        .height(Length::Fixed(12.0))
        .style(|theme: &Theme, _status| {
            let is_dark = theme.palette().background.r
                + theme.palette().background.g
                + theme.palette().background.b
                < 1.5;
            svg::Style {
                color: Some(if is_dark {
                    theme.palette().text.scale_alpha(0.9)
                } else {
                    theme.extended_palette().secondary.base.text
                }),
            }
        });
    let btn = button(eye).padding([2, 4]).style(apply_patch_file_button_style).on_press(on_press);
    Tooltip::new(
        btn,
        container(text("查找变更").size(12)).padding([6, 8]).style(|_theme: &Theme| {
            iced::widget::container::Style {
                text_color: Some(Color::WHITE),
                background: Some(Background::Color(Color::from_rgba8(0, 0, 0, 0.94))),
                border: Border { radius: 6.0.into(), ..Default::default() },
                ..Default::default()
            }
        }),
        TooltipPosition::Top,
    )
    .gap(6)
    .into()
}

fn resolve_apply_patch_change_preview(
    app: &App,
    changes: &[ChangeFile],
    kind: char,
    path: &str,
) -> Option<ChangeFile> {
    let resolved = find_apply_patch_change(changes, path).cloned();
    #[cfg(not(target_arch = "wasm32"))]
    let resolved = resolved
        .or_else(|| (kind == 'D').then(|| deleted_file_preview_from_repo_diff(app, path)).flatten())
        .or_else(|| (kind == 'D').then(|| deleted_file_preview_from_history(app, path)).flatten())
        .or_else(|| {
            let change = find_apply_patch_change(changes, path)?;
            if kind == 'D' && change.before.is_empty() {
                deleted_file_preview_from_repo_diff(app, path)
                    .or_else(|| deleted_file_preview_from_history(app, path))
            } else {
                None
            }
        });
    #[cfg(target_arch = "wasm32")]
    let _ = (app, kind);
    resolved
}

pub(super) fn apply_patch_header_summary(
    files: &[(char, String)],
) -> (String, String, Option<String>) {
    let Some((_, first_path)) = files.first() else {
        return (String::new(), String::new(), None);
    };

    let path = Path::new(first_path);
    let file_name = path
        .file_name()
        .and_then(|segment| segment.to_str())
        .unwrap_or(first_path.as_str())
        .to_string();
    let parent_dir = path
        .parent()
        .and_then(|parent| parent.to_str())
        .filter(|parent| !parent.is_empty() && *parent != ".")
        .map(|parent| format!("/{}", parent.trim_start_matches('/')))
        .unwrap_or_default();
    let extra = if files.len() > 1 { Some(format!("等 {} 个文件", files.len())) } else { None };
    (file_name, parent_dir, extra)
}

pub(super) fn diff_utils_looks_like_unified_diff(s: &str) -> bool {
    let t = s.trim();
    !t.is_empty() && t.starts_with("--- ") && t.contains("\n+++ ") && t.contains("\n@@")
}

pub(super) fn apply_patch_secondary_text(
    theme: &Theme,
    dark_alpha: f32,
    light_alpha: f32,
) -> Color {
    let is_dark =
        theme.palette().background.r + theme.palette().background.g + theme.palette().background.b
            < 1.5;
    if is_dark {
        theme.palette().text.scale_alpha(dark_alpha)
    } else {
        theme.extended_palette().secondary.base.text.scale_alpha(light_alpha)
    }
}

pub(super) fn apply_patch_change_totals<'a>(adds: usize, dels: usize) -> Element<'a, Message> {
    row![
        text(format!("+{}", adds)).size(14).font(bold_font()).style(|theme: &Theme| {
            iced::widget::text::Style { color: Some(theme.extended_palette().success.base.color) }
        }),
        text(format!("-{}", dels)).size(14).font(bold_font()).style(|theme: &Theme| {
            iced::widget::text::Style { color: Some(theme.extended_palette().danger.base.color) }
        })
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .into()
}

pub(super) fn apply_patch_outer_card_style(_theme: &Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: None,
        border: Border::default(),
        ..Default::default()
    }
}

pub(super) fn apply_patch_inner_card_style(_theme: &Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: None,
        border: Border::default(),
        ..Default::default()
    }
}

pub(super) fn apply_patch_file_button_style(
    theme: &Theme,
    _status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    iced::widget::button::Style {
        background: None,
        border: Border::default(),
        text_color: theme.palette().text,
        ..Default::default()
    }
}

fn apply_patch_file_icon<'a>(icon: Icon) -> Element<'a, Message> {
    container(icon_svg(icon).width(Length::Fixed(13.0)).height(Length::Fixed(13.0)).style(
        |_theme: &Theme, _status| svg::Style { color: Some(Color::from_rgb8(0xF4, 0x72, 0x42)) },
    ))
    .padding([4, 5])
    .style(|theme: &Theme| {
        let ext = theme.extended_palette();
        let is_dark = theme.palette().background.r
            + theme.palette().background.g
            + theme.palette().background.b
            < 1.5;
        iced::widget::container::Style {
            background: Some(Background::Color(if is_dark {
                Color::from_rgba(0.96, 0.45, 0.24, 0.16)
            } else {
                Color::from_rgba(0.96, 0.45, 0.24, 0.10)
            })),
            border: Border {
                width: 1.0,
                color: ext.background.strong.color.scale_alpha(if is_dark { 0.36 } else { 0.28 }),
                radius: 8.0.into(),
            },
            ..Default::default()
        }
    })
    .into()
}

fn header_file_type_icon(icon: Icon) -> iced::widget::svg::Svg<'static> {
    iced::widget::svg::Svg::new(assets::get_icon(icon))
        .width(Length::Fixed(14.0))
        .height(Length::Fixed(14.0))
}

pub(super) fn apply_patch_header_file_icon(path: &str) -> Icon {
    let lower = path.to_lowercase();

    if lower.ends_with(".rs") {
        Icon::Rust
    } else if lower.ends_with(".ts") || lower.ends_with(".tsx") {
        Icon::Typescript
    } else if lower.ends_with(".js") || lower.ends_with(".jsx") {
        Icon::Javascript
    } else if lower.ends_with(".json") {
        Icon::Json
    } else if lower.ends_with(".toml") {
        Icon::Toml
    } else if lower.ends_with(".yaml") || lower.ends_with(".yml") {
        Icon::Yaml
    } else if lower.ends_with(".md") {
        Icon::Markdown
    } else if lower.ends_with(".html") || lower.ends_with(".htm") {
        Icon::Html
    } else if lower.ends_with(".css") {
        Icon::Css
    } else if lower.ends_with(".py") {
        Icon::Python
    } else if lower.ends_with(".go") {
        Icon::Go
    } else if lower.ends_with(".sh") {
        Icon::Console
    } else {
        Icon::Document
    }
}

fn preview_eye_button<'a>(abs: String) -> Element<'a, Message> {
    let eye = icon_svg(Icon::Eye).width(Length::Fixed(12.0)).height(Length::Fixed(12.0)).style(
        |theme: &Theme, _status| {
            let is_dark = theme.palette().background.r
                + theme.palette().background.g
                + theme.palette().background.b
                < 1.5;
            svg::Style {
                color: Some(if is_dark {
                    theme.palette().text.scale_alpha(0.9)
                } else {
                    theme.extended_palette().secondary.base.text
                }),
            }
        },
    );
    button(eye)
        .padding([2, 4])
        .style(apply_patch_file_button_style)
        .on_press(Message::Preview(message::PreviewMessage::Open(abs)))
        .into()
}

pub(super) fn apply_patch_file_expand_key(msg_idx: usize, tool_idx: usize, path: &str) -> String {
    format!("{msg_idx}:{tool_idx}:{path}")
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn deleted_file_preview_from_history(app: &App, path: &str) -> Option<ChangeFile> {
    let root = app.project_path.as_deref()?;
    let repo = git2::Repository::open(root).ok()?;
    let head = repo.head().ok()?;
    let tree = head.peel_to_tree().ok()?;
    let entry = tree.get_path(Path::new(path)).ok()?;
    let obj = entry.to_object(&repo).ok()?;
    let blob = obj.as_blob()?;
    let before = String::from_utf8_lossy(blob.content()).to_string();
    let deletions = before.lines().count();
    Some(ChangeFile {
        path: path.to_string(),
        additions: 0,
        deletions,
        before,
        after: String::new(),
    })
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn deleted_file_preview_from_repo_diff(app: &App, path: &str) -> Option<ChangeFile> {
    let root = app.project_path.as_deref()?;
    let repo = git2::Repository::open(root).ok()?;
    let head = repo.head().ok()?;
    let tree = head.peel_to_tree().ok()?;

    let mut opts = git2::DiffOptions::new();
    opts.include_untracked(true);
    opts.recurse_untracked_dirs(true);
    opts.pathspec(path);

    let diff = repo.diff_tree_to_workdir_with_index(Some(&tree), Some(&mut opts)).ok()?;
    let mut patch = String::new();
    diff.print(git2::DiffFormat::Patch, |_delta, _hunk, line: git2::DiffLine| {
        patch.push_str(std::str::from_utf8(line.content()).unwrap_or(""));
        true
    })
    .ok()?;

    parse_unified_diff_change_files(&patch)
        .into_iter()
        .find(|change| !change.before.is_empty() && change.after.is_empty() && change.path == path)
}
