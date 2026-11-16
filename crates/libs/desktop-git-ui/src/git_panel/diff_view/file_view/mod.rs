//! Single-file diff rendering and file actions.
use iced::widget::{Space, button, column, container, row, text};
use iced::{Background, Border, Color, Element, Length, Point};
use super::DiffRenderCtx;
use crate::git_panel::utils::{FileStatus, Lang, lang_for_file};
pub mod added_deleted;
pub mod body;
pub mod gaps;
pub mod hunk_ops;
pub mod standard;
pub mod start_end;
const DIFF_LINE_SELECT_WIDTH: f32 = 18.0;

pub fn diff_line_select_button<Message: Clone + 'static>(
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

pub fn diff_line_select_spacer<Message: 'static>() -> Element<'static, Message> {
    container(Space::new().width(Length::Fill).height(Length::Shrink))
        .width(Length::Fixed(DIFF_LINE_SELECT_WIDTH))
        .into()
}


#[cfg(test)]
mod added_deleted_tests;

#[cfg(test)]
mod gaps_tests;

#[cfg(test)]
mod standard_tests;

#[cfg(test)]
mod start_end_tests;

use desktop_assets::assets::Icon;
use crate::DiffTheme;
use super::DiffMessage as Message;
use crate::git_panel::ui::small_plain_icon_button;

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
    locale: config_ui::DesktopLocale,
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
            desktop_i18n::I18n::new(locale).__("查看"),
            Message::PreviewDiffFile(file.to_string()),
        ),
        menu_item(
            desktop_i18n::I18n::new(locale).__("复制"),
            Message::CopyDiffFile {
                file: file.to_string(),
                deleted_content
            },
        ),
    ]
    .spacing(0)
    .width(Length::Fixed(78.0));
    if !read_only {
        items = items.push(menu_item(
            desktop_i18n::I18n::new(locale).__("回滚"),
            Message::RevertDiffFile(file.to_string()),
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

/// Render one file using borrowed diff state and the host's expansion/menu projection.
/// History callers resolve their selected file before invoking this view.
pub fn view_file(
    app: &super::DiffViewState<'_>,
    expanded: bool,
    file_menu_open: bool,
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
                let groups = diff.grouped_ops(desktop_git::DIFF_CONTEXT);
                changed_diff_line_sets(&groups)
            }
            FileStatus::Unknown => {
                (std::collections::HashSet::new(), std::collections::HashSet::new())
            }
        };
        file_line_selection_state(render_ctx, file, &changed_line_sets)
    } else {
        let has_any_selected = render_ctx.selected_new_lines.iter().any(|(f, _)| *f == file)
            || render_ctx.selected_old_lines.iter().any(|(f, _)| *f == file);
        if has_any_selected { FileLineSelectionState::All } else { FileLineSelectionState::None }
    };

    let file_header: Element<'static, Message> = {
        let deleted_copy_content =
            matches!(status, FileStatus::Deleted).then(|| old_content.to_string());
        let file_menu_button = small_plain_icon_button(
            Some(Icon::DotsThreeVertical),
            desktop_i18n::I18n::new(app.desktop_locale).__("文件操作").to_owned(),
            Message::OpenDiffFileMenu(file.to_string()),
        );

        let (line_select_icon, line_select_tip, line_select_message) = match line_selection_state {
            FileLineSelectionState::None => (
                Some(Icon::Square),
                desktop_i18n::I18n::new(app.desktop_locale).__("选中所有行").to_owned(),
                Message::SelectAllFileLines(file.to_string()),
            ),
            FileLineSelectionState::Partial => (
                Some(Icon::SquareHalf),
                desktop_i18n::I18n::new(app.desktop_locale)
                    .__("部分选中，点击选中所有行")
                    .to_owned(),
                Message::SelectAllFileLines(file.to_string()),
            ),
            FileLineSelectionState::All => (
                Some(Icon::CheckSquare),
                desktop_i18n::I18n::new(app.desktop_locale).__("取消所有行").to_owned(),
                Message::ClearAllFileLines(file.to_string()),
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

        let file_menu = desktop_ui_controls::overlays::PointBelowOverlay::new(
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
        .on_close(Message::CloseDiffFileMenu)
        .capture_outside_click(false);

        let action_bar =
            row![file_menu].width(Length::Fixed(28.0)).align_y(iced::Alignment::Center);

        let expand_toggle = button(title_with_stats)
            .width(Length::FillPortion(1))
            .padding(0)
            .on_press(Message::ToggleExpandFile(file.to_string()))
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

#[cfg(test)]
mod mod_tests;
#[cfg(test)]
mod tests;
