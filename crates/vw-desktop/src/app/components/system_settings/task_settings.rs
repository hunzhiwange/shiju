//! Hosts task settings using the project selected by the desktop.

use crate::app::components::system_settings_common::{
    rounded_action_btn_style, settings_error_banner, settings_muted_text_style,
    settings_page_heading,
};
use crate::app::i18n::I18n;
use crate::app::views::workflow_manager::{self, WorkflowManagerMessage};
use crate::app::{App, Message};
use desktop_settings_shell::SystemTab;
use iced::widget::{button, column, container, row, text};
use iced::{Element, Length, Task};

pub(in crate::app) mod labels;
pub(in crate::app) mod properties;

#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) mod quick_actions;

pub(in crate::app) fn project_path(app: &App) -> Option<&str> {
    app.project_editor
        .project_edit_path
        .as_deref()
        .or(app.navigation_tabs.project_path.as_deref())
}

pub(in crate::app) fn load(app: &mut App, tab: SystemTab) -> Task<Message> {
    match tab {
        SystemTab::TaskWorkflow => {
            app.workflow_manager.dismiss_dialogs();
            workflow_manager::update_app(app, WorkflowManagerMessage::RefreshLifecycles)
        }
        SystemTab::TaskLabels => labels::update(app, labels::Event::Refresh),
        SystemTab::TaskProperties => properties::update(app, properties::Event::Refresh),
        #[cfg(not(target_arch = "wasm32"))]
        SystemTab::TaskQuickActions => quick_actions::load(app),
        _ => Task::none(),
    }
}

pub(super) fn view(app: &App, tab: SystemTab) -> Element<'_, Message> {
    match tab {
        SystemTab::TaskWorkflow => workflow(app),
        SystemTab::TaskLabels => labels::view(app),
        SystemTab::TaskProperties => properties::view(app),
        #[cfg(not(target_arch = "wasm32"))]
        SystemTab::TaskQuickActions => quick_actions::view(app),
        #[cfg(target_arch = "wasm32")]
        SystemTab::TaskQuickActions => {
            let i18n = I18n::new(app.general_settings.desktop_locale);
            settings_page_heading(
                i18n.__(tab.label()),
                i18n.__("请在桌面客户端管理项目目录中的快捷操作。"),
            ).into()
        }
        _ => unreachable!("task settings tab"),
    }
}

fn workflow(app: &App) -> Element<'_, Message> {
    let i18n = I18n::new(app.general_settings.desktop_locale);
    let title = i18n.__(SystemTab::TaskWorkflow.label());
    let state = &app.workflow_manager;
    if let Some(error) = state.lifecycles_error() {
        let retry = button(text(i18n.__("重试")).size(13))
            .padding([8, 12])
            .style(rounded_action_btn_style)
            .on_press(Message::WorkflowManager(WorkflowManagerMessage::RefreshLifecycles));
        return desktop_settings_shell::centered_content(
            column![
                settings_page_heading(title, i18n.__("工作流加载失败")),
                settings_error_banner(i18n.__(error)),
                retry,
            ]
            .spacing(12),
        )
        .into();
    }
    if !state.lifecycles_loaded() {
        return desktop_settings_shell::centered_content(settings_page_heading(
            title,
            i18n.__("正在加载工作流…"),
        ))
        .into();
    }
    let project = match project_path(app) {
        Some(path) => row![
            text(i18n.__("当前项目")).size(12),
            text(path).size(12).style(settings_muted_text_style),
        ].spacing(8),
        None => row![text(i18n.__("选择项目后可为该项目应用工作流。")).size(12)],
    };
    column![container(project).padding([0, 24]), workflow_manager::settings_content(app)]
        .height(Length::Fill)
        .into()
}
