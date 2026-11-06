//! Git diff 局部渲染辅助。
//!
//! 本模块负责 diff 行、行号、选区、上下文菜单和配色的局部组合。

use iced::widget::{MouseArea, container, row, text};
/// 重新导出 use iced::{Color, Element, Length}，让上层模块通过稳定路径访问。
use iced::{Color, Element, Length};

/// 重新导出 use crate::app::{App, DiffTheme, Message, message}，让上层模块通过稳定路径访问。
use crate::app::{App, DiffTheme, Message, message};

/// 重新导出 use super::super::super::super::utils::{Lang, render_line_content}，让上层模块通过稳定路径访问。
use super::super::super::super::utils::{Lang, render_line_content};
/// 重新导出 use super::super::super::markers::{，让上层模块通过稳定路径访问。
use super::super::super::markers::{
    LineMarkerKind, LineNumberTone, line_marker_cell_emphasis, line_number_cell_with_tone_offset,
};
/// 重新导出 use super::super::super::selection::{is_diff_hovered, is_diff_selected}，让上层模块通过稳定路径访问。
use super::super::super::selection::{is_diff_hovered, is_diff_selected};
/// 重新导出 use super::super::super::wrap_diff_row_with_context_menu，让上层模块通过稳定路径访问。
use super::super::super::wrap_diff_row_with_context_menu;
/// 重新导出 use super::super::super::{，让上层模块通过稳定路径访问。
use super::super::super::{
    DiffSplitPaneTone, diff_highlight_enabled, diff_line_number_with_background,
    diff_split_divider, diff_split_pane, diff_split_pane_with_background, diff_split_row,
    merge_diff_row, merge_diff_row_with_background, split_line_number_area,
};
/// 重新导出 use super::super::diff_line_select_button，让上层模块通过稳定路径访问。
use super::super::diff_line_select_button;
/// 重新导出 use crate::app::components::git_panel::diff_view::DiffRenderCtx，让上层模块通过稳定路径访问。
use crate::app::components::git_panel::diff_view::DiffRenderCtx;

/// 渲染 delete ops 对应的 diff 行、工具卡片或控件内容。
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
pub fn render_delete_ops(
    app: &App,
    // render_ctx 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    render_ctx: &DiffRenderCtx<'_>,
    // file 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    file: &str,
    // old_index 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    old_index: usize,
    // old_len 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    old_len: usize,
    // _new_index 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    _new_index: usize,
    // old_lines 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    old_lines: &[&str],
    // lang 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    lang: Lang,
    // effective_theme 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    effective_theme: DiffTheme,
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

    for k in 0..old_len {
        let old_idx = old_index + k;
        let content = old_lines.get(old_idx).unwrap_or(&"");
        let old_num = (old_idx + 1).to_string();
        let selected = is_diff_selected(app, render_ctx, file, old_idx, true);
        let hovered = is_diff_hovered(app, file, old_idx, true);
        let hovered_or_selected = hovered || selected;

        let content_row = render_line_content(
            content,
            lang,
            effective_theme,
            diff_highlight_enabled(app),
            &[],
            // Color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            Color::TRANSPARENT,
            del_word_bg,
        );

        let content_area: Element<'static, Message> =
            MouseArea::new(container(content_row).width(Length::Fill))
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

        let old_num_area: Element<'static, Message> = MouseArea::new(
            line_number_cell_with_tone_offset(old_num.clone(), LineNumberTone::Delete, 3.0),
        )
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

        if app.merge_view {
            let r = merge_diff_row_with_background(
                container(
                    row![
                        line_marker_cell_emphasis(LineMarkerKind::Delete, hovered_or_selected),
                        stage_old_cb,
                        old_num_area,
                        container(content_area).padding([0, 2])
                    ]
                    .align_y(iced::Alignment::Center),
                )
                .width(Length::Fill)
                .into(),
                del_line_bg,
                hovered_or_selected,
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
        } else {
            let left_num_area = split_line_number_area(
                file,
                Some((old_idx, true)),
                content,
                // LineNumberTone 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                LineNumberTone::Delete,
            );
            let left_num_area = diff_line_number_with_background(left_num_area, del_line_bg);
            let right_num_area =
                split_line_number_area(file, None, content, LineNumberTone::Neutral);
            let left_part = diff_split_pane_with_background(
                container(
                    row![
                        left_num_area,
                        line_marker_cell_emphasis(LineMarkerKind::Delete, hovered_or_selected),
                        stage_old_cb,
                        content_area
                    ]
                    .spacing(4)
                    .align_y(iced::Alignment::Center)
                    .width(Length::Fill),
                )
                .padding([0, 1])
                .into(),
                del_line_bg,
                hovered_or_selected,
            );

            let right_part = diff_split_pane(
                container(
                    row![right_num_area, container(text("")).width(Length::Fill)]
                        .width(Length::Fill),
                )
                .padding([0, 2])
                .into(),
                // DiffSplitPaneTone 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                DiffSplitPaneTone::Empty,
                hovered_or_selected,
            );

            let r = merge_diff_row(
                container(diff_split_row([left_part, diff_split_divider(), right_part]))
                    .width(Length::Fill)
                    .into(),
                // DiffSplitPaneTone 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                DiffSplitPaneTone::Delete,
                hovered_or_selected,
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
    }

    elems
}
