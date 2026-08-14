//! 应用视图模块
//!
//! 本模块为 `App` 结构体实现了主视图渲染逻辑，负责构建整个应用的 UI 界面。
//! 该视图采用分层架构，从上到下依次为：
//!
//! - **顶部栏**（top bar）：包含应用标题和全局操作按钮
//! - **标签栏**（tab bar）：用于在不同功能模块间切换
//! - **内容区域**（content）：根据当前屏幕状态显示相应的功能视图
//! - **状态栏**（status bar）：显示应用状态信息
//!
//! 此外，该模块还负责处理各种模态框和叠加层的显示逻辑：
//!
//! - 系统设置面板
//! - 通知面板
//! - 错误提示横幅
//! - 关于弹窗
//! - CLI 安装提示
//! - 文件树重命名对话框
//! - 会话重命名对话框
//! - 项目编辑对话框
//! - 确认对话框

use iced::Element;

use super::components;
use super::state::WorkspaceMode;
use super::views;
use super::{App, Message, Screen};
use super::{app_view_modals, app_view_status};

impl App {
    pub fn view_window(&self, window: iced::window::Id) -> Element<'_, Message> {
        if self.task_pet_window_id == Some(window) {
            components::task_pet::window(self)
        } else {
            self.view()
        }
    }

    /// 渲染应用的主视图
    ///
    /// 该方法是整个应用的视图入口，负责构建完整的 UI 元素树。
    /// 它根据应用当前的状态（如当前屏幕、是否显示模态框等）来决定渲染哪些组件。
    ///
    /// # 返回值
    ///
    /// 返回一个 `Element<Message>` 类型的 UI 元素，这是 iced 框架的根视图元素。
    /// 该元素包含了整个应用的所有可见内容。
    ///
    /// # 视图结构
    ///
    /// 视图按照以下层次结构进行组织：
    ///
    /// 1. **基础布局**：垂直排列的顶部栏、标签栏、内容区域和状态栏
    /// 2. **系统设置层**：如果启用了系统设置，则在基础布局之上叠加设置面板
    /// 3. **通知层**：如果通知面板展开，则在主视图之上叠加通知组件
    /// 4. **错误提示层**：如果存在错误消息，则在最顶部显示红色错误横幅
    /// 5. **模态框层**：依次叠加各种模态框（关于、CLI 安装、重命名等）
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// // 在应用的主循环中调用此方法
    /// fn view(&self) -> Element<Message> {
    ///     self.app.view()
    /// }
    /// ```
    pub fn view(&self) -> Element<'_, Message> {
        // 构建顶部栏组件
        let top = components::top_bar::view(self);
        // 构建标签栏组件
        let tab_bar = components::tab_bar::view(self);

        // 根据当前屏幕状态选择对应的内容视图
        let content: Element<'_, Message> = match self.screen {
            Screen::Home => views::home::view(self),
            Screen::Project => views::project::view(self),
            Screen::Preview => components::preview_panel::view(self),
            Screen::Apps => views::apps::view(self),
            Screen::Usage => views::usage::view(self),
            Screen::Knowledge => views::knowledge::view(self),
            Screen::WorkflowTool => crate::apps::workflow::view(self),
            Screen::WorkflowManager => views::workflow_manager::view(self),
            Screen::TaskBoard => views::task_board::view(self),
            #[cfg(not(target_arch = "wasm32"))]
            Screen::ShijuApps => views::expert_hub::shiju_apps::view(self),
            #[cfg(not(target_arch = "wasm32"))]
            Screen::ShijuSkills => views::expert_hub::shiju_skills_view(self),
            #[cfg(not(target_arch = "wasm32"))]
            Screen::ExpertHub => views::expert_hub::view(self),
        };

        // 将内容视图包装在容器中，使其填充整个可用空间
        let content: Element<'_, Message> = iced::widget::container(content)
            .width(iced::Length::Fill)
            .height(iced::Length::Fill)
            .into();

        // 构建状态栏组件
        let status_bar: Element<'_, Message> = app_view_status::status_bar(self);

        // 当系统设置打开时，仅保留顶部菜单栏，其余区域全部由设置面板占满
        let main_view: Element<'_, Message> = if self.show_system_settings {
            iced::widget::column![top, components::system_settings::view(self)]
                .width(iced::Length::Fill)
                .height(iced::Length::Fill)
                .into()
        } else if show_tab_bar(self.screen, self.workspace_mode) {
            // 组装主视图：垂直排列顶部栏、标签栏、内容区域和状态栏
            iced::widget::column![top, tab_bar, content, status_bar]
                .width(iced::Length::Fill)
                .height(iced::Length::Fill)
                .into()
        } else {
            iced::widget::column![top, content, status_bar]
                .width(iced::Length::Fill)
                .height(iced::Length::Fill)
                .into()
        };

        // 如果通知面板已展开，则在主视图之上叠加通知组件
        // 通知面板定位在右下角，带有适当的内边距
        let main_view: Element<'_, Message> = if self.notifications_expanded {
            iced::widget::stack![
                main_view,
                iced::widget::container(components::notification::view(self))
                    .width(iced::Length::Fill)
                    .height(iced::Length::Fill)
                    .align_x(iced::alignment::Horizontal::Right)
                    .align_y(iced::alignment::Vertical::Bottom)
                    .padding(iced::Padding::default().top(0).right(8).bottom(32).left(0))
            ]
            .into()
        } else {
            main_view
        };

        // 如果存在错误消息，则在最顶部显示红色错误横幅
        // 错误横幅包含错误文本和关闭按钮
        let root_content = if let Some(error) = &self.error_message {
            use iced::widget::{button, column, container, row, text};
            use iced::{Background, Border, Color, Length};

            let card = container(
                column![
                    text("操作失败").size(20).color(Color::from_rgb(0.86, 0.2, 0.2)),
                    text(error).size(14).width(Length::Fill),
                    row![
                        container(text("")).width(Length::Fill),
                        button(text("知道了").size(14))
                            .on_press(Message::CloseError)
                            .padding([8, 12])
                            .style(|theme, status| {
                                let mut style = button::primary(theme, status);
                                style.border.radius = 8.0.into();
                                style
                            })
                    ]
                    .align_y(iced::Alignment::Center)
                ]
                .spacing(12),
            )
            .width(Length::Fixed(560.0))
            .padding(16)
            .style(|theme: &iced::Theme| {
                let palette = theme.extended_palette();
                container::Style {
                    background: Some(Background::Color(palette.background.base.color)),
                    text_color: Some(palette.background.base.text),
                    border: Border {
                        radius: 12.0.into(),
                        width: 1.0,
                        color: palette.secondary.weak.color,
                    },
                    ..Default::default()
                }
            });

            iced::widget::stack![
                main_view,
                container(
                    container(card)
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .center_x(Length::Fill)
                        .center_y(Length::Fill)
                )
                .width(Length::Fill)
                .height(Length::Fill)
                .style(|_theme| container::Style {
                    background: Some(Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.4))),
                    ..Default::default()
                })
            ]
            .into()
        } else {
            main_view
        };

        // 如果显示了"关于"模态框，则在根内容之上叠加
        let root_content: Element<'_, Message> = if self.show_about_modal {
            iced::widget::stack![root_content, components::about_modal::view(self)].into()
        } else {
            root_content
        };

        // 如果显示了 CLI 安装提示模态框，则在根内容之上叠加
        let root_content: Element<'_, Message> = if self.show_cli_install_modal {
            let modal = if self.cli_install_modal_show_update_action {
                components::install_cli_modal::view_update_check_with_progress(
                    &self.cli_install_modal_title,
                    &self.cli_install_modal_message,
                    &self.cli_install_modal_current_version,
                    &self.cli_install_modal_server_version,
                    self.cli_install_modal_is_checking_update,
                    self.cli_install_modal_show_install_action,
                    self.cli_install_modal_use_app_update_action,
                    self.app_update_progress.as_ref(),
                    self.last_copied_code_hash,
                )
            } else {
                components::install_cli_modal::view(
                    &self.cli_install_modal_title,
                    &self.cli_install_modal_message,
                )
            };
            iced::widget::stack![root_content, modal].into()
        } else {
            root_content
        };

        // 依次叠加各种功能性模态框
        // 文件树重命名模态框
        let root_content = app_view_modals::with_file_tree_rename(self, root_content);
        // 会话重命名模态框
        let root_content = app_view_modals::with_session_rename(self, root_content);
        // 项目编辑模态框
        let root_content = app_view_modals::with_project_edit(self, root_content);
        let root_content = app_view_modals::with_git_diff_overlays(self, root_content);
        let root_content = app_view_modals::with_account_details(self, root_content);
        let root_content = app_view_modals::with_chat_reset_dialog(self, root_content);
        let root_content = app_view_modals::with_chat_fork_dialog(self, root_content);
        let root_content = app_view_modals::with_recent_project_remove_confirm(self, root_content);
        // 确认对话框（问题模态框）
        let root_content = app_view_modals::with_question_modal(self, root_content);
        let root_content = app_view_modals::with_permission_modal(self, root_content);
        let root_content: Element<'_, Message> = if self.show_search_overlay {
            iced::widget::stack![root_content, components::search_panel::overlay(self)].into()
        } else {
            root_content
        };

        let root_content: Element<'_, Message> = if self.active_toast.is_some() {
            let toast_layer: Element<'_, Message> = iced::widget::column![
                iced::widget::Space::new().height(iced::Length::Fill),
                iced::widget::container(
                    iced::widget::row![
                        iced::widget::Space::new().width(iced::Length::Fill),
                        components::toast::view(self)
                    ]
                    .width(iced::Length::Fill)
                )
                .width(iced::Length::Fill)
                .height(iced::Length::Shrink)
                .padding(iced::Padding::default().top(0).right(12).bottom(44).left(0))
            ]
            .width(iced::Length::Fill)
            .height(iced::Length::Fill)
            .into();

            iced::widget::stack![root_content, toast_layer].into()
        } else {
            root_content
        };

        // 自定义主题背景图：置于最底层；无背景时保持原视图。
        if let Some(path) =
            self.active_theme_skin.as_ref().and_then(|skin| skin.background_path.as_ref())
        {
            use iced::widget::Image;
            use iced::widget::image::Handle as ImageHandle;
            use iced::{ContentFit, Length};

            let wallpaper: Element<'_, Message> = iced::widget::container(
                Image::new(ImageHandle::from_path(path.clone()))
                    .content_fit(ContentFit::Cover)
                    .width(Length::Fill)
                    .height(Length::Fill),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .into();

            iced::widget::stack![wallpaper, root_content].into()
        } else {
            root_content
        }
    }
}

fn show_tab_bar(_screen: Screen, _workspace_mode: WorkspaceMode) -> bool {
    true
}

#[cfg(test)]
#[path = "app_view_tests.rs"]
mod app_view_tests;
