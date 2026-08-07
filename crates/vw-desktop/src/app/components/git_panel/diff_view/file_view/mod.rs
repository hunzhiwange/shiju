//! Git 差异文件视图渲染模块
//!
//! 本模块负责渲染单个 Git 差异文件的完整视图，包括文件头部、差分行和交互控件。
//! 它是 Git 面板差异视图的核心组件，用于展示文件的增删改状态。
//!
//! # 主要功能
//!
//! - 渲染文件头部，包含文件路径、状态指示器和操作按钮
//! - 根据文件状态（新增、删除、修改等）渲染不同的差异内容
//! - 支持文件级别的暂存、回滚和预览操作
//! - 支持差异块（hunk）的展开/折叠
//! - 提供行级别的交互功能（选择、暂存等）
//!
//! # 子模块
//!
//! - `added_deleted`: 处理新增和删除文件的渲染逻辑
//! - `gaps`: 处理差异块之间的间隔区域渲染
//! - `hunk_ops`: 处理差异块的详细渲染
//! - `standard`: 处理标准差异行的渲染
//! - `start_end`: 定义起始和结束范围的数据结构

use iced::Point;
use iced::widget::{Space, button, column, container, row, text};
use iced::{Background, Border, Color, Element, Length};

use crate::app::assets::Icon;
use crate::app::{App, DiffTheme, Message, message};

use super::super::ui::small_plain_icon_button;
use super::super::utils::{FileStatus, Lang, lang_for_file};
use super::DiffRenderCtx;

mod added_deleted;
mod gaps;
mod hunk_ops;
mod standard;
mod start_end;

#[cfg(test)]
#[path = "added_deleted_tests.rs"]
mod added_deleted_tests;
#[cfg(test)]
#[path = "gaps_tests.rs"]
mod gaps_tests;
#[cfg(test)]
#[path = "mod_tests.rs"]
mod mod_tests;
#[cfg(test)]
#[path = "standard_tests.rs"]
mod standard_tests;
#[cfg(test)]
#[path = "start_end_tests.rs"]
mod start_end_tests;
#[cfg(test)]
mod tests;

const DIFF_LINE_SELECT_WIDTH: f32 = 18.0;

pub(super) fn diff_line_select_button(selected: bool, on: Message) -> Element<'static, Message> {
    let glyph = if selected { "✓" } else { "" };
    button(text(glyph).size(12))
        .on_press(on)
        .padding([2, 4])
        .width(Length::Fixed(DIFF_LINE_SELECT_WIDTH))
        .style(|theme: &iced::Theme, _status| iced::widget::button::Style {
            background: None,
            border: Border::default(),
            text_color: theme.extended_palette().background.strong.text,
            ..Default::default()
        })
        .into()
}

pub(super) fn diff_line_select_spacer() -> Element<'static, Message> {
    container(Space::new().width(Length::Fill).height(Length::Shrink))
        .width(Length::Fixed(DIFF_LINE_SELECT_WIDTH))
        .into()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FileLineSelectionState {
    None,
    Partial,
    All,
}

fn changed_diff_line_sets(
    groups: &[Vec<similar::DiffOp>],
) -> (std::collections::HashSet<usize>, std::collections::HashSet<usize>) {
    let mut old_lines = std::collections::HashSet::new();
    let mut new_lines = std::collections::HashSet::new();

    for group in groups {
        for op in group {
            match op {
                similar::DiffOp::Delete { old_index, old_len, .. } => {
                    for k in 0..*old_len {
                        old_lines.insert(*old_index + k);
                    }
                }
                similar::DiffOp::Insert { new_index, new_len, .. } => {
                    for k in 0..*new_len {
                        new_lines.insert(*new_index + k);
                    }
                }
                similar::DiffOp::Replace { old_index, old_len, new_index, new_len } => {
                    for k in 0..*old_len {
                        old_lines.insert(*old_index + k);
                    }
                    for k in 0..*new_len {
                        new_lines.insert(*new_index + k);
                    }
                }
                similar::DiffOp::Equal { .. } => {}
            }
        }
    }

    (old_lines, new_lines)
}

fn file_line_selection_state(
    _app: &App,
    render_ctx: &DiffRenderCtx<'_>,
    file: &str,
    changed_line_sets: &(std::collections::HashSet<usize>, std::collections::HashSet<usize>),
) -> FileLineSelectionState {
    let (old_changed, new_changed) = changed_line_sets;
    let total_changed = old_changed.len() + new_changed.len();

    if total_changed == 0 {
        return FileLineSelectionState::None;
    }

    if render_ctx.is_file_staged(file) {
        return FileLineSelectionState::All;
    }

    let selected_old =
        old_changed.iter().filter(|line| render_ctx.is_old_line_staged(file, **line)).count();
    let selected_new =
        new_changed.iter().filter(|line| render_ctx.is_new_line_staged(file, **line)).count();
    let selected_changed = selected_old + selected_new;

    if selected_changed == 0 {
        FileLineSelectionState::None
    } else if selected_changed >= total_changed {
        FileLineSelectionState::All
    } else {
        FileLineSelectionState::Partial
    }
}

fn diff_file_actions_menu(
    file: &str,
    deleted_content: Option<String>,
) -> Element<'static, Message> {
    let menu_button_style = |theme: &iced::Theme, status: iced::widget::button::Status| {
        let ext = theme.extended_palette();
        let text_color = match status {
            iced::widget::button::Status::Hovered => ext.primary.base.color,
            iced::widget::button::Status::Pressed => ext.primary.strong.color,
            _ => theme.palette().text,
        };
        let background = match status {
            iced::widget::button::Status::Pressed => {
                Some(Background::Color(ext.background.strong.color.scale_alpha(0.12)))
            }
            _ => None,
        };

        iced::widget::button::Style {
            background,
            border: Border { width: 0.0, color: Color::TRANSPARENT, radius: 5.0.into() },
            text_color,
            ..Default::default()
        }
    };

    let menu_item = |label: &'static str, message: Message| {
        button(
            container(text(label).size(12))
                .width(Length::Fill)
                .padding([5, 6])
                .align_x(iced::alignment::Horizontal::Left),
        )
        .width(Length::Fill)
        .padding(0)
        .style(menu_button_style)
        .on_press(message)
    };

    container(
        column![
            menu_item(
                "查看",
                Message::Git(message::GitMessage::PreviewDiffFile(file.to_string())),
            ),
            menu_item(
                "复制",
                Message::Git(message::GitMessage::CopyDiffFile {
                    file: file.to_string(),
                    deleted_content: deleted_content.clone(),
                }),
            ),
            menu_item(
                "回滚",
                Message::Git(message::GitMessage::RevertDiffFile(file.to_string())),
            ),
        ]
        .spacing(0)
        .width(Length::Fixed(78.0)),
    )
    .padding([3, 3])
    .style(|theme: &iced::Theme| {
        let ext = theme.extended_palette();
        let is_dark = theme.palette().background.r
            + theme.palette().background.g
            + theme.palette().background.b
            < 1.5;

        iced::widget::container::Style {
            background: Some(Background::Color(if is_dark {
                ext.background.base.color.scale_alpha(0.96)
            } else {
                ext.background.base.color.scale_alpha(0.99)
            })),
            border: Border {
                width: 1.0,
                color: ext.background.strong.color.scale_alpha(if is_dark { 0.52 } else { 0.74 }),
                radius: 10.0.into(),
            },
            shadow: iced::Shadow {
                color: Color::BLACK.scale_alpha(if is_dark { 0.20 } else { 0.08 }),
                offset: iced::Vector::new(0.0, 4.0),
                blur_radius: 10.0,
            },
            ..Default::default()
        }
    })
    .into()
}

/// 渲染单个 Git 差异文件的完整视图
///
/// 该函数是本模块的核心入口，负责构建文件的完整 UI 视图，包括：
/// - 文件头部（路径、统计信息、操作按钮）
/// - 差异内容（根据文件状态渲染不同的差异展示）
/// - 交互控件（暂存、回滚、预览等）
///
/// # 参数
///
/// - `app`: 应用状态引用，包含所有必要的应用数据
/// - `file`: 文件路径字符串
/// - `old_content`: 文件的旧版本内容（修改前）
/// - `new_content`: 文件的新版本内容（修改后）
/// - `status`: 文件的 Git 状态（新增、删除、修改等）
/// - `insertions`: 新增行数统计
/// - `deletions`: 删除行数统计
/// - `effective_theme`: 差异视图的主题配置
/// - `bg_default`: 默认背景颜色
/// - `add_line_bg`: 新增行的背景颜色
/// - `add_word_bg`: 新增词的背景颜色（用于行内差异高亮）
/// - `del_line_bg`: 删除行的背景颜色
/// - `del_word_bg`: 删除词的背景颜色（用于行内差异高亮）
/// - `is_modified`: 文件是否处于修改状态
///
/// # 返回值
///
/// 返回一个 `Element<'static, Message>`，包含完整的文件差异视图
///
/// # 示例
///
/// ```ignore
/// let file_view = view_file(
///     &app,
///     "src/main.rs",
///     old_content,
///     new_content,
///     FileStatus::Modified,
///     10,  // insertions
///     5,   // deletions
///     effective_theme,
///     bg_default,
///     add_line_bg,
///     add_word_bg,
///     del_line_bg,
///     del_word_bg,
///     true, // is_modified
/// );
/// ```
pub fn view_file(
    app: &App,
    render_ctx: &DiffRenderCtx<'_>,
    file: &str,
    contents: Option<(&str, &str)>,
    loading: bool,
    status: FileStatus,
    insertions: usize,
    deletions: usize,
    effective_theme: DiffTheme,
    bg_default: Color,
    add_line_bg: Color,
    add_word_bg: Color,
    del_line_bg: Color,
    del_word_bg: Color,
    is_modified: bool,
) -> Element<'static, Message> {
    let expanded = app.is_diff_file_expanded(file);
    let (old_content, new_content) = contents.unwrap_or(("", ""));
    let old_lines: Vec<&str> =
        if expanded { old_content.lines().collect() } else { Default::default() };
    let new_lines: Vec<&str> =
        if expanded { new_content.lines().collect() } else { Default::default() };
    let mut diff_groups = None;
    let line_selection_state = if contents.is_some() {
        let changed_line_sets = match status {
            FileStatus::Added | FileStatus::Untracked => {
                (std::collections::HashSet::new(), (0..new_content.lines().count()).collect())
            }
            FileStatus::Deleted => {
                ((0..old_content.lines().count()).collect(), std::collections::HashSet::new())
            }
            FileStatus::Modified | FileStatus::Renamed => {
                let diff = similar::TextDiff::from_lines(old_content, new_content);
                let groups = diff.grouped_ops(crate::app::git::DIFF_CONTEXT);
                let changed_line_sets = changed_diff_line_sets(&groups);
                diff_groups = Some(groups);
                changed_line_sets
            }
            FileStatus::Unknown => {
                (std::collections::HashSet::new(), std::collections::HashSet::new())
            }
        };
        file_line_selection_state(app, render_ctx, file, &changed_line_sets)
    } else {
        let has_any_selected = app.staged_lines_selected.iter().any(|(f, _)| f == file)
            || app.staged_old_lines_selected.iter().any(|(f, _)| f == file);
        if has_any_selected { FileLineSelectionState::All } else { FileLineSelectionState::None }
    };

    let allow_revert = matches!(
        status,
        FileStatus::Modified
            | FileStatus::Renamed
            | FileStatus::Added
            | FileStatus::Untracked
            | FileStatus::Deleted
    );
    let file_header: Element<'static, Message> = {
        let file_menu_open = app.git_diff_file_menu.as_ref().is_some_and(|menu| menu.file == file);
        let deleted_copy_content =
            matches!(status, FileStatus::Deleted).then(|| old_content.to_string());
        let file_menu_button = small_plain_icon_button(
            Some(Icon::DotsThreeVertical),
            "文件操作".to_string(),
            Message::Git(message::GitMessage::OpenDiffFileMenu(file.to_string())),
        );

        let (line_select_icon, line_select_tip, line_select_message) = match line_selection_state {
            FileLineSelectionState::None => (
                Some(Icon::Square),
                "选中所有行".to_string(),
                Message::Git(message::GitMessage::SelectAllFileLines(file.to_string())),
            ),
            FileLineSelectionState::Partial => (
                Some(Icon::SquareHalf),
                "部分选中，点击选中所有行".to_string(),
                Message::Git(message::GitMessage::SelectAllFileLines(file.to_string())),
            ),
            FileLineSelectionState::All => (
                Some(Icon::CheckSquare),
                "取消所有行".to_string(),
                Message::Git(message::GitMessage::ClearAllFileLines(file.to_string())),
            ),
        };
        let line_select_btn =
            small_plain_icon_button(line_select_icon, line_select_tip, line_select_message);

        let stat_badge = |value: String, positive: bool| {
            text(value).size(10).line_height(iced::widget::text::LineHeight::Relative(1.0)).style(
                move |theme: &iced::Theme| {
                    let ext = theme.extended_palette();
                    let color =
                        if positive { ext.success.base.color } else { ext.danger.base.color };
                    iced::widget::text::Style { color: Some(color) }
                },
            )
        };
        let stats = row![
            stat_badge(format!("+{}", insertions), true),
            stat_badge(format!("-{}", deletions), false)
        ]
        .spacing(3)
        .align_y(iced::Alignment::Center);

        let status_dot =
            container(Space::new().width(Length::Fixed(6.0)).height(Length::Fixed(6.0)))
                .width(Length::Fixed(6.0))
                .style(move |theme: &iced::Theme| {
                    let ext = theme.extended_palette();
                    let color = match status {
                        FileStatus::Deleted => ext.danger.base.color,
                        FileStatus::Added | FileStatus::Untracked => ext.success.base.color,
                        FileStatus::Modified | FileStatus::Renamed => ext.primary.base.color,
                        FileStatus::Unknown => ext.secondary.base.text,
                    };
                    iced::widget::container::Style {
                        background: Some(Background::Color(color.scale_alpha(0.92))),
                        border: Border {
                            width: 0.0,
                            color: Color::TRANSPARENT,
                            radius: 999.0.into(),
                        },
                        ..Default::default()
                    }
                });

        let title_with_stats = row![
            status_dot,
            container(
                text(file.to_string()).size(11).wrapping(iced::widget::text::Wrapping::None).style(
                    |theme: &iced::Theme| iced::widget::text::Style {
                        color: Some(theme.palette().text.scale_alpha(0.92)),
                    }
                ),
            )
            .width(Length::Fill)
            .padding([4, 6])
            .clip(true),
            container(stats).padding([0, 4]).width(Length::Shrink)
        ]
        .spacing(6)
        .width(Length::FillPortion(1))
        .align_y(iced::Alignment::Center);

        let menu_anchor_x = -54.0;
        let menu_anchor_y = 22.0;
        let menu_trigger = container(file_menu_button)
            .width(Length::Fixed(24.0))
            .height(Length::Fixed(24.0))
            .center_x(Length::Fill)
            .center_y(Length::Fill);

        let file_menu = crate::app::components::overlays::PointBelowOverlay::new(
            menu_trigger,
            diff_file_actions_menu(file, deleted_copy_content),
        )
        .show(file_menu_open)
        .anchor(Point::new(menu_anchor_x, menu_anchor_y))
        .gap(4.0)
        .on_close(Message::Git(message::GitMessage::CloseDiffFileMenu))
        .capture_outside_click(false);

        let action_bar =
            row![file_menu].spacing(6).width(Length::Shrink).align_y(iced::Alignment::Center);

        let expand_toggle = button(title_with_stats)
            .width(Length::FillPortion(1))
            .padding(0)
            .on_press(Message::Git(message::GitMessage::ToggleExpandFile(file.to_string())))
            .style(|_theme: &iced::Theme, _status| iced::widget::button::Style {
                background: None,
                border: Border::default(),
                text_color: Color::TRANSPARENT,
                ..Default::default()
            });

        let content =
            row![container(line_select_btn).width(Length::Shrink), expand_toggle, action_bar]
                .spacing(8)
                .width(Length::Fill)
                .align_y(iced::Alignment::Center);

        // 根据文件状态设置头部的背景色和边框色
        let header = container(content).width(Length::Fill).padding([4, 4]).style(move |theme| {
            let ext = theme.extended_palette();
            let is_dark = theme.palette().background.r
                + theme.palette().background.g
                + theme.palette().background.b
                < 1.5;
            let neutral_bg = if is_dark {
                ext.background.weak.color.scale_alpha(0.20)
            } else {
                ext.background.base.color.scale_alpha(0.98)
            };
            let neutral_border =
                ext.background.strong.color.scale_alpha(if is_dark { 0.28 } else { 0.18 });
            let (bg, border_color) = match status {
                FileStatus::Deleted => (
                    ext.danger.base.color.scale_alpha(if is_dark { 0.08 } else { 0.05 }),
                    neutral_border,
                ),
                FileStatus::Added | FileStatus::Untracked => (
                    ext.success.base.color.scale_alpha(if is_dark { 0.08 } else { 0.05 }),
                    neutral_border,
                ),
                FileStatus::Modified | FileStatus::Renamed => (
                    ext.primary.base.color.scale_alpha(if is_dark { 0.07 } else { 0.04 }),
                    neutral_border,
                ),
                _ => (neutral_bg, neutral_border),
            };
            iced::widget::container::Style {
                text_color: None,
                background: Some(Background::Color(bg)),
                border: Border { width: 1.0, color: border_color, radius: 10.0.into() },
                shadow: iced::Shadow::default(),
                snap: false,
            }
        });

        header.into()
    };

    // ========== 构建差异内容区域 ==========
    let mut list: Vec<Element<'static, Message>> = Vec::new();
    list.push(file_header);

    if expanded {
        if loading || contents.is_none() {
            let placeholder =
                if loading { "正在加载文件内容..." } else { "文件内容暂不可用" };
            list.push(
                container(text(placeholder).size(12)).width(Length::Fill).padding([10, 12]).into(),
            );
            return container(
                iced::widget::Column::with_children(list).spacing(0).width(Length::Fill),
            )
            .width(Length::Fill)
            .into();
        }

        // 确定文件的语言类型（用于语法高亮）
        // 新增、未跟踪、删除的文件不进行语法高亮
        let lang =
            if matches!(status, FileStatus::Added | FileStatus::Untracked | FileStatus::Deleted) {
                Lang::Other
            } else {
                lang_for_file(file)
            };

        // 悬停高亮颜色设置
        let hover_color = Color::from_rgba8(255, 210, 0, 1.0);
        let hover_mix: f32 = 0.22;
        let hover_alpha: f32 = 0.22;
        let hover_tint = Color::from_rgba(hover_color.r, hover_color.g, hover_color.b, hover_alpha);

        // 检查是否有选中的行（影响某些交互行为）
        // ========== 处理新增文件（纯绿色显示） ==========
        if matches!(status, FileStatus::Added | FileStatus::Untracked) {
            let col = added_deleted::render_added_lines(
                app,
                render_ctx,
                file,
                &new_lines,
                lang,
                effective_theme,
                add_line_bg,
                add_word_bg,
                hover_color,
                hover_mix,
                hover_tint,
                false,
                allow_revert,
            );
            list.push(container(col).width(Length::Fill).padding([4, 0]).into());
            return container(
                iced::widget::Column::with_children(list).spacing(0).width(Length::Fill),
            )
            .width(Length::Fill)
            .into();
        }

        // ========== 处理删除文件（纯红色显示） ==========
        if matches!(status, FileStatus::Deleted) {
            let col = added_deleted::render_deleted_lines(
                app,
                render_ctx,
                file,
                &old_lines,
                lang,
                effective_theme,
                del_line_bg,
                hover_color,
                hover_mix,
                hover_tint,
                false,
                allow_revert,
            );
            list.push(container(col).width(Length::Fill).padding([4, 0]).into());
            return container(
                iced::widget::Column::with_children(list).spacing(0).width(Length::Fill),
            )
            .width(Length::Fill)
            .into();
        }

        // ========== 处理修改/重命名文件（标准差异视图） ==========
        let groups = diff_groups.as_deref().unwrap_or(&[]);

        // 检查是否有手动展开的差异块
        let has_hunk_manual_expanded = app.expanded_hunks.iter().any(|(f, _)| f.as_str() == file);

        // 自动展开预算：小于等于这个行数的差异块会自动展开
        let mut auto_expanded_budget: usize = 500;

        // 追踪上一个差异块结束的行号（用于计算间隙区域）
        let mut last_old_end = 0;
        let mut last_new_end = 0;

        // 遍历每个差异块（hunk）并渲染
        for (i, group) in groups.iter().enumerate() {
            // 估算当前差异块的行数（用于决定是否自动展开）
            let group_line_estimate = group.iter().fold(0usize, |acc, op| match op {
                similar::DiffOp::Equal { len, .. } => acc + len,
                similar::DiffOp::Delete { old_len, .. } => acc + old_len,
                similar::DiffOp::Insert { new_len, .. } => acc + new_len,
                similar::DiffOp::Replace { old_len, new_len, .. } => acc + old_len + new_len,
            });

            // 判断差异块是否展开
            // 展开条件：手动展开 或 （没有手动展开过且行数在预算内）
            let hunk_expanded = app
                .expanded_hunks
                .iter()
                .any(|(f, idx)| f.as_str() == file && *idx == i)
                || (!has_hunk_manual_expanded && group_line_estimate <= auto_expanded_budget && {
                    auto_expanded_budget = auto_expanded_budget.saturating_sub(group_line_estimate);
                    true
                });

            // 计算当前差异块在旧文件和新文件中的范围
            let mut current_old_start = usize::MAX;
            let mut current_old_end = last_old_end;
            let mut current_new_end = last_new_end;

            // 遍历差异块中的所有操作，确定行号范围
            for op in group.iter() {
                match op {
                    similar::DiffOp::Equal { old_index, new_index, len } => {
                        current_old_start = current_old_start.min(*old_index);
                        current_old_end = current_old_end.max(old_index + *len);
                        current_new_end = current_new_end.max(new_index + *len);
                    }
                    similar::DiffOp::Delete { old_index, old_len, new_index } => {
                        current_old_start = current_old_start.min(*old_index);
                        current_old_end = current_old_end.max(old_index + *old_len);
                        current_new_end = current_new_end.max(*new_index);
                    }
                    similar::DiffOp::Insert { new_index, new_len, .. } => {
                        current_new_end = current_new_end.max(new_index + *new_len);
                    }
                    similar::DiffOp::Replace { old_index, old_len, new_index, new_len } => {
                        current_old_start = current_old_start.min(*old_index);
                        current_old_end = current_old_end.max(old_index + *old_len);
                        current_new_end = current_new_end.max(new_index + *new_len);
                    }
                }
            }

            // 如果没有找到起始位置（例如只有插入操作），使用上一个块的结束位置
            if current_old_start == usize::MAX {
                current_old_start = last_old_end;
            }
            // 渲染差异块之间的间隙区域（折叠的内容）
            let gap_elems = gaps::render_gap(
                app,
                render_ctx,
                file,
                &old_lines,
                start_end::GapRange {
                    start_old: last_old_end,
                    end_old: current_old_start,
                    start_new: last_new_end,
                    gap_id: i * 2,
                },
                lang,
                effective_theme,
                bg_default,
                hover_tint,
                false,
            );
            for elem in gap_elems {
                list.push(elem);
            }

            // 创建差异块列容器
            let mut col = column![].spacing(0);

            // 如果差异块未展开，显示折叠摘要
            if !hunk_expanded {
                col = col.push(
                    container(
                        text(format!("已折叠 {} 行差异", group_line_estimate)).size(12).style(
                            |theme: &iced::Theme| iced::widget::text::Style {
                                color: Some(
                                    theme.extended_palette().secondary.base.text.scale_alpha(0.80),
                                ),
                            },
                        ),
                    )
                    .width(Length::Fill)
                    .padding([6, 10])
                    .style(|theme: &iced::Theme| {
                        let is_dark = theme.palette().background.r
                            + theme.palette().background.g
                            + theme.palette().background.b
                            < 1.5;
                        let ext = theme.extended_palette();
                        iced::widget::container::Style {
                            background: Some(Background::Color(if is_dark {
                                ext.background.weak.color.scale_alpha(0.40)
                            } else {
                                ext.background.weak.color.scale_alpha(0.88)
                            })),
                            border: Border {
                                width: 1.0,
                                color: ext.background.strong.color.scale_alpha(if is_dark {
                                    0.28
                                } else {
                                    0.14
                                }),
                                radius: 8.0.into(),
                            },
                            ..Default::default()
                        }
                    }),
                );
                list.push(col.into());

                // 更新最后一个差异块的结束位置
                last_old_end = current_old_end;
                last_new_end = current_new_end;
                continue;
            }

            // 差异块已展开，渲染详细的差异操作
            let hunk_col = hunk_ops::render_hunk_ops(
                app,
                render_ctx,
                file,
                group,
                i,
                &old_lines,
                &new_lines,
                lang,
                effective_theme,
                bg_default,
                add_line_bg,
                add_word_bg,
                del_line_bg,
                del_word_bg,
                hover_color,
                hover_mix,
                hover_tint,
                false,
                is_modified,
            );

            // 将差异操作行添加到列容器
            for elem in hunk_col {
                col = col.push(elem);
            }

            // 更新最后一个差异块的结束位置
            last_old_end = current_old_end;
            last_new_end = current_new_end;

            list.push(container(col).padding([4, 0]).into());
        }

        // 渲染最后一个差异块之后到文件末尾的间隙区域
        let gap_elems = gaps::render_gap(
            app,
            render_ctx,
            file,
            &old_lines,
            start_end::GapRange {
                start_old: last_old_end,
                end_old: old_lines.len(),
                start_new: last_new_end,
                gap_id: groups.len(),
            },
            lang,
            effective_theme,
            bg_default,
            hover_tint,
            false,
        );
        for elem in gap_elems {
            list.push(elem);
        }
    }

    // 返回完整的文件差异视图（包含头部和差异内容）
    container(iced::widget::Column::with_children(list).spacing(0)).width(Length::Fill).into()
}
