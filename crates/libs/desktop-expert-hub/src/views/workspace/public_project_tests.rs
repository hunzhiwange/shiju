use super::fixture::Fixture;
use crate::message::public::PublicSkillMessage;
use desktop_skill_hub::skill_hub::{LoadState, PublicSkillMarketState};
use gateway_skill_market::{SkillMarketSkillLocatorDto, SkillMarketSourceDto};
fn loaded_sources(state: &mut PublicSkillMarketState) {
    state.sources = LoadState::Loaded(vec![SkillMarketSourceDto {
        source: "skillhub".into(),
        display_name: "SkillHub".into(),
    }]);
    state.current_source = Some("skillhub".into());
}
fn update_public(
    app: &mut Fixture,
    message: PublicSkillMessage,
) -> iced::Task<crate::message::Message> {
    super::fixture::update(app, crate::message::skill_hub::SkillHubMessage::Public(message))
}
#[test]
fn public_skill_detail_can_switch_install_project_without_switching_workspace() {
    let (mut app, _) = Fixture::new();
    app.project_path = Some("/repo/current".to_string());
    app.recent_projects = vec!["/repo/other".to_string(), "/repo/current".to_string()];
    loaded_sources(&mut app.expert_hub.public_market);
    let locator = SkillMarketSkillLocatorDto {
        source: "skillhub".to_string(),
        namespace: "public".to_string(),
        slug: "safe-skill".to_string(),
    };

    let _ = update_public(&mut app, PublicSkillMessage::OpenSkillDetail(locator));
    assert_eq!(
        app.expert_hub
            .public_market
            .detail
            .as_ref()
            .and_then(|detail| detail.selected_install_project_path.as_deref()),
        Some("/repo/current")
    );

    let previous_generation = app.expert_hub.public_market.installed_generation;
    let _ = update_public(
        &mut app,
        PublicSkillMessage::SelectDetailInstallProject("/repo/other".to_string()),
    );

    assert_eq!(app.project_path.as_deref(), Some("/repo/current"));
    assert_eq!(
        app.expert_hub
            .public_market
            .detail
            .as_ref()
            .and_then(|detail| detail.selected_install_project_path.as_deref()),
        Some("/repo/other")
    );
    assert!(app.expert_hub.public_market.installed_generation > previous_generation);
}
