//! 专家领域更新的桌面映射。
use crate::app::{App, Message};
use iced::Task;
pub(crate) use desktop_expert_hub::message::public::PublicSkillMessage;
#[cfg(test)]
pub(crate) use desktop_expert_hub::message::public::{PublicRequest, reduce};
use crate::app::message::{SettingsMessage, ViewMessage, settings};
use crate::app::skill_hub::{ExpertHubSection, LoadState, PublicSkillTab};

pub(crate) fn ensure_loaded(app: &mut App) -> Task<Message> {
    if crate::app::components::system_settings_skills::is_visible(app) {
        return settings::skills::update(app, SettingsMessage::SkillsRefresh);
    }
    super::run_domain(app, |input| desktop_expert_hub::message::public::ensure_loaded(input))
}

pub(super) fn open_local_skills(app: &mut App) -> Task<Message> {
    let close_settings = if app.navigation_panels.show_system_settings {
        crate::app::message::view::update(app, ViewMessage::ToggleSystemSettings)
    } else {
        Task::none()
    };
    app.popover_visibility.file = false;
    app.expert_hub.reset_navigation();
    let market = &mut app.expert_hub.public_market;
    market.close_detail();
    market.installed_open = false;
    market.delete_confirmation_open = false;
    market.tab = PublicSkillTab::LocalSkills;
    super::open_expert_hub_tab(app);

    let select = settings::skills::update(
        app,
        SettingsMessage::SkillsTabChanged(crate::app::state::SkillsSettingsTab::Skills),
    );
    Task::batch([close_settings, select, ensure_loaded(app)])
}

pub(crate) fn update(app: &mut App, message: PublicSkillMessage) -> Task<Message> {
    if matches!(message, PublicSkillMessage::SelectTab(PublicSkillTab::LocalSkills)) {
        if crate::app::components::system_settings_skills::is_visible(app) {
            return Task::none();
        }
        return open_local_skills(app);
    }
    let local_tab = app.expert_hub.section == ExpertHubSection::Skills
        && app.expert_hub.public_market.tab == PublicSkillTab::LocalSkills;
    let leaving_local = local_tab && matches!(message, PublicSkillMessage::SelectTab(_));
    let returning_to_local = local_tab && matches!(message, PublicSkillMessage::CloseInstalled);
    let task = super::run_domain(app, |input| {
        desktop_expert_hub::message::public::update(input, message)
    });
    if returning_to_local
        || (leaving_local
            && matches!(app.expert_hub.public_market.sources, LoadState::Idle | LoadState::Error(_)))
    {
        return Task::batch([task, ensure_loaded(app)]);
    }
    task
}
#[cfg(test)]
pub(crate) fn selectable_expert_skill(app: &App) -> Result<String, &'static str> {
    desktop_expert_hub::message::public::selectable_expert_skill(&app.expert_hub.public_market, app.navigation_tabs.project_path.as_deref(), app.navigation_tabs.open_tabs.iter().any(|tab| tab.project_path.as_deref() == app.navigation_tabs.project_path.as_deref() && tab.screen == crate::app::Screen::Project))
}
