//! 新建会话组件模块
//!
//! 本模块提供新建会话相关的 UI 组件，包括：
//! - 新建会话按钮：用于触发新建会话流程
//! - 会话选择器图层：用于选择工作区、创建新工作区等操作
//!
//! # 主要功能
//!
//! - 显示可用的 Git 工作区列表
//! - 支持创建新的 Git 工作区
//! - 支持重置和删除现有工作区
//! - 提供删除和重置的确认对话框
//! - 支持强制删除失败的工作区
//!
//! # 主题支持
//!
//! 组件支持亮色和暗色主题自动适配

use iced::widget::{
    Space, button, column, container, mouse_area, row, scrollable, text, text_input,
};
use iced::{Background, Color, Element, Length, Theme};

use desktop_assets::assets::{self, Icon};
use desktop_ui_controls::system_settings_common::{
    primary_action_btn_style, rounded_action_btn_style,
};

use display_format::truncate;
#[derive(Default)]
pub struct Input {
    pub new_session_picker_project: Option<String>,
    pub new_session_creating: bool,
    pub new_session_picker_options: Vec<(String, String)>,
    pub new_session_worktree_name: String,
    pub new_session_confirm_delete_directory: Option<String>,
    pub new_session_force_delete_directory: Option<String>,
    pub new_session_delete_error: Option<String>,
    pub new_session_confirm_reset_directory: Option<String>,
    pub new_session_reset_error: Option<String>,
    pub recent_projects: Vec<String>,
    pub recent_projects_edits: Vec<String>,
    pub window_size: (f32, f32),
    pub desktop_locale: config_ui::DesktopLocale,
}
#[derive(Debug, Clone)]
pub enum Message {
 None,
    ProjectCreateSession(String),
    ProjectCreateSessionDeleteWorktree(String),
    ProjectCreateSessionDeleteWorktreeCancel,
    ProjectCreateSessionDeleteWorktreeConfirmed,
    ProjectCreateSessionDeleteWorktreeForceConfirmed,
    ProjectCreateSessionPicked { project_path: String, directory: String },
    ProjectCreateSessionPickerClose,
    ProjectCreateSessionResetWorktree(String),
    ProjectCreateSessionResetWorktreeCancel,
    ProjectCreateSessionResetWorktreeConfirmed,
    ProjectCreateSessionWorktree(String),
    ProjectCreateSessionWorktreeNameChanged(String),
}


mod styles;
use styles::*;

fn worktree_display_name(directory: &str) -> &str {
    std::path::Path::new(directory).file_name().and_then(|s| s.to_str()).unwrap_or(directory)
}

fn project_display_title(
    path: &str,
    recent_projects: &[String],
    recent_project_edits: &[String],
) -> String {
    recent_projects
        .iter()
        .position(|p| p == path)
        .and_then(|i| recent_project_edits.get(i))
        .map(|name| if name.trim().is_empty() { path.to_owned() } else { name.to_owned() })
        .unwrap_or_else(|| path.to_owned())
}

fn create_session_message(path: &str) -> Message {
    Message::ProjectCreateSession(path.to_owned())
}

fn pick_session_message(project_path: &str, directory: &str) -> Message {
    Message::ProjectCreateSessionPicked {
        project_path: project_path.to_owned(),
        directory: directory.to_owned(),
    }
}

fn create_worktree_message(project_path: &str) -> Message {
    Message::ProjectCreateSessionWorktree(project_path.to_owned())
}

fn worktree_name_changed_message(value: String) -> Message {
    Message::ProjectCreateSessionWorktreeNameChanged(value)
}

fn delete_worktree_message(directory: &str) -> Message {
    Message::ProjectCreateSessionDeleteWorktree(
        directory.to_owned(),
    )
}

fn reset_worktree_message(directory: &str) -> Message {
    Message::ProjectCreateSessionResetWorktree(
        directory.to_owned(),
    )
}

/// 创建新建会话按钮
///
/// 生成一个用于触发新建会话流程的按钮组件。
/// 按钮显示加号图标和"新建会话"文字。
///
/// # 参数
///
/// - `app`: 应用状态的引用，用于检查当前活动状态
/// - `path`: 项目路径，用于标识项目
///
/// # 返回值
///
/// 返回一个 Iced Element，包含配置好的按钮组件
///
/// # 样式规则
///
/// - 悬停状态：弱背景色
/// - 按下状态：强背景色
/// - 活动状态（当前项目）：半透明弱背景色
/// - 默认状态：透明背景
/// - 使用圆角按钮样式，活动态带主色强调
///
/// # 交互
///
/// 点击按钮会发送 `ProjectCreateSession` 消息，打开会话选择器
pub fn new_session_button<'a>(app: &Input, path: String) -> Element<'a, Message> {
    let is_active = app.new_session_picker_project.as_ref() == Some(&path);
    let is_creating = app.new_session_creating;
    let mut create_button = button(
        container(
            row![
                iced::widget::svg::Svg::new(assets::get_icon(Icon::Plus))
                    .width(Length::Fixed(14.0))
                    .height(Length::Fixed(14.0))
                    .style(move |theme: &Theme, _status| iced::widget::svg::Style {
                        color: Some(if is_creating {
                            theme.palette().text.scale_alpha(0.35)
                        } else if is_active {
                            Color::WHITE
                        } else {
                            theme.palette().text
                        }),
                    }),
                text(desktop_i18n::I18n::new(app.desktop_locale).__(if is_creating {
                    "创建中..."
                } else {
                    "新建会话"
                }))
                .size(12)
                .style(move |theme: &Theme| text::Style {
                    color: is_creating.then(|| theme.palette().text.scale_alpha(0.35)),
                }),
            ]
            .align_y(iced::alignment::Vertical::Center)
            .spacing(6),
        )
        .width(Length::Fill)
        .align_x(iced::alignment::Horizontal::Center),
    )
    .width(Length::Fill)
    .padding([10, 14])
    .style(move |theme: &Theme, status| {
        let mut style = if is_active {
            primary_action_btn_style(theme, status)
        } else {
            rounded_action_btn_style(theme, status)
        };
        style.border.radius = 14.0.into();
        if is_creating {
            style.background = Some(Background::Color(
                theme.extended_palette().background.strong.color.scale_alpha(0.24),
            ));
            style.text_color = theme.palette().text.scale_alpha(0.35);
        }
        style.shadow = iced::Shadow {
            color: if is_creating {
                Color::TRANSPARENT
            } else if is_active {
                theme.extended_palette().primary.base.color.scale_alpha(0.18)
            } else {
                Color::BLACK.scale_alpha(if is_dark_theme(theme) { 0.10 } else { 0.04 })
            },
            offset: iced::Vector::new(0.0, 10.0),
            blur_radius: 20.0,
        };
        style
    });
    if !is_creating {
        create_button = create_button.on_press(create_session_message(&path));
    }
    create_button.into()
}

/// 创建新建会话选择器图层
///
/// 生成一个全屏覆盖的会话选择器组件，用于选择工作区或创建新工作区。
/// 该组件包含：
/// - 可用工作区列表
/// - 创建新工作区的输入框
/// - 工作区操作按钮（重置、删除）
/// - 确认对话框
///
/// # 参数
///
/// - `app`: 应用状态的引用，包含所有必要的状态信息
///
/// # 返回值
///
/// 返回一个 Iced Element，包含完整的选择器图层
///
/// # 组件结构
///
/// 1. **标题栏**：显示"创建会话"标题和项目名称
/// 2. **工作区列表**：显示所有可用的工作区选项
/// 3. **创建工作区**：输入框和创建按钮
/// 4. **错误提示**：显示删除和重置失败的错误信息
/// 5. **确认对话框**：删除、重置、强制删除的确认界面
///
/// # 交互
///
/// - 点击背景区域关闭选择器
/// - 点击工作区选项发送 `ProjectCreateSessionPicked` 消息
/// - 输入框变化发送 `ProjectCreateSessionWorktreeNameChanged` 消息
/// - 点击各种操作按钮发送对应的消息
pub fn new_session_picker_layer<'a>(app: &Input) -> Element<'a, Message> {
    // 如果没有选择项目，返回空容器
    let Some(path) = app.new_session_picker_project.clone() else {
        return container(Space::new()).width(Length::Fill).height(Length::Fill).into();
    };

    let mut picker_col: iced::widget::Column<'a, Message> = column![].spacing(6);

    // 如果选项列表为空，显示加载提示
    if app.new_session_picker_options.is_empty() {
        picker_col = picker_col.push(
            container(
                text(desktop_i18n::I18n::new(app.desktop_locale).__("加载中...")).size(12),
            )
            .padding([4, 6]),
        );
    } else {
        // 遍历所有工作区选项
        for (index, (directory, label)) in app.new_session_picker_options.iter().enumerate() {
            let display_label = desktop_i18n::project::worktree_label(
                desktop_i18n::I18n::new(app.desktop_locale),
                directory,
                label,
                index == 0,
            );
            let item: Element<'a, Message> = if directory == "__create_worktree__" {
                // 创建新工作区的特殊项
                column![
                    container(text(display_label.clone()).size(12))
                        .width(Length::Fill)
                        .padding([2, 6]),
                    // 工作区名称输入框
                    text_input(
                        desktop_i18n::I18n::new(app.desktop_locale).__("例如: feature-login"),
                        &app.new_session_worktree_name
                    )
                    .on_input(worktree_name_changed_message)
                    .padding([6, 8])
                    .size(12),
                    // 创建按钮
                    button(
                        container(
                            text(
                                desktop_i18n::I18n::new(app.desktop_locale)
                                    .__("创建并使用该工作区")
                            )
                            .size(12)
                        )
                        .width(Length::Fill)
                    )
                    .on_press(create_worktree_message(&path))
                    .style(create_worktree_button_style)
                    .width(Length::Fill)
                ]
                .spacing(4)
                .into()
            } else {
                // 现有工作区项
                let is_primary_workspace = index == 0;
                // 主工作区显示标签，其他工作区显示操作按钮
                let action_buttons: Element<'a, Message> = if is_primary_workspace {
                    container(
                        text(desktop_i18n::I18n::new(app.desktop_locale).__("主工作区"))
                            .size(10)
                            .style(|theme: &Theme| text::Style {
                                color: Some(theme.palette().text.scale_alpha(0.55)),
                            }),
                    )
                    .padding([2, 6])
                    .into()
                } else {
                    // 非主工作区显示重置和删除按钮
                    row![
                        button(
                            container(
                                text(desktop_i18n::I18n::new(app.desktop_locale).__("重置"))
                                    .size(10)
                            )
                            .padding([2, 8])
                        )
                        .on_press(reset_worktree_message(directory))
                        .style(worktree_action_button_style),
                        button(
                            container(
                                text(desktop_i18n::I18n::new(app.desktop_locale).__("删除"))
                                    .size(10)
                            )
                            .padding([2, 8])
                        )
                        .on_press(delete_worktree_message(directory))
                        .style(worktree_action_button_style),
                    ]
                    .spacing(6)
                    .into()
                };

                // 工作区选择按钮
                button(
                    container(
                        row![
                            container(text(display_label).size(12)).width(Length::Fill),
                            action_buttons,
                        ]
                        .align_y(iced::alignment::Vertical::Center)
                        .spacing(6),
                    )
                    .width(Length::Fill)
                    .padding([2, 6]),
                )
                .on_press(pick_session_message(&path, directory))
                .style(list_item_button_style)
                .width(Length::Fill)
                .into()
            };
            picker_col = picker_col.push(item);
        }
    }

    // 显示删除失败的错误信息
    if let Some(err) = app.new_session_delete_error.as_ref() {
        picker_col = picker_col.push(
            container(
                text(
                    desktop_i18n::__!(
                        desktop_i18n::I18n::new(app.desktop_locale),
                        "删除失败: %s",
                        err.clone()
                    )
                    .expect("valid input display template"),
                )
                .size(11)
                .style(|_: &Theme| text::Style { color: Some(Color::from_rgb8(220, 38, 38)) }),
            )
            .width(Length::Fill)
            .padding([4, 6]),
        );
    }

    // 显示重置失败的错误信息
    if let Some(err) = app.new_session_reset_error.as_ref() {
        picker_col = picker_col.push(
            container(
                text(
                    desktop_i18n::__!(
                        desktop_i18n::I18n::new(app.desktop_locale),
                        "重置失败: %s",
                        err.clone()
                    )
                    .expect("valid input display template"),
                )
                .size(11)
                .style(|_: &Theme| text::Style { color: Some(Color::from_rgb8(220, 38, 38)) }),
            )
            .width(Length::Fill)
            .padding([4, 6]),
        );
    }

    // 删除确认对话框
    if let Some(directory) = app.new_session_confirm_delete_directory.as_ref() {
        // 提取目录名称用于显示
        let name = worktree_display_name(directory);
        picker_col = picker_col.push(
            container(
                column![
                    text(
                        desktop_i18n::__!(
                            desktop_i18n::I18n::new(app.desktop_locale),
                            "确认删除工作区 %s ?",
                            format!("{}", name)
                        )
                        .expect("valid input display template")
                    )
                    .size(11),
                    row![
                        // 取消按钮
                        button(
                            container(
                                text(desktop_i18n::I18n::new(app.desktop_locale).__("取消"))
                                    .size(11)
                            )
                            .padding([2, 8])
                        )
                        .on_press(
                            Message::ProjectCreateSessionDeleteWorktreeCancel,
                        )
                        .style(neutral_button_style),
                        // 确认删除按钮
                        button(
                            container(
                                text(
                                    desktop_i18n::I18n::new(app.desktop_locale).__("确认删除")
                                )
                                .size(11)
                            )
                            .padding([2, 8])
                        )
                        .on_press(
                            Message::ProjectCreateSessionDeleteWorktreeConfirmed,
                        )
                        .style(danger_soft_button_style),
                    ]
                    .spacing(8)
                ]
                .spacing(6),
            )
            .padding([6, 8])
            .style(confirmation_panel_style),
        );
    }

    // 强制删除确认对话框（删除失败后显示）
    if let Some(directory) = app.new_session_force_delete_directory.as_ref() {
        // 提取目录名称用于显示
        let name = worktree_display_name(directory);
        picker_col = picker_col.push(
            container(
                column![
                    text(desktop_i18n::__!(desktop_i18n::I18n::new(app.desktop_locale), "删除失败，是否强制删除工作区 %s ?", format!("{}", name)).expect("valid input display template")).size(11),
                    row![
                        // 取消按钮
                        button(container(text(desktop_i18n::I18n::new(app.desktop_locale).__("取消")).size(11)).padding([2, 8]))
                            .on_press(
                                Message::ProjectCreateSessionDeleteWorktreeCancel,
                            )
                            .style(neutral_button_style),
                        // 强制删除按钮
                        button(container(text(desktop_i18n::I18n::new(app.desktop_locale).__("强制删除")).size(11)).padding([2, 8]))
                            .on_press(
                                Message::ProjectCreateSessionDeleteWorktreeForceConfirmed,
                            )
                            .style(danger_soft_button_style),
                    ]
                    .spacing(8)
                ]
                .spacing(6),
            )
            .padding([6, 8])
            .style(confirmation_panel_style),
        );
    }

    // 重置确认对话框
    if let Some(directory) = app.new_session_confirm_reset_directory.as_ref() {
        // 提取目录名称用于显示
        let name = worktree_display_name(directory);
        picker_col = picker_col.push(
            container(
                column![
                    text(
                        desktop_i18n::__!(
                            desktop_i18n::I18n::new(app.desktop_locale),
                            "确认重置工作区 %s ?",
                            format!("{}", name)
                        )
                        .expect("valid input display template")
                    )
                    .size(11),
                    // 显示重置操作的详细说明
                    text(
                        desktop_i18n::I18n::new(app.desktop_locale)
                            .__("将执行 git reset --hard <base-ref> 和 git clean -fd")
                    )
                    .size(10)
                    .style(|theme: &Theme| text::Style {
                        color: Some(theme.palette().text.scale_alpha(0.65)),
                    },),
                    row![
                        // 取消按钮
                        button(
                            container(
                                text(desktop_i18n::I18n::new(app.desktop_locale).__("取消"))
                                    .size(11)
                            )
                            .padding([2, 8])
                        )
                        .on_press(
                            Message::ProjectCreateSessionResetWorktreeCancel,
                        )
                        .style(neutral_button_style),
                        // 确认重置按钮
                        button(
                            container(
                                text(
                                    desktop_i18n::I18n::new(app.desktop_locale).__("确认重置")
                                )
                                .size(11)
                            )
                            .padding([2, 8])
                        )
                        .on_press(
                            Message::ProjectCreateSessionResetWorktreeConfirmed,
                        )
                        .style(worktree_action_button_style),
                    ]
                    .spacing(8)
                ]
                .spacing(6),
            )
            .padding([6, 8])
            .style(confirmation_panel_style),
        );
    }

    // 获取项目显示标题
    // 优先使用用户自定义的项目名称，如果为空则使用路径
    let title = project_display_title(&path, &app.recent_projects, &app.recent_projects_edits);

    // 主面板容器
    let picker_panel = container(
        column![
            // 标题栏
            row![
                column![
                    text(desktop_i18n::I18n::new(app.desktop_locale).__("创建会话"))
                        .size(14)
                        .font(iced::Font {
                            weight: iced::font::Weight::Bold,
                            ..Default::default()
                        }),
                    text(truncate(&title, 42)).size(11).style(|theme: &Theme| text::Style {
                        color: Some(theme.palette().text.scale_alpha(0.65)),
                    })
                ]
                .spacing(2)
                .width(Length::Fill),
                // 关闭按钮
                button(text(desktop_i18n::I18n::new(app.desktop_locale).__("关闭")).size(11))
                    .on_press(
                        Message::ProjectCreateSessionPickerClose,
                    )
                    .padding([4, 8])
                    .style(close_button_style)
            ]
            .align_y(iced::alignment::Vertical::Center),
            // 可滚动的工作区列表
            scrollable(picker_col).height(Length::Shrink)
        ]
        .spacing(10),
    )
    .width(Length::Fixed(360.0))
    .max_height((app.window_size.1 * 0.72).max(220.0)) // 最大高度为窗口高度的 72%
    .padding([10, 12])
    .style(picker_panel_style);

    // 全屏覆盖层
    // 点击背景关闭选择器，点击面板内容不关闭
    container(
        mouse_area(
            container(mouse_area(picker_panel).on_press(Message::None))
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(iced::alignment::Horizontal::Center)
                .align_y(iced::alignment::Vertical::Center),
        )
        .on_press(Message::ProjectCreateSessionPickerClose),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .style(overlay_style)
    .into()
}
#[cfg(test)]
#[path = "new_session_tests.rs"]
mod new_session_tests;
