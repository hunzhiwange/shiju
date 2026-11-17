use crate::tool_text_support::{ToolTextTarget, selected_chat_text_for_target, tool_text_editor};
use super::tool_permission::tool_permission_target_summary;
// 渲染 apply_patch 工具结果。
// 视图将补丁摘要、文件列表和差异预览组合呈现，帮助用户审查自动修改。

use iced::widget::svg;
use iced::widget::tooltip::{Position as TooltipPosition, Tooltip};
use iced::widget::{Space, button, column, container, mouse_area, row, scrollable, text};
use iced::{Alignment, Element, Length, Theme, mouse};

use desktop_assets::assets::Icon;
use desktop_ui_controls::right_click_area::RightClickArea;
use crate::card_input::{CardInput, Message};

use super::apply_patch_preview::collect_apply_patch_changes;
use super::apply_patch_card::*;
pub use super::apply_patch_card::apply_patch_file_expand_key;
use super::changes::parse_changes_file_summaries;
use super::diff_utils::{
    count_apply_patch_format_changes, count_unified_diff_changes, extract_diff_block,
    parse_apply_patch_line_changes, parse_apply_patch_summary,
};
use super::file_link::{file_name_link, file_name_only, file_path_tooltip};

use super::tool_meta::{tool_activity_header, tool_visual_state};
use super::tool_parse::{
    resolve_output_path, tool_change_file_summaries, tool_change_files, tool_error_text,
    tool_input, tool_output_path, tool_output_text, tool_status, tool_structured_diff_text,
};
use super::{
    canonical_tool_name, tool_permission_state,
};
use crate::card_style::{
    change_pills, chat_context_target_key, chat_scroll_direction, deletions_pill,
    eye_icon_button_style, eye_icon_svg_style, file_button_style, icon_svg,
    simplified_block_style, simplified_code_block_style, truncate_chars, truncate_lines_middle,
};

/// 执行 tool_apply_patch_view 对应的模块功能。
///
/// 参数含义保持与函数签名一致；返回值用于调用方继续组合处理，错误由返回类型显式表达。
pub fn tool_apply_patch_view<'a, Host: Clone + 'static>(
    app: &CardInput<'a>,
    msg_idx: usize,
    tool_idx: usize,
    visible: &str,
    render_diff: impl Fn(String, Option<String>, String, String) -> Element<'a, Host>,
) -> Option<Element<'a, Message<Host>>> {
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
    let _is_hovered = app.hovered == Some(key);
    let context_key = chat_context_target_key(msg_idx, Some(tool_idx));
    let selected_context_text = selected_chat_text_for_target(&app.text, context_key);
    let detail_btn = button(
        icon_svg(Icon::ChevronRight)
            .width(Length::Fixed(10.0))
            .height(Length::Fixed(10.0))
            .style(eye_icon_svg_style),
    )
    .padding([2, 4])
    .style(|theme: &Theme, status| eye_icon_button_style(theme, status))
    .on_press(Message::OpenToolDetail(
        msg_idx,
        tool_idx,
        visible.to_string(),
    ));

    let (primary_name, _primary_dir, trailing_summary) =
        apply_patch_header_summary(app.i18n, &files);
    let permission_target = tool_permission_target_summary(tool_name, &v);
    let title = if let Some(permission_state) = tool_permission_state(tool_name, &v) {
        crate::tools::localized_tool_permission_title(
            app.i18n,
            app.i18n.__("补丁"),
            permission_state,
        )
    } else if is_error {
        app.i18n.__("补丁失败").to_string()
    } else if is_running {
        app.i18n.__("应用补丁中").to_string()
    } else {
        app.i18n.__("补丁").to_string()
    };
    let header_icon = apply_patch_header_file_icon(
        files.first().map(|(_, path)| path.as_str()).unwrap_or(primary_name.as_str()),
    );

    let mut header_summary =
        if primary_name.is_empty() { permission_target.unwrap_or_default() } else { primary_name };
    if let Some(extra) = trailing_summary {
        if !header_summary.is_empty() {
            header_summary.push_str(" · ");
        }
        header_summary.push_str(&extra);
    }

    let mut actions =
        row![container(header_file_type_icon(header_icon)).width(Length::Fixed(16.0))]
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
    let head_row =
        tool_activity_header("apply_patch", title, header_summary, visual, actions.into());

    let head_with_hover = mouse_area(container(head_row).width(Length::Fill).padding([1, 0]))
        .on_enter(Message::ToolHover(msg_idx, tool_idx))
        .on_exit(Message::ToolHoverLeave);

    let mut content = column![head_with_hover].spacing(10);
    if is_running {
        content = content.push(
            container(
                text(app.i18n.__("应用补丁中…"))
                    .size(14)
                    .style(|theme: &Theme| iced::widget::text::Style {
                        color: Some(theme.extended_palette().secondary.base.text.scale_alpha(0.85)),
                    }),
            )
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
            let err_full =
                crate::tools::localized_tool_permission_error_text(
                    app.i18n,
                    tool_name,
                    &v,
                )
                .or_else(|| tool_error_text(&v))
                .unwrap_or_default();
            if !err_full.trim().is_empty() {
                let err_view: Element<'a, Message<Host>> = tool_text_editor(
                    &app.text,
                    ToolTextTarget::ToolCardText { msg_idx, tool_idx, text_idx: 0 },
                    "Noto Sans CJK SC",
                    14.0,
                    false,
                    true,
                )
                .map(|element| element.map(Message::Text))
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
                let err_view: Element<'a, Message<Host>> = RightClickArea::new(
                    err_view,
                    Box::new({
                        let text = selected_context_text
                            .clone()
                            .unwrap_or_else(|| err_full.trim().to_string());
                        move |point| {
                            Message::OpenMessageContextMenu {
                                target: context_key,
                                x: point.x,
                                y: point.y,
                                text: text.clone(),
                            }
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
                let abs_for_preview = if *kind != 'D' { crate::path::resolve_path(app.project_path, path) } else { None };
                let full_path = crate::path::resolve_path(app.project_path, path).unwrap_or_else(|| path.clone());
                let file_ref: Element<'a, Message<Host>> = if let Some(abs) = abs_for_preview.as_deref() {
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
                let eye_btn: Element<'a, Message<Host>> = abs_for_preview
                    .as_ref()
                    .cloned()
                    .map(preview_eye_button)
                    .unwrap_or_else(|| Space::new().width(Length::Fixed(20.0)).into());
                let count_meta: Element<'a, Message<Host>> = if let Some(c) = summaries_by_path.get(path)
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
                    let file_expanded = app.file_expanded.contains(&expanded_key);
                    let title = if *kind == 'D' {
                        format!("{}  -{}", path, c.deletions)
                    } else {
                        format!("{}  +{}-{}", path, c.additions, c.deletions)
                    };
                    let resolved_change =
                        resolve_apply_patch_change_preview(app, &changes, *kind, path);
                    let external_diff_btn: Element<'a, Message<Host>> =
                        if let Some(change) = resolved_change.clone() {
                            external_diff_button(
                                app.i18n,
                                Message::OpenChatTextDiff {
                                    title: title.clone(),
                                    file: path.clone(),
                                    before: change.before,
                                    after: change.after,
                                },
                            )
                        } else {
                            Space::new().width(Length::Fixed(20.0)).into()
                        };
                    let toggle_message = Message::ToggleToolFile(
                        msg_idx,
                        tool_idx,
                        path.clone(),
                    );
                    let view_changes_btn =
                        view_changes_toggle_button(file_expanded, toggle_message.clone());
                    let header_row = row![
                        row![apply_patch_file_icon(icon), file_ref, eye_btn, external_diff_btn]
                            .spacing(8)
                            .align_y(Alignment::Center),
                        count_meta,
                        view_changes_btn,
                    ]
                    .align_y(Alignment::Center)
                    .spacing(8);
                    let header = mouse_area(
                        container(header_row)
                            .padding([8, 0])
                            .width(Length::Fill)
                            .style(apply_patch_inner_card_style),
                    )
                    .on_press(toggle_message)
                    .interaction(mouse::Interaction::Pointer);

                    if file_expanded {
                        let diff_body: Element<'a, Message<Host>> = if let Some(change) = resolved_change
                        {
                            container(render_diff(
                                title,
                                Some(path.clone()),
                                change.before,
                                change.after,
                            ).map(Message::Diff))
                            .width(Length::Fill)
                            .clip(true)
                            .padding([4, 0])
                            .into()
                        } else if *kind == 'D' {
                            container(
                                text(
                                    app.i18n
                                        .__("文件已删除，但当前记录未包含删除前内容预览"),
                                )
                                .size(14)
                                .style(|theme: &Theme| {
                                    iced::widget::text::Style {
                                        color: Some(apply_patch_secondary_text(theme, 0.72, 0.74)),
                                    }
                                }),
                            )
                            .width(Length::Fill)
                            .clip(true)
                            .padding([10, 0])
                            .style(apply_patch_inner_card_style)
                            .into()
                        } else {
                            container(
                                text(
                                    app.i18n
                                        .__("未能解析文件变更"),
                                )
                                .size(14)
                                .style(|theme: &Theme| {
                                    iced::widget::text::Style {
                                        color: Some(apply_patch_secondary_text(theme, 0.72, 0.74)),
                                    }
                                }),
                            )
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
            let open_path = resolve_output_path(app.project_path, op);
            let btn = button(
                row![
                    icon_svg(Icon::ChevronRight).style(|theme: &Theme, _status| {
                        svg::Style { color: Some(apply_patch_secondary_text(theme, 0.92, 0.90)) }
                    }),
                    text(app.i18n.__("打开完整输出"))
                        .size(14)
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            )
            .padding([6, 10])
            .style(file_button_style)
            .on_press(Message::OpenPreview(open_path));
            content = content.push(container(btn).width(Length::Fill));
        }

        if let Some(d) = diff.as_ref().map(|s| s.trim()).filter(|s| !s.is_empty()) {
            let out = truncate_lines_middle(d, 200, 2000);
            let code = tool_text_editor(
                &app.text,
                ToolTextTarget::ToolCardText { msg_idx, tool_idx, text_idx: 0 },
                "JetBrains Mono",
                14.0,
                false,
                false,
            )
            .map(|element| element.map(Message::Text))
            .unwrap_or_else(|| {
                text(out)
                    .size(14)
                    .font(iced::Font::with_name("JetBrains Mono"))
                    .style(|theme: &Theme| iced::widget::text::Style {
                        color: Some(theme.palette().text),
                    })
                    .into()
            });
            let body: Element<'a, Message<Host>> = scrollable(
                container(code)
                    .width(Length::Fill)
                    .padding([6, 8])
                    .style(simplified_code_block_style),
            )
            .direction(chat_scroll_direction())
            .height(Length::Fixed(240.0))
            .into();
            let body: Element<'a, Message<Host>> = RightClickArea::new(
                body,
                Box::new({
                    let text = selected_context_text.clone().unwrap_or_else(|| d.to_string());
                    move |point| {
                        Message::OpenMessageContextMenu {
                            target: context_key,
                            x: point.x,
                            y: point.y,
                            text: text.clone(),
                        }
                    }
                }),
            )
            .use_global_position()
            .preserve_on_right_click()
            .into();
            content = content.push(body);
        }
    }

    let content: Element<'a, Message<Host>> =
        container(content).padding([2, 0]).width(Length::Fill).style(simplified_block_style).into();

    Some(content)
}
