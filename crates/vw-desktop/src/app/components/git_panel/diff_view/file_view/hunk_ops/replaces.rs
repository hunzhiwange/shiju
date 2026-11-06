//! Git diff 局部渲染辅助。
//!
//! 本模块负责 diff 行、行号、选区、上下文菜单和配色的局部组合。

use iced::widget::{MouseArea, container, row, text};
/// 重新导出 use iced::{Color, Element, Length}，让上层模块通过稳定路径访问。
use iced::{Color, Element, Length};

/// 重新导出 use crate::app::{App, DiffTheme, Message, message}，让上层模块通过稳定路径访问。
use crate::app::{App, DiffTheme, Message, message};

/// 重新导出 use super::super::super::super::utils::{Lang, get_word_diff_ranges, render_line_content}，让上层模块通过稳定路径访问。
use super::super::super::super::utils::{Lang, get_word_diff_ranges, render_line_content};
/// 重新导出 use super::super::super::markers::{，让上层模块通过稳定路径访问。
use super::super::super::markers::{
    LineMarkerKind, LineNumberTone, line_marker_cell_emphasis, line_number_cell_with_tone,
};
/// 重新导出 use super::super::super::selection::{is_diff_hovered, is_diff_selected}，让上层模块通过稳定路径访问。
use super::super::super::selection::{is_diff_hovered, is_diff_selected};
/// 重新导出 use super::super::super::wrap_diff_row_with_context_menu，让上层模块通过稳定路径访问。
use super::super::super::wrap_diff_row_with_context_menu;
/// 重新导出 use super::super::super::{，让上层模块通过稳定路径访问。
use super::super::super::{
    DiffSplitPaneTone, diff_highlight_enabled, diff_line_number_with_background,
    diff_split_divider, diff_split_pane, diff_split_pane_with_background, diff_split_row,
    merge_diff_row_with_background, split_line_number_area,
};
/// 重新导出 use super::super::diff_line_select_button，让上层模块通过稳定路径访问。
use super::super::diff_line_select_button;
/// 重新导出 use crate::app::components::git_panel::diff_view::DiffRenderCtx，让上层模块通过稳定路径访问。
use crate::app::components::git_panel::diff_view::DiffRenderCtx;

/// WORD_DIFF_MAX_LINE_LEN 是当前模块共享的固定参数。
const WORD_DIFF_MAX_LINE_LEN: usize = 512;

/// 处理 should compute word diff 对应的局部职责。
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
pub(super) fn should_compute_word_diff(
    app: &App,
    // old_len 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    old_len: usize,
    // new_len 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    new_len: usize,
    // old_line 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    old_line: &str,
    // new_line 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    new_line: &str,
) -> bool {
    diff_highlight_enabled(app)
        && old_len == new_len
        && old_line != new_line
        // 限制单行词级 diff 的长度，避免极长行拖慢交互渲染。
        && old_line.len() <= WORD_DIFF_MAX_LINE_LEN
        // 限制单行词级 diff 的长度，避免极长行拖慢交互渲染。
        && new_line.len() <= WORD_DIFF_MAX_LINE_LEN
}

/// 渲染 replace ops 对应的 diff 行、工具卡片或控件内容。
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
pub fn render_replace_ops(
    app: &App,
    // render_ctx 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    render_ctx: &DiffRenderCtx<'_>,
    // file 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    file: &str,
    // old_index 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    old_index: usize,
    // old_len 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    old_len: usize,
    // new_index 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    new_index: usize,
    // new_len 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    new_len: usize,
    // old_lines 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    old_lines: &[&str],
    // new_lines 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    new_lines: &[&str],
    // lang 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    lang: Lang,
    // effective_theme 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    effective_theme: DiffTheme,
    // add_line_bg 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    add_line_bg: Color,
    // add_word_bg 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    add_word_bg: Color,
    // del_line_bg 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    del_line_bg: Color,
    // del_word_bg 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    del_word_bg: Color,
    // _hover_color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    _hover_color: Color,
    // _hover_mix 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    _hover_mix: f32,
    // _hover_tint 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    _hover_tint: Color,
    // _has_selection 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    _has_selection: bool,
    // _is_modified 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    _is_modified: bool,
) -> Vec<Element<'static, Message>> {
    let mut elems = Vec::new();
    let mut old_highlights = vec![vec![]; old_len];
    let mut new_highlights = vec![vec![]; new_len];

    if old_len == new_len && diff_highlight_enabled(app) {
        for k in 0..old_len {
            let old_line = old_lines.get(old_index + k).copied().unwrap_or("");
            let new_line = new_lines.get(new_index + k).copied().unwrap_or("");

            if should_compute_word_diff(app, old_len, new_len, old_line, new_line) {
                let (o, n) = get_word_diff_ranges(old_line, new_line);
                old_highlights[k] = o;
                new_highlights[k] = n;
            }
        }
    }

    if app.merge_view {
        for k in 0..old_len {
            let old_idx = old_index + k;
            let content = old_lines.get(old_idx).unwrap_or(&"");
            let old_num = (old_idx + 1).to_string();
            let selected = is_diff_selected(app, render_ctx, file, old_idx, true);
            let hovered = is_diff_hovered(app, file, old_idx, true);
            let marker_emphasis = hovered || selected;
            let old_num_area: Element<'static, Message> =
                MouseArea::new(line_number_cell_with_tone(old_num.clone(), LineNumberTone::Delete))
                    .on_press(Message::Git(message::GitMessage::DiffDragSelectStart(
                        file.to_string(),
                        old_idx,
                        true,
                        (*content).to_string(),
                    )))
                    .on_enter(Message::Git(message::GitMessage::DiffDragSelectHover(
                        file.to_string(),
                        old_idx,
                        true,
                    )))
                    .into();
            let old_num_area = diff_line_number_with_background(old_num_area, del_line_bg);
            let content_row = render_line_content(
                content,
                lang,
                effective_theme,
                diff_highlight_enabled(app),
                &old_highlights[k],
                // Color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                Color::TRANSPARENT,
                del_word_bg,
            );
            let content_area: Element<'static, Message> =
                MouseArea::new(container(content_row).width(Length::Fill).padding([0, 2]))
                    .on_press(Message::Git(message::GitMessage::DiffDragSelectStart(
                        file.to_string(),
                        old_idx,
                        true,
                        (*content).to_string(),
                    )))
                    .on_enter(Message::Git(message::GitMessage::DiffDragSelectHover(
                        file.to_string(),
                        old_idx,
                        true,
                    )))
                    .into();

            let old_line_checked_base = render_ctx.selected_old_lines.contains(&(file, old_idx));
            let old_line_checked = render_ctx.is_old_line_staged(file, old_idx);
            let stage_old_cb = diff_line_select_button(
                old_line_checked,
                if old_line_checked_base {
                    // Message 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                    Message::Git(message::GitMessage::ToggleStageOldLine(
                        file.to_string(),
                        old_idx,
                        false,
                    ))
                } else {
                    // Message 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                    Message::Git(message::GitMessage::ToggleStageOldLine(
                        file.to_string(),
                        old_idx,
                        true,
                    ))
                },
                render_ctx.read_only,
            );

            let r = merge_diff_row_with_background(
                container(
                    row![
                        line_marker_cell_emphasis(LineMarkerKind::Delete, marker_emphasis),
                        stage_old_cb,
                        old_num_area,
                        content_area
                    ]
                    .align_y(iced::Alignment::Center),
                )
                .width(Length::Fill)
                .into(),
                del_line_bg,
                marker_emphasis,
            );
            let wrapped = MouseArea::new(r)
                .on_enter(Message::Git(message::GitMessage::DiffHoverEnter(
                    file.to_string(),
                    old_idx,
                    true,
                )))
                .on_exit(Message::Git(message::GitMessage::DiffHoverExit(
                    file.to_string(),
                    old_idx,
                    true,
                )));
            elems.push(wrap_diff_row_with_context_menu(
                app,
                file,
                old_idx,
                true,
                (*content).to_string(),
                wrapped.into(),
            ));
        }
        for k in 0..new_len {
            let new_idx = new_index + k;
            let content = new_lines.get(new_idx).unwrap_or(&"");
            let new_num = (new_idx + 1).to_string();
            let selected = is_diff_selected(app, render_ctx, file, new_idx, false);
            let hovered = is_diff_hovered(app, file, new_idx, false);
            let marker_emphasis = hovered || selected;
            let new_num_area: Element<'static, Message> =
                MouseArea::new(line_number_cell_with_tone(new_num.clone(), LineNumberTone::Add))
                    .on_press(Message::Git(message::GitMessage::DiffDragSelectStart(
                        file.to_string(),
                        new_idx,
                        false,
                        (*content).to_string(),
                    )))
                    .on_enter(Message::Git(message::GitMessage::DiffDragSelectHover(
                        file.to_string(),
                        new_idx,
                        false,
                    )))
                    .into();
            let new_num_area = diff_line_number_with_background(new_num_area, add_line_bg);
            let content_row = render_line_content(
                content,
                lang,
                effective_theme,
                diff_highlight_enabled(app),
                &new_highlights[k],
                // Color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                Color::TRANSPARENT,
                add_word_bg,
            );
            let content_area: Element<'static, Message> =
                MouseArea::new(container(content_row).width(Length::Fill).padding([0, 2]))
                    .on_press(Message::Git(message::GitMessage::DiffDragSelectStart(
                        file.to_string(),
                        new_idx,
                        false,
                        (*content).to_string(),
                    )))
                    .on_enter(Message::Git(message::GitMessage::DiffDragSelectHover(
                        file.to_string(),
                        new_idx,
                        false,
                    )))
                    .into();

            let line_checked_base = render_ctx.selected_new_lines.contains(&(file, new_idx));
            let line_checked = render_ctx.is_new_line_staged(file, new_idx);
            let stage_cb = diff_line_select_button(
                line_checked,
                if line_checked_base {
                    // Message 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                    Message::Git(message::GitMessage::ToggleStageLine(
                        file.to_string(),
                        new_idx,
                        false,
                    ))
                } else {
                    // Message 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                    Message::Git(message::GitMessage::ToggleStageLine(
                        file.to_string(),
                        new_idx,
                        true,
                    ))
                },
                render_ctx.read_only,
            );

            let r = merge_diff_row_with_background(
                container(
                    row![
                        line_marker_cell_emphasis(LineMarkerKind::Add, marker_emphasis),
                        stage_cb,
                        new_num_area,
                        content_area
                    ]
                    .align_y(iced::Alignment::Center),
                )
                .width(Length::Fill)
                .into(),
                add_line_bg,
                marker_emphasis,
            );
            let wrapped = MouseArea::new(r)
                .on_enter(Message::Git(message::GitMessage::DiffHoverEnter(
                    file.to_string(),
                    new_idx,
                    false,
                )))
                .on_exit(Message::Git(message::GitMessage::DiffHoverExit(
                    file.to_string(),
                    new_idx,
                    false,
                )));
            elems.push(wrap_diff_row_with_context_menu(
                app,
                file,
                new_idx,
                false,
                (*content).to_string(),
                wrapped.into(),
            ));
        }
    } else {
        let count = std::cmp::max(old_len, new_len);
        for k in 0..count {
            let has_old = k < old_len;
            let has_new = k < new_len;
            let old_idx = old_index + k;
            let new_idx = new_index + k;
            let hovered_old = has_old && is_diff_hovered(app, file, old_idx, true);
            let hovered_new = has_new && is_diff_hovered(app, file, new_idx, false);
            let hovered_any = hovered_old || hovered_new;
            let (hover_line, hover_is_old, row_text) = if has_old {
                (old_idx, true, old_lines.get(old_idx).copied().unwrap_or("").to_string())
            } else {
                (new_idx, false, new_lines.get(new_idx).copied().unwrap_or("").to_string())
            };
            let old_content = if has_old { old_lines.get(old_idx).unwrap_or(&"") } else { &"" };
            let new_content = if has_new { new_lines.get(new_idx).unwrap_or(&"") } else { &"" };

            let old_ranges = if has_old && k < old_highlights.len() {
                &old_highlights[k]
            } else {
                &[] as &[std::ops::Range<usize>]
            };
            let new_ranges = if has_new && k < new_highlights.len() {
                &new_highlights[k]
            } else {
                &[] as &[std::ops::Range<usize>]
            };
            let left_num_area = split_line_number_area(
                file,
                if has_old { Some((old_idx, true)) } else { None },
                &row_text,
                if has_old { LineNumberTone::Delete } else { LineNumberTone::Neutral },
            );
            let left_num_area = if has_old {
                diff_line_number_with_background(left_num_area, del_line_bg)
            } else {
                left_num_area
            };
            let right_num_area = split_line_number_area(
                file,
                if has_new { Some((new_idx, false)) } else { None },
                &row_text,
                if has_new { LineNumberTone::Add } else { LineNumberTone::Neutral },
            );
            let right_num_area = if has_new {
                diff_line_number_with_background(right_num_area, add_line_bg)
            } else {
                right_num_area
            };

            let old_part = if has_old {
                let selected = is_diff_selected(app, render_ctx, file, old_idx, true);
                let marker_emphasis = selected || hovered_any;
                let old_line_checked_base =
                    render_ctx.selected_old_lines.contains(&(file, old_idx));
                let old_line_checked = render_ctx.is_old_line_staged(file, old_idx);
                let stage_old_cb = diff_line_select_button(
                    old_line_checked,
                    if old_line_checked_base {
                        // Message 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                        Message::Git(message::GitMessage::ToggleStageOldLine(
                            file.to_string(),
                            old_idx,
                            false,
                        ))
                    } else {
                        // Message 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                        Message::Git(message::GitMessage::ToggleStageOldLine(
                            file.to_string(),
                            old_idx,
                            true,
                        ))
                    },
                    render_ctx.read_only,
                );
                let row = render_line_content(
                    old_content,
                    lang,
                    effective_theme,
                    diff_highlight_enabled(app),
                    old_ranges,
                    // Color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                    Color::TRANSPARENT,
                    del_word_bg,
                );
                let content_area: Element<'static, Message> =
                    MouseArea::new(container(row).width(Length::Fill))
                        .on_press(Message::Git(message::GitMessage::DiffDragSelectStart(
                            file.to_string(),
                            old_idx,
                            true,
                            old_content.to_string(),
                        )))
                        .on_enter(Message::Git(message::GitMessage::DiffDragSelectHover(
                            file.to_string(),
                            old_idx,
                            true,
                        )))
                        .into();
                diff_split_pane_with_background(
                    container(
                        row![
                            left_num_area,
                            line_marker_cell_emphasis(LineMarkerKind::None, false),
                            stage_old_cb,
                            content_area
                        ]
                        .spacing(4)
                        .align_y(iced::Alignment::Center)
                        .width(Length::Fill),
                    )
                    .padding([0, 2])
                    .into(),
                    del_line_bg,
                    marker_emphasis,
                )
            } else {
                diff_split_pane(
                    container(
                        row![left_num_area, container(text("")).width(Length::Fill)]
                            .width(Length::Fill),
                    )
                    .padding([0, 2])
                    .into(),
                    // DiffSplitPaneTone 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                    DiffSplitPaneTone::Empty,
                    hovered_any,
                )
            };

            let new_part = if has_new {
                let selected = is_diff_selected(app, render_ctx, file, new_idx, false);
                let marker_emphasis = selected || hovered_any;
                let line_checked_base = render_ctx.selected_new_lines.contains(&(file, new_idx));
                let line_checked = render_ctx.is_new_line_staged(file, new_idx);
                let stage_cb = diff_line_select_button(
                    line_checked,
                    if line_checked_base {
                        // Message 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                        Message::Git(message::GitMessage::ToggleStageLine(
                            file.to_string(),
                            new_idx,
                            false,
                        ))
                    } else {
                        // Message 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                        Message::Git(message::GitMessage::ToggleStageLine(
                            file.to_string(),
                            new_idx,
                            true,
                        ))
                    },
                    render_ctx.read_only,
                );
                let row = render_line_content(
                    new_content,
                    lang,
                    effective_theme,
                    diff_highlight_enabled(app),
                    new_ranges,
                    // Color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                    Color::TRANSPARENT,
                    add_word_bg,
                );
                let content_area: Element<'static, Message> =
                    MouseArea::new(container(row).width(Length::Fill))
                        .on_press(Message::Git(message::GitMessage::DiffDragSelectStart(
                            file.to_string(),
                            new_idx,
                            false,
                            new_content.to_string(),
                        )))
                        .on_enter(Message::Git(message::GitMessage::DiffDragSelectHover(
                            file.to_string(),
                            new_idx,
                            false,
                        )))
                        .into();
                diff_split_pane_with_background(
                    container(
                        row![
                            right_num_area,
                            line_marker_cell_emphasis(LineMarkerKind::None, false),
                            stage_cb,
                            content_area
                        ]
                        .spacing(4)
                        .align_y(iced::Alignment::Center)
                        .width(Length::Fill),
                    )
                    .padding([0, 2])
                    .into(),
                    add_line_bg,
                    marker_emphasis,
                )
            } else {
                diff_split_pane(
                    container(
                        row![right_num_area, container(text("")).width(Length::Fill)]
                            .width(Length::Fill),
                    )
                    .padding([0, 2])
                    .into(),
                    // DiffSplitPaneTone 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                    DiffSplitPaneTone::Empty,
                    hovered_any,
                )
            };

            let r = container(diff_split_row([old_part, diff_split_divider(), new_part]))
                .width(Length::Fill);
            let wrapped = MouseArea::new(r)
                .on_enter(Message::Git(message::GitMessage::DiffHoverEnter(
                    file.to_string(),
                    hover_line,
                    hover_is_old,
                )))
                .on_exit(Message::Git(message::GitMessage::DiffHoverExit(
                    file.to_string(),
                    hover_line,
                    hover_is_old,
                )));
            elems.push(wrap_diff_row_with_context_menu(
                app,
                file,
                hover_line,
                hover_is_old,
                row_text,
                wrapped.into(),
            ));
        }
    }

    elems
}
