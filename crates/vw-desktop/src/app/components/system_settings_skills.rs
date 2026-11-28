//! 技能配置设置界面组件。

mod backup_modal;
mod bindings_view;
mod browser;
mod catalog;
mod claude_view;
mod deeplink;
mod help;
mod import_modal;
mod install_view;
mod jobs_view;
mod market_view;
mod plugin_install_modal;
pub(crate) mod plugins;
mod repo_modal;
mod storage_modal;
mod updates_view;
pub(crate) mod workspace;
pub(crate) mod sync;

use crate::app::components::system_settings_search::SettingsTabDescriptor;
use crate::app::state::SkillsSettingsTab;
use crate::app::{App, Message, message};
#[cfg(test)]
use desktop_i18n::settings_extensions::text as tr;
use iced::Element;

use desktop_settings_skills::page::SKILLS_TAB_LABELS;
fn page_message(event: desktop_settings_skills::page::Message) -> Message {
    use desktop_settings_skills::page::Message as Ui;
    Message::Settings(match event {
        Ui::HelpOpen => message::SettingsMessage::SkillsHelpOpen,
        #[cfg(not(target_arch = "wasm32"))]
        Ui::OpenSkillList => {
            return Message::Batch(vec![
                Message::View(message::ViewMessage::ToggleSystemSettings),
                Message::SkillHub(message::SkillHubMessage::OpenShijuSkills),
            ]);
        }
        Ui::RepositoriesOpen => message::SettingsMessage::SkillRepositories(
            crate::app::message::settings::skills::repo_modal::Event::Open,
        ),
        Ui::ClaudeOpen => message::SettingsMessage::SkillsClaudeConfig(
            crate::app::message::settings::skills::claude_view::Event::Open,
        ),
        Ui::MarketOpen => message::SettingsMessage::SkillsMarket(
            crate::app::message::settings::skills::market_view::Event::Open,
        ),
        Ui::UpdatesOpen => message::SettingsMessage::SkillsUpdates(
            crate::app::message::settings::skills::updates_view::Event::Open,
        ),
        Ui::InstallOpen => message::SettingsMessage::SkillsInstall(
            crate::app::message::settings::skills::install_view::Event::Open,
        ),
        Ui::ImportOpen => message::SettingsMessage::SkillsImport(
            crate::app::message::settings::skills::import_modal::Event::Open,
        ),
        Ui::JobsOpen => message::SettingsMessage::SkillsJobs(
            crate::app::message::settings::skills::jobs_view::Event::Open,
        ),
        Ui::BindingsOpen => message::SettingsMessage::SkillsBindings(
            crate::app::message::settings::skills::bindings_view::Event::Open,
        ),
        Ui::StorageOpen => message::SettingsMessage::SkillsStorage(
            crate::app::message::settings::skills::storage_modal::Event::Open,
        ),
        Ui::BackupOpen => message::SettingsMessage::SkillsBackups(
            crate::app::message::settings::skills::backup_modal::Event::Open,
        ),
        Ui::None => return Message::None,
        Ui::CopyText(text) => return Message::CopyCode(text),
        Ui::SkillsTabChanged(v) => message::SettingsMessage::SkillsTabChanged(v),
        Ui::SkillsProjectAutoLoadToggled(v) => {
            message::SettingsMessage::SkillsProjectAutoLoadToggled(v)
        }
        Ui::SkillsPromptInjectionModeChanged(v) => {
            message::SettingsMessage::SkillsPromptInjectionModeChanged(v)
        }
    })
}
fn config_context(app: &App) -> desktop_settings_skills::page::ConfigContext<'_> {
    let s = &app.skills_settings;
    desktop_settings_skills::page::ConfigContext {
        help,
        desktop_locale: app.general_settings.desktop_locale,
        app_theme: &app.appearance.app_theme,
        auto_load_project_skills: s.auto_load_project_skills,
        prompt_injection_mode: s.prompt_injection_mode,
    }
}
pub(super) const SKILLS_TABS: &[SettingsTabDescriptor<SkillsSettingsTab>] = &[
    SettingsTabDescriptor::new(
        SKILLS_TAB_LABELS[0].1,
        SKILLS_TAB_LABELS[0].0,
        &["skills", "catalog", "技能目录"],
    ),
    SettingsTabDescriptor::new(
        SKILLS_TAB_LABELS[1].1,
        SKILLS_TAB_LABELS[1].0,
        &["discovery order", "发现顺序", "project", "ancestor", "global", "bundled"],
    ),
    SettingsTabDescriptor::new(
        SKILLS_TAB_LABELS[2].1,
        SKILLS_TAB_LABELS[2].0,
        &["项目技能", "project skills", "提示注入", "prompt injection"],
    ),
    SettingsTabDescriptor::new(
        SKILLS_TAB_LABELS[3].1,
        SKILLS_TAB_LABELS[3].0,
        &["third-party skills", "第三方技能", "仓库管理", "Claude 配置", "技能市场", "导入技能", "应用绑定", "存储与同步", "备份与恢复"],
    ),
];

fn system_config_view(app: &App) -> Element<'_, Message> {
    desktop_settings_skills::page::system_config_view(config_context(app)).map(page_message)
}

pub fn view(app: &App) -> Element<'_, Message> {
    let content = match app.skills_settings.active_tab {
        SkillsSettingsTab::Skills => browser::view(app),
        SkillsSettingsTab::DiscoveryOrder => browser::discovery_order_view(app),
        SkillsSettingsTab::SystemConfig => system_config_view(app),
        SkillsSettingsTab::Distribution => workspace::view(app),
    };
    desktop_settings_skills::page::view(
        config_context(app),
        app.skills_settings.active_tab,
        &app.skills_settings.save_error,
        content,
        page_message,
    )
}

pub fn view_overlays<'a>(app: &'a App, dialog: Element<'a, Message>) -> Element<'a, Message> {
    let dialog = browser::view_overlays(app, dialog);
    let dialog = help::view_overlays(app, dialog);
    if workspace_visible(app) { workspace_overlays(app, dialog) } else { sync::overlay(app, dialog) }
}

fn workspace_overlays<'a>(app: &'a App, dialog: Element<'a, Message>) -> Element<'a, Message> {
    let dialog = repo_modal::view(app, dialog);
    let dialog = claude_view::view(app, dialog);
    let dialog = bindings_view::view(app, dialog);
    let dialog = storage_modal::view(app, dialog);
    let dialog = backup_modal::view(app, dialog);
    let dialog = market_view::view(app, dialog);
    let dialog = install_view::view(app, dialog);
    let dialog = import_modal::view(app, dialog);
    let dialog = jobs_view::view(app, dialog);
    updates_view::view(app, dialog)
}


pub(crate) fn is_visible(app: &App) -> bool {
    app.navigation_panels.show_system_settings
        && app.navigation_panels.system_settings_tab == super::system_settings::SystemTab::Agents
        && app.agents_settings.active_config_tab == crate::app::state::AgentConfigTab::Skills
}

pub(crate) fn workspace_visible(app: &App) -> bool {
    is_visible(app) && app.skills_settings.active_tab == SkillsSettingsTab::Distribution
}

pub(crate) fn open(app: &mut App) -> iced::Task<Message> {
    crate::app::message::view::update(
        app,
        message::ViewMessage::OpenSystemSettingsTarget(
            super::system_settings_search::SystemSettingsSearchTarget::AgentSkills(
                SkillsSettingsTab::Skills,
            ),
        ),
    )
}

pub(crate) fn open_workspace(app: &mut App) -> iced::Task<Message> {
    if workspace_visible(app) { workspace::open(app) } else { iced::Task::none() }
}

pub(crate) fn invalidate(app: &mut App) {
    use crate::app::message::SettingsMessage;
    use crate::app::message::settings::skills;
    for event in [
        SettingsMessage::SkillRepositories(skills::repo_modal::Event::Close),
        SettingsMessage::SkillsClaudeConfig(skills::claude_view::Event::Close),
        SettingsMessage::SkillsBindings(skills::bindings_view::Event::Close),
        SettingsMessage::SkillsImport(skills::import_modal::Event::Close),
        SettingsMessage::SkillsInstall(skills::install_view::Event::Close),
        SettingsMessage::SkillsMarket(skills::market_view::Event::Close),
        SettingsMessage::SkillsUpdates(skills::updates_view::Event::Close),
        SettingsMessage::SkillsJobs(skills::jobs_view::Event::Close),
        SettingsMessage::SkillsStorage(skills::storage_modal::Event::Close),
        SettingsMessage::SkillsBackups(skills::backup_modal::Event::Close),
    ] {
        // Close reducers only dismiss presentation; writes keep their original scope.
        let _ = skills::update(app, event);
    }
    // Busy reducers retain their captured request scope, but no presentation
    // may survive a navigation or host change.
    let skills = &mut app.skills_settings;
    skills.repositories.open = false;
    skills.claude_config.open = false;
    skills.bindings.open = false;
    skills.import.open = false;
    skills.install.open = false;
    skills.market.open = false;
    skills.updates.open = false;
    skills.jobs.open = false;
    skills.storage.open = false;
    skills.backups.open = false;
    skills.sync.menu = None;
    skills.sync.form = None;
    skills.workspace.sync(None, app.apps_catalog.navigation_generation);
}

#[cfg(test)]
#[path = "system_settings_skills_tests.rs"]
mod system_settings_skills_tests;

pub use desktop_settings_skills::{modal_card_style, plugin_dependency_summary};

pub(crate) fn with_plugin_install<'a>(
    app: &'a App,
    base: Element<'a, Message>,
) -> Element<'a, Message> {
    match app.skills_settings.plugin_install.as_ref() {
        Some(form) => plugin_install_modal::view(app, base, form),
        None => base,
    }
}

#[cfg(test)]
#[path = "system_settings_skills_ui_refresh_19_tests.rs"]
mod ui_refresh_19_tests;

fn help<'a>(
    i18n: desktop_i18n::I18n,
    theme: Option<&iced::Theme>,
    title: &'a str,
    label: iced::widget::Text<'a>,
    description: std::borrow::Cow<'a, str>,
) -> iced::widget::Row<'a, desktop_settings_skills::page::Message> {
    let content = crate::app::components::chat_panel::message_view::selectable_titled_text(
        title,
        description,
        theme.map(|t| t.palette().text),
        i18n,
    )
    .map(|message| match message {
        Message::CopyCode(text) => desktop_settings_skills::page::Message::CopyText(text),
        Message::None => desktop_settings_skills::page::Message::None,
        _ => unreachable!("plain selectable help only emits copy or no-op events"),
    });
    desktop_ui_controls::system_settings_common::help::settings_hover_help_label(label, content)
}
