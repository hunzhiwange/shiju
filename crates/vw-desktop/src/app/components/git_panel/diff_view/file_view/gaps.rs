//! Git Diff 间隙渲染模块
//!
//! 本模块负责渲染 Git diff 视图中的间隙区域。间隙是指在 diff 视图中被折叠隐藏的
//! 未变更代码行。为了提高大型文件 diff 的可读性，系统会自动折叠连续的未变更行，
//! 只显示上下文行（变更行前后的若干行）。
//!
//! # 主要功能
//!
//! - 渲染间隙区域的代码行（包含行号和内容）
//! - 支持上下文展开功能（用户可以手动展开被隐藏的行）
//! - 支持两种视图模式：合并视图（merge view）和分栏视图（split view）
//! - 处理鼠标交互（悬停高亮、行选择、拖拽选择）
//! - 提供展开控制按钮（向上展开、向下展开、全部展开）
//!
//! # 视图模式
//!
//! - **合并视图**：左右两侧使用相同的内容，适用于显示未变更的上下文行
//! - **分栏视图**：左侧显示旧版本，右侧显示新版本，保持与 diff 的分栏布局一致

use iced::widget::{MouseArea, container, row, text};
use iced::{Background, Border, Color, Element, Length};

use crate::app::assets::Icon;
use crate::app::{App, DiffTheme, Message, message};

use super::super::super::ui::small_plain_icon_button;
use super::super::super::utils::{Lang, render_line_content};
use super::super::markers::{
    LineMarkerKind, LineNumberTone, empty_line_number_cell, line_marker_cell_emphasis,
};
use super::super::selection::{is_diff_hovered, is_diff_selected};
use super::super::wrap_diff_row_with_context_menu;
use super::super::{
    DiffRenderCtx, DiffSplitPaneTone, diff_highlight_enabled, diff_split_divider, diff_split_pane,
    merge_diff_row, split_line_number_area,
};
use super::diff_line_select_spacer;
use super::start_end::GapRange;

pub(super) fn gap_summary_text_style(theme: &iced::Theme) -> iced::widget::text::Style {
    let ext = theme.extended_palette();
    let is_dark =
        theme.palette().background.r + theme.palette().background.g + theme.palette().background.b
            < 1.5;

    iced::widget::text::Style {
        color: Some(if is_dark {
            theme.palette().text.scale_alpha(0.98)
        } else {
            ext.background.strong.text.scale_alpha(0.92)
        }),
    }
}

pub(super) fn gap_expand_container_style(theme: &iced::Theme) -> iced::widget::container::Style {
    let ext = theme.extended_palette();
    let is_dark =
        theme.palette().background.r + theme.palette().background.g + theme.palette().background.b
            < 1.5;

    iced::widget::container::Style {
        background: Some(Background::Color(if is_dark {
            ext.background.weak.color.scale_alpha(0.28)
        } else {
            ext.background.weak.color.scale_alpha(0.82)
        })),
        border: Border {
            width: 1.0,
            color: ext.background.strong.color.scale_alpha(if is_dark { 0.24 } else { 0.10 }),
            radius: 7.0.into(),
        },
        ..Default::default()
    }
}

fn gap_summary_chip(hidden_count: usize) -> Element<'static, Message> {
    container(text(format!("{} 行未修改", hidden_count)).size(12).style(gap_summary_text_style))
        .padding([1, 4])
        .into()
}

/// 渲染 Git diff 中的间隙区域
///
/// 该函数负责渲染被折叠隐藏的未变更代码行。系统会根据上下文展开配置
/// 决定显示哪些行，以及是否需要显示展开按钮。
///
/// # 参数
///
/// - `app`: 应用状态引用，包含配置、展开状态等信息
/// - `file`: 当前文件路径，用于标识和消息传递
/// - `old_lines`: 旧版本文件的所有行内容（字符串切片数组）
/// - `range`: 间隙范围信息，包含起始/结束行号和间隙ID
/// - `lang`: 编程语言类型，用于语法高亮
/// - `effective_theme`: diff 主题配置
/// - `bg_default`: 默认背景颜色
/// - `hover_tint`: 悬停时的着色颜色
/// - `has_selection`: 是否有选中的内容（保留参数以兼容调用方）
///
/// # 返回值
///
/// 返回一个 Element 向量，包含渲染后的行元素和可能的展开按钮
///
/// # 示例
///
/// ```ignore
/// let elements = render_gap(
///     &app,
///     "src/main.rs",
///     &old_lines,
///     gap_range,
///     Lang::Rust,
///     DiffTheme::Dark,
///     Color::WHITE,
///     Color::from_rgb(0.95, 0.95, 0.95),
///     false,
/// );
/// ```
///
/// # 渲染逻辑
///
/// 1. 计算间隙长度（被隐藏的行数）
/// 2. 获取上下文展开配置（默认显示3行上下文）
/// 3. 根据展开配置决定显示哪些行
/// 4. 如果所有行都能显示，则直接渲染所有行
/// 5. 如果仍有隐藏行，则显示部分行 + 展开按钮 + 剩余行
///
/// # 交互支持
///
/// - 鼠标悬停高亮
/// - 行选择和边框高亮
/// - 拖拽选择多行
/// - 复制选中内容
/// - 展开上下文
pub fn render_gap(
    app: &App,
    render_ctx: &DiffRenderCtx<'_>,
    file: &str,
    old_lines: &[&str],
    range: GapRange,
    lang: Lang,
    effective_theme: DiffTheme,
    bg_default: Color,
    _hover_tint: Color,
    _has_selection: bool,
) -> Vec<Element<'static, Message>> {
    let mut elements: Vec<Element<'static, Message>> = Vec::new();

    // 计算间隙长度（被隐藏的行数）
    let gap_len = range.end_old.saturating_sub(range.start_old);
    if gap_len == 0 {
        return elements;
    }

    // 获取上下文展开配置
    // exp_down: 向下展开的额外行数
    // exp_up: 向上展开的额外行数
    let (exp_down, exp_up) =
        app.context_expansions.get(&(file.to_string(), range.gap_id)).cloned().unwrap_or((0, 0));

    // 默认上下文行数（变更行前后显示的未变更行数）
    let default_ctx = 3;

    // 计算顶部和底部可见的行数（默认 + 用户展开的额外行数）
    let visible_top = default_ctx + exp_down;
    let visible_bottom = default_ctx + exp_up;

    // 渲染单行内容的闭包
    // k: 相对于间隙起始位置的偏移量（0-based）
    let render_line = |k: usize| {
        // 计算实际的旧版本和新版本行索引
        let old_idx = range.start_old + k;
        let new_idx = range.start_new + k;

        // 获取该行的内容，如果索引越界则使用空字符串
        let content = old_lines.get(old_idx).unwrap_or(&"");

        // 行号从1开始（1-indexed）
        let new_num = (new_idx + 1).to_string();
        let hovered_or_selected = is_diff_selected(app, render_ctx, file, old_idx, true)
            || is_diff_selected(app, render_ctx, file, new_idx, false)
            || is_diff_hovered(app, file, old_idx, true)
            || is_diff_hovered(app, file, new_idx, false);

        // 创建新版本行号区域（带鼠标交互）
        let new_num_area: Element<'static, Message> =
            MouseArea::new(super::super::markers::line_number_cell_with_tone(
                new_num.clone(),
                LineNumberTone::Neutral,
            ))
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

        // 创建复制按钮（仅在有待复制内容时显示）
        // 根据视图模式渲染不同的布局
        if app.merge_view {
            // 合并视图：左右两侧显示相同内容
            let content_row = render_line_content(
                content,
                lang,
                effective_theme,
                diff_highlight_enabled(app),
                &[],
                bg_default,
                bg_default,
            );

            // 创建内容区域（带鼠标交互）
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

            // 构建行布局：标记 | 复制按钮 | 旧行号 | 新行号 | 分隔符 | 内容
            let r = row![
                line_marker_cell_emphasis(LineMarkerKind::None, hovered_or_selected),
                diff_line_select_spacer(),
                new_num_area,
                content_area
            ];

            let r = container(r).width(Length::Fill);

            // 包装鼠标区域以支持悬停事件
            wrap_diff_row_with_context_menu(
                app,
                file,
                old_idx,
                true,
                (*content).to_string(),
                MouseArea::new(r)
                    .on_enter(Message::Git(message::GitMessage::DiffHoverEnter(
                        file.to_string(),
                        old_idx,
                        true,
                    )))
                    .on_exit(Message::Git(message::GitMessage::DiffHoverExit(
                        file.to_string(),
                        old_idx,
                        true,
                    )))
                    .into(),
            )
        } else {
            let left_num_area = split_line_number_area(
                file,
                Some((old_idx, true)),
                content,
                LineNumberTone::Neutral,
            );
            let right_num_area = split_line_number_area(
                file,
                Some((new_idx, false)),
                content,
                LineNumberTone::Neutral,
            );
            // 分栏视图：左侧显示旧版本，右侧显示新版本
            let left_row = render_line_content(
                content,
                lang,
                effective_theme,
                diff_highlight_enabled(app),
                &[],
                bg_default,
                bg_default,
            );
            let right_row = render_line_content(
                content,
                lang,
                effective_theme,
                diff_highlight_enabled(app),
                &[],
                bg_default,
                bg_default,
            );

            // 创建左侧内容区域（带鼠标交互）
            let left_area: Element<'static, Message> =
                MouseArea::new(container(left_row).width(Length::Fill).padding([0, 2]))
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

            // 创建右侧内容区域（带鼠标交互）
            let right_area: Element<'static, Message> =
                MouseArea::new(container(right_row).width(Length::Fill).padding([0, 2]))
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

            // 构建行布局：标记 | 复制按钮 | 旧行号 | 左侧内容 | 新行号 | 右侧内容
            let left_part = diff_split_pane(
                container(
                    row![left_num_area, left_area]
                        .spacing(4)
                        .align_y(iced::Alignment::Center)
                        .width(Length::Fill),
                )
                .padding([0, 2])
                .into(),
                DiffSplitPaneTone::Neutral,
                hovered_or_selected,
            );
            let right_part = diff_split_pane(
                container(
                    row![right_num_area, right_area]
                        .spacing(4)
                        .align_y(iced::Alignment::Center)
                        .width(Length::Fill),
                )
                .padding([0, 2])
                .into(),
                DiffSplitPaneTone::Neutral,
                hovered_or_selected,
            );
            let r = merge_diff_row(
                container(
                    row![
                        line_marker_cell_emphasis(LineMarkerKind::None, hovered_or_selected),
                        left_part,
                        diff_split_divider(),
                        right_part,
                    ]
                    .width(Length::Fill),
                )
                .width(Length::Fill)
                .into(),
                DiffSplitPaneTone::Neutral,
                hovered_or_selected,
            );

            // 包装鼠标区域以支持悬停事件
            wrap_diff_row_with_context_menu(
                app,
                file,
                old_idx,
                true,
                (*content).to_string(),
                MouseArea::new(r)
                    .on_enter(Message::Git(message::GitMessage::DiffHoverEnter(
                        file.to_string(),
                        old_idx,
                        true,
                    )))
                    .on_exit(Message::Git(message::GitMessage::DiffHoverExit(
                        file.to_string(),
                        old_idx,
                        true,
                    )))
                    .into(),
            )
        }
    };

    // 计算实际可见的顶部和底部行数（不能超过间隙长度）
    let vis_top = std::cmp::min(visible_top, gap_len);
    let vis_bottom = std::cmp::min(visible_bottom, gap_len);
    let total = vis_top.saturating_add(vis_bottom);

    // 判断是否所有行都能显示
    if total >= gap_len {
        // 情况1：展开的上下文已经覆盖整个间隙，直接渲染所有行
        for k in 0..gap_len {
            elements.push(render_line(k));
        }
    } else {
        // 情况2：仍有部分行被隐藏，显示部分行 + 展开按钮 + 剩余行

        // 渲染顶部可见的行
        for k in 0..vis_top {
            elements.push(render_line(k));
        }

        // 创建展开按钮容器
        let hidden_count = gap_len.saturating_sub(total);
        let expand_content = container(
            row![
                gap_summary_chip(hidden_count),
                iced::widget::Space::new().width(Length::Fill).height(Length::Shrink),
                small_plain_icon_button(
                    Some(Icon::ChevronUp),
                    "展开上方".to_string(),
                    Message::Git(message::GitMessage::ExpandContext(
                        file.to_string(),
                        range.gap_id,
                        crate::app::message::git::ExpandDirection::Up,
                    )),
                ),
                small_plain_icon_button(
                    Some(Icon::ChevronDown),
                    "展开下方".to_string(),
                    Message::Git(message::GitMessage::ExpandContext(
                        file.to_string(),
                        range.gap_id,
                        crate::app::message::git::ExpandDirection::Down,
                    )),
                ),
                small_plain_icon_button(
                    Some(Icon::ArrowsFullscreen),
                    "全部展开".to_string(),
                    Message::Git(message::GitMessage::ExpandContext(
                        file.to_string(),
                        range.gap_id,
                        crate::app::message::git::ExpandDirection::All,
                    )),
                ),
            ]
            .spacing(6)
            .align_y(iced::Alignment::Center),
        )
        .width(Length::Fill)
        .padding([3, 6])
        .style(gap_expand_container_style);
        let expand_btn: Element<'static, Message> = if app.merge_view {
            container(
                row![
                    line_marker_cell_emphasis(LineMarkerKind::None, false),
                    diff_line_select_spacer(),
                    empty_line_number_cell(),
                    expand_content
                ]
                .align_y(iced::Alignment::Center),
            )
            .width(Length::Fill)
            .into()
        } else {
            let left_part = diff_split_pane(
                container(
                    row![empty_line_number_cell(), container(text("")).width(Length::Fill)]
                        .width(Length::Fill),
                )
                .padding([0, 2])
                .into(),
                DiffSplitPaneTone::Empty,
                false,
            );
            let right_part = diff_split_pane(
                container(
                    row![empty_line_number_cell(), expand_content]
                        .spacing(4)
                        .align_y(iced::Alignment::Center)
                        .width(Length::Fill),
                )
                .padding([0, 2])
                .into(),
                DiffSplitPaneTone::Empty,
                false,
            );
            merge_diff_row(
                container(
                    row![
                        line_marker_cell_emphasis(LineMarkerKind::None, false),
                        left_part,
                        diff_split_divider(),
                        right_part
                    ]
                    .width(Length::Fill),
                )
                .width(Length::Fill)
                .into(),
                DiffSplitPaneTone::Empty,
                false,
            )
        };
        elements.push(expand_btn);

        // 渲染底部可见的行（从间隙末尾向前计算）
        for k in (gap_len.saturating_sub(vis_bottom))..gap_len {
            elements.push(render_line(k));
        }
    }

    elements
}
