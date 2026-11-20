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

pub(in crate::app) fn project_path(app: &App) -> Option<&str> {
    app.project_editor
        .project_edit_path
        .as_deref()
        .or(app.navigation_tabs.project_path.as_deref())
}

pub(in crate::app) fn load_workflows(app: &mut App) -> Task<Message> {
    if project_path(app).is_some() && !app.workflow_manager.lifecycles_loading() {
        workflow_manager::update_app(app, WorkflowManagerMessage::RefreshLifecycles)
    } else {
        Task::none()
    }
}

pub(super) fn view(app: &App, tab: SystemTab) -> Element<'_, Message> {
    if tab == SystemTab::TaskWorkflow {
        workflow(app)
    } else {
        unavailable(app, tab)
    }
}

fn workflow(app: &App) -> Element<'_, Message> {
    let i18n = I18n::new(app.general_settings.desktop_locale);
    let title = i18n.__(SystemTab::TaskWorkflow.label());
    let Some(project_path) = project_path(app) else {
        return desktop_settings_shell::centered_content(settings_page_heading(
            title,
            i18n.__("请先选择项目"),
        ))
        .into();
    };
    let state = &app.workflow_manager;
    if let Some(error) = state.lifecycles_error() {
        let retry = button(text(i18n.__("重试")).size(13))
            .padding([8, 12])
            .style(rounded_action_btn_style)
            .on_press(Message::WorkflowManager(WorkflowManagerMessage::RefreshLifecycles));
        return desktop_settings_shell::centered_content(
            column![
                settings_page_heading(title, i18n.__("工作流加载失败")),
                settings_error_banner(error),
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
    let project = row![
        text(i18n.__("当前项目")).size(12),
        text(project_path).size(12).style(settings_muted_text_style),
    ]
    .spacing(8);
    column![container(project).padding([0, 24]), workflow_manager::view(app)]
        .height(Length::Fill)
        .into()
}

fn unavailable(app: &App, tab: SystemTab) -> Element<'_, Message> {
    let i18n = I18n::new(app.general_settings.desktop_locale);
    let title = settings_page_heading(i18n.__(tab.label()), i18n.__("此设置暂不可用"));
    let mut content = column![title].spacing(8);
    if project_path(app).is_none() {
        content =
            content.push(text(i18n.__("请先选择项目")).size(13).style(settings_muted_text_style));
    }
    content.into()
}
