//! Hosts the settings pages with a shared project selector.

use crate::app::components::system_settings_common::{
    rounded_action_btn_style, settings_error_banner, settings_field_label, settings_page_heading,
    settings_select,
};
use crate::app::i18n::I18n;
use crate::app::views::workflow_manager::{self, WorkflowManagerMessage};
use crate::app::{App, Message};
use desktop_assets::assets::Icon;
use desktop_settings_shell::SystemTab;
use iced::widget::{button, column, container, row, text};
use iced::{Alignment, Element, Length, Task};

pub(in crate::app) mod catalog;
pub(in crate::app) mod labels;
pub(in crate::app) mod properties;

#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) mod quick_actions;

fn workspace_project(app: &App) -> Option<&str> {
    app.project_editor.project_edit_path.as_deref().or(app.navigation_tabs.project_path.as_deref())
}

fn projects(app: &App) -> impl Iterator<Item = &str> {
    workspace_project(app)
        .into_iter()
        .chain(app.navigation_tabs.project_path.as_deref())
        .chain(app.workspace_state.recent_projects.iter().map(String::as_str))
}

pub(in crate::app) fn active_tab(app: &App) -> Option<SystemTab> {
    let tab = app.navigation_panels.system_settings_tab;
    (app.navigation_panels.show_system_settings
        && matches!(
            tab,
            SystemTab::TaskWorkflow
                | SystemTab::TaskLabels
                | SystemTab::TaskProperties
                | SystemTab::TaskQuickActions
        ))
    .then_some(tab)
}

pub(in crate::app) fn project_path(app: &App) -> Option<&str> {
    if active_tab(app).is_some() {
        app.task_settings_project
            .as_deref()
            .filter(|selected| projects(app).any(|path| path == *selected))
            .or_else(|| projects(app).next())
    } else {
        workspace_project(app)
    }
}

pub(in crate::app) fn select_project(app: &mut App, path: String) -> Task<Message> {
    let Some(tab) = active_tab(app) else {
        return Task::none();
    };
    if saving_catalog(app)
        || project_path(app) == Some(path.as_str())
        || !projects(app).any(|project| project == path)
    {
        return Task::none();
    }
    app.task_settings_project = Some(path);
    load(app, tab)
}

fn saving_catalog(app: &App) -> bool {
    let saving = app.task_label_settings.saving() || app.task_property_settings.saving();
    #[cfg(not(target_arch = "wasm32"))]
    let saving = saving || app.task_quick_actions.settings.saving();
    saving
}

fn project_picker(app: &App) -> Element<'_, Message> {
    let i18n = I18n::new(app.general_settings.desktop_locale);
    let mut paths = Vec::new();
    for path in projects(app) {
        if !paths.contains(&path) {
            paths.push(path);
        }
    }
    let display_name = |path: &str| {
        app.workspace_state
            .recent_projects_meta
            .iter()
            .find(|project| project.path == path && !project.name.is_empty())
            .map(|project| project.name.as_str())
    };
    let options = paths.iter().map(|path| (*path).to_owned()).collect::<Vec<_>>();
    let picker = settings_select(
        options,
        project_path(app).map(str::to_owned),
        Message::TaskSettingsProjectSelected,
    )
    .format_labels(|path| match display_name(path) {
        Some(name) if paths.iter().filter(|path| display_name(path) == Some(name)).count() == 1 => {
            name.to_owned()
        }
        _ => path.clone(),
    })
    .placeholder(i18n.__("暂无可用项目"))
    .width(Length::Fill)
    .disabled(saving_catalog(app));
    row![
        catalog::glyph(Icon::FolderClosed, 16.0),
        settings_field_label(i18n.__("当前项目")),
        container(picker).width(Length::Fill).max_width(320),
    ]
    .spacing(10)
    .align_y(Alignment::Center)
    .into()
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
    if tab == SystemTab::TaskWorkflow {
        return column![container(project_picker(app)).padding([8, 24]), workflow(app),]
            .spacing(8)
            .height(Length::Fill)
            .into();
    }
    let content = match tab {
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
            )
            .into()
        }
        _ => unreachable!("task settings tab"),
    };
    column![project_picker(app), content].spacing(18).width(Length::Fill).into()
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
    workflow_manager::settings_content(app)
}
