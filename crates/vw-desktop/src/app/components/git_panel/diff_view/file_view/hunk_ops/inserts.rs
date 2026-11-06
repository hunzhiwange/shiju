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
    merge_diff_row, merge_diff_row_with_background, split_line_number_area,
};
/// 重新导出 use super::super::diff_line_select_button，让上层模块通过稳定路径访问。
use super::super::diff_line_select_button;
/// 重新导出 use crate::app::components::git_panel::diff_view::DiffRenderCtx，让上层模块通过稳定路径访问。
use crate::app::components::git_panel::diff_view::DiffRenderCtx;

/// 渲染 insert ops 对应的 diff 行、工具卡片或控件内容。
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
pub fn render_insert_ops(
    app: &App,
    // render_ctx 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    render_ctx: &DiffRenderCtx<'_>,
    // file 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    file: &str,
    // new_index 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    new_index: usize,
    // new_len 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    new_len: usize,
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
    let mut elems = Vec::with_capacity(new_len);
    for k in 0..new_len {
        let new_idx = new_index + k;
        let content = new_lines.get(new_idx).unwrap_or(&"");
        let new_num = (new_idx + 1).to_string();
        let selected = is_diff_selected(app, render_ctx, file, new_idx, false);
        let hovered = is_diff_hovered(app, file, new_idx, false);
        let hovered_or_selected = hovered || selected;
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
            &[],
            // Color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            Color::TRANSPARENT,
            add_word_bg,
        );
        let content_area: Element<'static, Message> =
            MouseArea::new(container(content_row).width(Length::Fill))
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
                Message::Git(message::GitMessage::ToggleStageLine(file.to_string(), new_idx, false))
            } else {
                // Message 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                Message::Git(message::GitMessage::ToggleStageLine(file.to_string(), new_idx, true))
            },
            render_ctx.read_only,
        );

        if app.merge_view {
            let r = merge_diff_row_with_background(
                container(
                    row![
                        line_marker_cell_emphasis(LineMarkerKind::Add, hovered_or_selected),
                        stage_cb,
                        new_num_area,
                        content_area
                    ]
                    .align_y(iced::Alignment::Center),
                )
                .width(Length::Fill)
                .padding([0, 2])
                .into(),
                add_line_bg,
                hovered_or_selected,
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
        } else {
            let left_num_area =
                split_line_number_area(file, None, content, LineNumberTone::Neutral);
            let right_num_area =
                split_line_number_area(file, Some((new_idx, false)), content, LineNumberTone::Add);
            let right_num_area = diff_line_number_with_background(right_num_area, add_line_bg);
            let left_part = diff_split_pane(
                container(
                    row![left_num_area, container(text("")).width(Length::Fill)]
                        .width(Length::Fill),
                )
                .padding([0, 2])
                .into(),
                // DiffSplitPaneTone 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                DiffSplitPaneTone::Empty,
                hovered_or_selected,
            );

            let right_part = diff_split_pane_with_background(
                container(
                    row![
                        right_num_area,
                        line_marker_cell_emphasis(LineMarkerKind::Add, hovered_or_selected),
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
                hovered_or_selected,
            );
            let r = merge_diff_row(
                container(diff_split_row([left_part, diff_split_divider(), right_part]))
                    .width(Length::Fill)
                    .into(),
                // DiffSplitPaneTone 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                DiffSplitPaneTone::Add,
                hovered_or_selected,
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
    }

    elems
}
