use super::*;
pub struct Fixture {
    pub expert_hub: desktop_skill_hub::skill_hub::ExpertHubState,
    pub shiju_apps: desktop_app_launch::ShijuAppsState,
    pub desktop_locale: config_ui::DesktopLocale,
    pub app_theme: iced::Theme,
    pub project_path: Option<String>,
    pub recent_projects: Vec<String>,
    pub screen: Screen,
}
impl Fixture {
    pub fn new() -> (Self, ()) {
        (
            Self {
                expert_hub: Default::default(),
                shiju_apps: Default::default(),
                desktop_locale: Default::default(),
                app_theme: iced::Theme::Dark,
                project_path: None,
                recent_projects: Vec::new(),
                screen: Screen::ExpertHub,
            },
            (),
        )
    }
    pub fn input(&self) -> ViewInput<'_> {
        ViewInput {
            expert_hub: &self.expert_hub,
            shiju_apps: &self.shiju_apps,
            desktop_locale: self.desktop_locale,
            app_theme: &self.app_theme,
            project_path: &self.project_path,
            recent_projects: &self.recent_projects,
            screen: self.screen,
            has_project_chat: false,
        }
    }
}
pub fn update(
    app: &mut Fixture,
    message: crate::message::skill_hub::SkillHubMessage,
) -> iced::Task<crate::message::Message> {
    let mut input = crate::message::SkillUpdate {
        expert_hub: &mut app.expert_hub,
        active_account: None,
        project_path: &app.project_path,
        recent_projects: &app.recent_projects,
        screen: crate::message::HubPage::ExpertHub,
        has_project_chat: false,
        desktop_locale: app.desktop_locale,
        gateway: || Err("fixture does not execute network tasks".into()),
        effects: Vec::new(),
    };
    crate::message::skill_hub::update(&mut input, message)
}
