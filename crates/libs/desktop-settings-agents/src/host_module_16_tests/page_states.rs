use super::profile_inputs as inputs;
use super::profile_update::ProfileFixture;
#[test]
fn agents_view_builds_empty_entries_fallback() {
    let mut app = ProfileFixture::default();
    app.agents_settings.config_ready = true;
    app.agents_settings.entries.clear();
    app.agents_settings.selected_agent = "missing".to_string();

    let _: iced::Element<'_, crate::message::Event> = inputs::agents(&app);
}

#[test]
fn agents_view_routes_each_detail_tab() {
    let mut app = ProfileFixture::default();
    app.agents_settings.config_ready = true;

    for tab in [
        crate::state::AGENT_DETAIL_BASIC_TAB,
        crate::state::AGENT_DETAIL_IDENTITY_TAB,
        crate::state::AGENT_DETAIL_TOOLS_TAB,
        crate::state::AGENT_DETAIL_SKILLS_TAB,
    ] {
        app.agents_settings.active_detail_tab = tab.to_string();
        let _: iced::Element<'_, crate::message::Event> = inputs::agents(&app);
    }
}

#[test]
fn agents_view_builds_main_enabled_disabled_and_model_metadata() {
    let mut app = ProfileFixture::default();
    app.agents_settings.config_ready = true;
    let mut main = app.agents_settings.entries[0].clone();
    main.key = "main".to_string();
    main.label = "Main".to_string();
    main.enabled = true;
    main.model.clear();

    let mut enabled = main.clone();
    enabled.key = "enabled-worker".to_string();
    enabled.label = "Enabled worker".to_string();
    enabled.kind = crate::state::AgentSettingsEntryKind::BuiltinWorker;
    enabled.model = "provider/model".to_string();

    let mut disabled = main.clone();
    disabled.key = "disabled-custom".to_string();
    disabled.label = "Disabled custom".to_string();
    disabled.kind = crate::state::AgentSettingsEntryKind::Custom;
    disabled.enabled = false;
    disabled.model = "   ".to_string();

    app.agents_settings.entries = vec![main, enabled, disabled];
    app.agents_settings.selected_agent = "enabled-worker".to_string();

    let element = inputs::agents(&app);

    assert_eq!(element.as_widget().children().len(), 4);
}
