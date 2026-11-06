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
mod body;
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

pub(super) fn diff_line_select_button(
    selected: bool,
    on: Message,
    read_only: bool,
) -> Element<'static, Message> {
    if read_only {
        // 分栏 diff 的行布局依赖控件的固有高度；隐藏暂存框时仍保留行高。
        return container(Space::new().height(20)).width(DIFF_LINE_SELECT_WIDTH).into();
    }
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
    locale: vw_config_types::ui::DesktopLocale,
    file: &str,
    deleted_content: Option<String>,
    read_only: bool,
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

    let mut items = column![
        menu_item(
            crate::app::i18n::I18n::new(locale).__("查看"),
            Message::Git(message::GitMessage::PreviewDiffFile(file.to_string())),
        ),
        menu_item(
            crate::app::i18n::I18n::new(locale).__("复制"),
            Message::Git(message::GitMessage::CopyDiffFile {
                file: file.to_string(),
                deleted_content
            }),
        ),
    ]
    .spacing(0)
    .width(Length::Fixed(78.0));
    if !read_only {
        items = items.push(menu_item(
            crate::app::i18n::I18n::new(locale).__("回滚"),
            Message::Git(message::GitMessage::RevertDiffFile(file.to_string())),
        ));
    }
    container(items)
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
                    color: ext.background.strong.color.scale_alpha(if is_dark {
                        0.52
                    } else {
                        0.74
                    }),
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
    let expanded = render_ctx.history_file.as_ref().map_or_else(
        || app.is_diff_file_expanded(file),
        |(index, _)| app.git_repository.selected_file == Some(*index),
    );
    let (old_content, new_content) = contents.unwrap_or(("", ""));
    let line_selection_state = if render_ctx.read_only {
        FileLineSelectionState::None
    } else if contents.is_some() {
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
                changed_diff_line_sets(&groups)
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

    let file_header: Element<'static, Message> = {
        let file_menu_open = render_ctx.history_file.as_ref().map_or_else(
            || app.git_diff_file_menu.as_ref().is_some_and(|menu| menu.file == file),
            |(index, _)| app.git_repository.file_menu == Some(*index),
        );
        let deleted_copy_content =
            matches!(status, FileStatus::Deleted).then(|| old_content.to_string());
        let file_menu_button = small_plain_icon_button(
            Some(Icon::DotsThreeVertical),
            crate::app::i18n::I18n::new(app.desktop_locale).__("文件操作").to_owned(),
            Message::Git(message::GitMessage::OpenDiffFileMenu(file.to_string())),
        );

        let (line_select_icon, line_select_tip, line_select_message) = match line_selection_state {
            FileLineSelectionState::None => (
                Some(Icon::Square),
                crate::app::i18n::I18n::new(app.desktop_locale).__("选中所有行").to_owned(),
                Message::Git(message::GitMessage::SelectAllFileLines(file.to_string())),
            ),
            FileLineSelectionState::Partial => (
                Some(Icon::SquareHalf),
                crate::app::i18n::I18n::new(app.desktop_locale)
                    .__("部分选中，点击选中所有行")
                    .to_owned(),
                Message::Git(message::GitMessage::SelectAllFileLines(file.to_string())),
            ),
            FileLineSelectionState::All => (
                Some(Icon::CheckSquare),
                crate::app::i18n::I18n::new(app.desktop_locale).__("取消所有行").to_owned(),
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
        .spacing(10)
        .width(Length::FillPortion(1))
        .align_y(iced::Alignment::Center);

        let menu_anchor_x = -54.0;
        let menu_anchor_y = 22.0;
        let menu_trigger =
            container(file_menu_button).center_x(Length::Fixed(28.0)).center_y(Length::Fixed(28.0));

        let file_menu = crate::app::components::overlays::PointBelowOverlay::new(
            menu_trigger,
            diff_file_actions_menu(
                app.desktop_locale,
                file,
                deleted_copy_content,
                render_ctx.read_only,
            ),
        )
        .show(file_menu_open)
        .anchor(Point::new(menu_anchor_x, menu_anchor_y))
        .gap(4.0)
        .on_close(Message::Git(message::GitMessage::CloseDiffFileMenu))
        .capture_outside_click(false);

        let action_bar =
            row![file_menu].width(Length::Fixed(28.0)).align_y(iced::Alignment::Center);

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

        let mut content = row![];
        if !render_ctx.read_only {
            content = content.push(container(line_select_btn).width(Length::Shrink));
        }
        let content = content
            .push(expand_toggle)
            .push(action_bar)
            .spacing(8)
            .width(Length::Fill)
            .align_y(iced::Alignment::Center);

        // 根据文件状态设置头部的背景色和边框色
        let header = container(content).width(Length::Fill).padding([4, 10]).style(move |theme| {
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

    if expanded {
        column![
            file_header,
            body::view_file_body(
                app,
                render_ctx,
                render_ctx.history_file.as_ref().map_or(file, |(_, key)| key.as_str()),
                contents,
                loading,
                status,
                effective_theme,
                bg_default,
                add_line_bg,
                add_word_bg,
                del_line_bg,
                del_word_bg,
                is_modified
            )
        ]
        .spacing(0)
        .width(Length::Fill)
        .into()
    } else {
        file_header
    }
}
