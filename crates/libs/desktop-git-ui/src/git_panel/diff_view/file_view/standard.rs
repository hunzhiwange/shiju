//! Git diff 局部渲染辅助。
//!
//! 本模块负责 diff 行、行号、选区、上下文菜单和配色的局部组合。

use iced::widget::{MouseArea, container, row};
use iced::{Color, Element, Length};

use crate::DiffTheme;
use crate::git_panel::diff_view::{DiffViewState, DiffMessage as Message};

use super::super::super::utils::{Lang, render_line_content};
use super::super::markers::{LineMarkerKind, line_marker_cell_emphasis, line_number_cell};
use super::super::selection::{is_diff_hovered, is_diff_selected};
use super::super::wrap_diff_row_with_context_menu;
use super::super::{DiffRenderCtx, diff_highlight_enabled};
use super::diff_line_select_spacer;

/// 渲染 equal line 对应的 diff 行、工具卡片或控件内容。
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
pub fn render_equal_line(
    app: &DiffViewState<'_>,
    // render_ctx 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    render_ctx: &DiffRenderCtx<'_>,
    // file 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    file: &str,
    // old_idx 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    old_idx: usize,
    // new_idx 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    new_idx: usize,
    // content 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    content: &str,
    // lang 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    lang: Lang,
    // effective_theme 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    effective_theme: DiffTheme,
    // bg_default 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    bg_default: Color,
    // _hover_tint 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    _hover_tint: Color,
    // _has_selection 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    _has_selection: bool,
) -> Element<'static, Message> {
    let hovered_or_selected = is_diff_selected(app, render_ctx, file, old_idx, true)
        || is_diff_selected(app, render_ctx, file, new_idx, false)
        || is_diff_hovered(app, file, old_idx, true)
        || is_diff_hovered(app, file, new_idx, false);
    let new_num = (new_idx + 1).to_string();
    let new_num_area: Element<'static, Message> = MouseArea::new(line_number_cell(new_num.clone()))
        .on_press(Message::DiffDragSelectStart(
            file.to_string(),
            new_idx,
            false,
            content.to_string(),
        ))
        .on_enter(Message::DiffDragSelectHover(
            file.to_string(),
            new_idx,
            false,
        ))
        .into();

    let content_row = render_line_content(
        content,
        lang,
        effective_theme,
        diff_highlight_enabled(app),
        &[],
        bg_default,
        bg_default,
    );
    let content_area: Element<'static, Message> =
        MouseArea::new(container(content_row).width(Length::Fill).padding([0, 2]))
            .on_press(Message::DiffDragSelectStart(
                file.to_string(),
                new_idx,
                false,
                content.to_string(),
            ))
            .on_enter(Message::DiffDragSelectHover(
                file.to_string(),
                new_idx,
                false,
            ))
            .into();
    let r = row![
        line_marker_cell_emphasis(LineMarkerKind::None, hovered_or_selected),
        diff_line_select_spacer(),
        new_num_area,
        content_area
    ];
    let r = container(r).width(Length::Fill);
    wrap_diff_row_with_context_menu(
        app,
        file,
        new_idx,
        false,
        content.to_string(),
        // MouseArea 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        MouseArea::new(r)
            .on_enter(Message::DiffHoverEnter(
                file.to_string(),
                new_idx,
                false,
            ))
            .on_exit(Message::DiffHoverExit(
                file.to_string(),
                new_idx,
                false,
            ))
            .into(),
    )
}
