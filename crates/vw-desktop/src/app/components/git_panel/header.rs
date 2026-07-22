//! Git 面板头部组件
//!
//! 该模块提供 Git 面板的头部界面渲染功能，包含分支显示和操作按钮。
//!
//! # 主要功能
//!
//! - 显示当前分支名称
//! - 展开/折叠全部文件列表
//! - 显示/隐藏摘要输入框
//! - 过滤选项按钮
//!
//! # 交互特性
//!
//! - 顶部操作按钮始终可见
//! - 保持紧凑间距，避免右侧按钮贴边

use iced::Element;
use iced::alignment::Vertical;
use iced::widget::tooltip::{Position as TooltipPosition, Tooltip};
use iced::widget::{Space, button, column, container, row, text};
use iced::{Background, Border, Color, Length, Point};

use crate::app::assets::Icon;
use crate::app::components::overlays::PointBelowOverlay;
use crate::app::components::system_settings_common::icon_svg;
use crate::app::{App, Message, message};

use super::ui::small_plain_icon_button;

const WORKTREE_TRIGGER_WIDTH: f32 = 90.0;
const WORKTREE_MENU_WIDTH: f32 = 300.0;
const WORKTREE_TRIGGER_LABEL_CHARS: usize = 7;
const WORKTREE_MENU_LABEL_CHARS: usize = 34;
const WORKTREE_BRANCH_LABEL_CHARS: usize = 10;
const HEADER_BRANCH_LABEL_CHARS: usize = 8;

pub(super) fn truncate_label(value: &str, max_chars: usize) -> String {
    let count = value.chars().count();
    if count <= max_chars {
        return value.to_string();
    }

    let keep = max_chars.saturating_sub(1);
    let mut out = value.chars().take(keep).collect::<String>();
    out.push('…');
    out
}

pub(super) fn trigger_label(value: &str) -> String {
    let primary = value.split('·').next().unwrap_or(value).trim();
    truncate_label(primary, WORKTREE_TRIGGER_LABEL_CHARS)
}

pub(super) fn worktree_option_display_label(
    option: &crate::app::state::GitWorktreeOption,
) -> String {
    let primary = option.label.split('·').next().unwrap_or(&option.label).trim();
    if let Some(branch) = option.branch.as_deref().filter(|value| !value.trim().is_empty()) {
        format!("{primary} · {}", truncate_label(branch.trim(), WORKTREE_BRANCH_LABEL_CHARS))
    } else {
        primary.to_string()
    }
}

pub(super) fn selected_worktree_trigger_label(app: &App) -> String {
    app.selected_git_worktree_directory
        .as_ref()
        .and_then(|directory| {
            app.git_worktree_options
                .iter()
                .find(|option| option.directory.as_str() == directory.as_str())
        })
        .map(|option| trigger_label(&option.label))
        .unwrap_or_else(|| "工作区".to_string())
}

pub(super) fn summary_toggle_icon_and_tip(show_summary: bool) -> (Icon, String) {
    if show_summary {
        (Icon::EyeSlash, "隐藏摘要输入框".to_string())
    } else {
        (Icon::Eye, "显示摘要输入框".to_string())
    }
}

pub(super) fn fullscreen_icon_and_tip(is_fullscreen: bool) -> (Icon, String) {
    if is_fullscreen {
        (Icon::FullscreenExit, "退出全屏".to_string())
    } else {
        (Icon::Fullscreen, "全屏".to_string())
    }
}

pub(super) fn worktree_menu_toggle_message(is_open: bool) -> Message {
    Message::Git(message::GitMessage::ToggleGitWorktreeMenu(!is_open))
}

pub(super) fn filter_options_toggle_message(is_open: bool) -> Message {
    Message::Git(message::GitMessage::ToggleFilterOptions(!is_open))
}

fn tooltip_content(label: String) -> Element<'static, Message> {
    container(text(label).size(12))
        .padding([5, 8])
        .style(|theme: &iced::Theme| {
            let ext = theme.extended_palette();
            iced::widget::container::Style {
                text_color: Some(ext.background.strong.text),
                background: Some(Background::Color(ext.background.base.color)),
                border: Border {
                    width: 1.0,
                    color: ext.background.strong.color,
                    radius: 6.0.into(),
                },
                ..Default::default()
            }
        })
        .into()
}

pub(super) fn worktree_button_style(
    theme: &iced::Theme,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let ext = theme.extended_palette();
    let background = match status {
        iced::widget::button::Status::Hovered => Some(ext.background.weak.color.into()),
        iced::widget::button::Status::Pressed => Some(ext.background.strong.color.into()),
        _ => Some(ext.background.base.color.into()),
    };
    iced::widget::button::Style {
        background,
        border: Border {
            width: 1.0,
            color: ext.background.strong.color.scale_alpha(0.55),
            radius: 10.0.into(),
        },
        text_color: theme.palette().text,
        ..Default::default()
    }
}

fn worktree_menu<'a>(app: &App) -> Element<'a, Message> {
    let selected_directory = app.selected_git_worktree_directory.as_deref();
    let item_style = |selected: bool| {
        move |theme: &iced::Theme, status: iced::widget::button::Status| {
            let ext = theme.extended_palette();
            let background = if selected {
                Some(ext.primary.weak.color.scale_alpha(0.18).into())
            } else {
                match status {
                    iced::widget::button::Status::Hovered => {
                        Some(ext.background.weak.color.scale_alpha(0.75).into())
                    }
                    iced::widget::button::Status::Pressed => {
                        Some(ext.background.strong.color.scale_alpha(0.35).into())
                    }
                    _ => None,
                }
            };
            iced::widget::button::Style {
                background,
                border: Border { width: 0.0, color: Color::TRANSPARENT, radius: 7.0.into() },
                text_color: if selected { ext.primary.base.color } else { theme.palette().text },
                ..Default::default()
            }
        }
    };

    let mut items = column![].spacing(2).width(Length::Fixed(WORKTREE_MENU_WIDTH));
    for option in &app.git_worktree_options {
        let selected =
            selected_directory.is_some_and(|directory| option.directory.as_str() == directory);
        let display_label =
            truncate_label(&worktree_option_display_label(option), WORKTREE_MENU_LABEL_CHARS);
        let label = if selected { format!("✓ {}", display_label) } else { display_label };
        let row_button = button(
            container(
                text(label)
                    .size(13)
                    .height(Length::Fill)
                    .align_y(Vertical::Center)
                    .wrapping(iced::widget::text::Wrapping::None),
            )
            .height(Length::Fixed(30.0))
            .width(Length::Fill)
            .padding([0, 8])
            .align_x(iced::alignment::Horizontal::Left),
        )
        .width(Length::Fill)
        .padding(0)
        .style(item_style(selected))
        .on_press(Message::Git(message::GitMessage::SelectGitWorktree(option.clone())));
        let row =
            Tooltip::new(row_button, tooltip_content(option.label.clone()), TooltipPosition::Right)
                .gap(8);
        items = items.push(row);
    }

    container(items)
        .padding([5, 5])
        .width(Length::Fixed(WORKTREE_MENU_WIDTH))
        .style(|theme: &iced::Theme| {
            let ext = theme.extended_palette();
            iced::widget::container::Style {
                background: Some(Background::Color(ext.background.base.color)),
                border: Border {
                    width: 1.0,
                    color: ext.background.strong.color.scale_alpha(0.70),
                    radius: 12.0.into(),
                },
                shadow: iced::Shadow {
                    color: Color::BLACK.scale_alpha(0.18),
                    offset: iced::Vector::new(0.0, 8.0),
                    blur_radius: 18.0,
                },
                ..Default::default()
            }
        })
        .into()
}

/// 渲染 Git 面板的头部界面
///
/// 该函数构建 Git 面板头部的完整 UI，包括分支名称和一系列操作按钮。
/// 操作按钮始终显示，方便直接切换常用视图和过滤能力。
///
/// # 参数
///
/// - `app`: 应用状态引用，用于读取当前分支、视图模式等状态
/// - `files_list`: 文件列表，用于判断是否全部展开/折叠
///
/// # 返回值
///
/// 返回构建好的 Element，包含完整的头部 UI 组件
///
/// # 界面元素
///
/// 1. **分支按钮**: 显示当前分支名称，点击打开终端
/// 2. **全选按钮**: 选中当前列表中的所有变更行
/// 3. **反选按钮**: 反选当前列表中的所有变更行
/// 4. **摘要输入框开关**: 显示或隐藏 Git diff 摘要输入框
/// 5. **过滤选项**: 打开或关闭 Git 过滤选项面板
///
pub fn view(app: &App, files_list: Vec<String>) -> Element<'_, Message> {
    // 获取当前分支名称，如果未选择则显示 "-"
    let full_branch_name = app.selected_branch.clone().unwrap_or_else(|| "-".to_string());
    let branch_name = truncate_label(&full_branch_name, HEADER_BRANCH_LABEL_CHARS);

    // 构建分支按钮，显示当前分支名称
    // 点击时发送打开终端的消息
    let branch_button = button(
        text(branch_name)
            .size(13)
            .height(Length::Fill)
            .align_y(Vertical::Center)
            .wrapping(iced::widget::text::Wrapping::None),
    )
    .on_press(Message::View(message::ViewMessage::OpenTerminalPressed))
    .height(Length::Fixed(21.0))
    .padding([0, 10])
    .style(|theme: &iced::Theme, status| {
        let palette = theme.extended_palette();
        // 根据按钮状态（悬停、按下、默认）设置不同的背景色
        let bg = match status {
            iced::widget::button::Status::Hovered => Some(palette.background.weak.color.into()),
            iced::widget::button::Status::Pressed => Some(palette.background.strong.color.into()),
            _ => None,
        };
        iced::widget::button::Style {
            background: bg,
            border: Border { width: 0.0, color: iced::Color::TRANSPARENT, radius: 8.0.into() },
            text_color: theme.palette().text,
            ..Default::default()
        }
    });
    let branch: Element<'_, Message> =
        Tooltip::new(branch_button, tooltip_content(full_branch_name), TooltipPosition::Top)
            .gap(6)
            .into();

    let worktree_picker: Option<Element<'_, Message>> =
        (app.git_worktree_options.len() > 1).then(|| {
            let selected_label = selected_worktree_trigger_label(app);
            let trigger = button(
                container(
                    row![
                        text(selected_label)
                            .size(12)
                            .height(Length::Fill)
                            .align_y(Vertical::Center)
                            .width(Length::Fill),
                        icon_svg(Icon::ChevronDown, 11.0)
                    ]
                    .spacing(4)
                    .align_y(iced::Alignment::Center),
                )
                .height(Length::Fill)
                .center_y(Length::Fill),
            )
            .on_press(worktree_menu_toggle_message(app.git_worktree_menu_open))
            .height(Length::Fixed(26.0))
            .width(Length::Fixed(WORKTREE_TRIGGER_WIDTH))
            .padding([0, 7])
            .style(worktree_button_style);

            PointBelowOverlay::new(trigger, worktree_menu(app))
                .show(app.git_worktree_menu_open)
                .anchor(Point::new(0.0, 28.0))
                .gap(2.0)
                .on_close(Message::Git(message::GitMessage::ToggleGitWorktreeMenu(false)))
                .into()
        });

    let select_all_btn = small_plain_icon_button(
        Some(Icon::CheckSquare),
        "全选当前列表的变更行".to_string(),
        Message::Git(message::GitMessage::SelectAllVisibleFileLines(files_list.clone())),
    );

    let invert_select_btn = small_plain_icon_button(
        Some(Icon::Square),
        "反选当前列表的变更行".to_string(),
        Message::Git(message::GitMessage::InvertVisibleFileLines(files_list.clone())),
    );

    // 构建摘要输入框显示/隐藏切换按钮
    // 用于控制 Git diff 摘要输入框的可见性
    let summary_toggle = {
        // 根据当前显示状态选择图标和提示文本
        let (icon, tip) = summary_toggle_icon_and_tip(app.show_git_diff_summary);
        small_plain_icon_button(
            Some(icon),
            tip,
            Message::View(message::ViewMessage::ToggleGitDiffSummary),
        )
    };

    // 构建过滤选项按钮
    // 用于打开或关闭 Git 文件过滤选项面板
    let filter_btn = small_plain_icon_button(
        Some(Icon::Sliders),
        "过滤选项".to_string(),
        filter_options_toggle_message(app.show_git_filter_options),
    );

    let half_fullscreen_btn = small_plain_icon_button(
        Some(Icon::LayoutTextWindow),
        "半屏".to_string(),
        Message::Git(message::GitMessage::ToggleHalfFullscreen),
    );

    let (fullscreen_icon, fullscreen_tip) = fullscreen_icon_and_tip(app.git_diff_fullscreen);
    let fullscreen_btn = small_plain_icon_button(
        Some(fullscreen_icon),
        fullscreen_tip,
        Message::Git(message::GitMessage::ToggleFullscreen),
    );

    let actions: Element<'_, Message> = row![
        row![
            select_all_btn,
            invert_select_btn,
            summary_toggle,
            filter_btn,
            half_fullscreen_btn,
            fullscreen_btn
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center),
        Space::new().width(Length::Fixed(10.0))
    ]
    .align_y(iced::Alignment::Center)
    .into();

    // 构建头部内容行：分支名称 + 弹性空间 + 操作按钮
    let leading: Element<'_, Message> = if let Some(worktree_picker) = worktree_picker {
        row![worktree_picker, branch].spacing(8).align_y(iced::Alignment::Center).into()
    } else {
        branch
    };

    let content = row![leading, Space::new().width(Length::Fill), actions]
        .spacing(10)
        .align_y(iced::Alignment::Center);

    // 为内容添加容器样式，设置背景色
    let content = container(content).style(|_theme: &iced::Theme| iced::widget::container::Style {
        background: None,
        ..Default::default()
    });

    // 用 MouseArea 包裹内容，实现悬停显示/隐藏操作按钮的交互
    iced::widget::MouseArea::new(content)
        .on_enter(Message::Git(message::GitMessage::HoverGitPanelHeaderEnter))
        .on_exit(Message::Git(message::GitMessage::HoverGitPanelHeaderExit))
        .into()
}
