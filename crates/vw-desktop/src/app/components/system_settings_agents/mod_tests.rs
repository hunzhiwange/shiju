//! 验证代理设置测试子模块已正确接入。
//! 各标签页的行为由细分测试覆盖，此处仅防止测试模块装配回归。

#[test]
fn agents_view_builds_default_loading_and_error_states() {
    let (mut app, _) = crate::app::App::new();

    let _: iced::Element<'_, crate::app::Message> = super::view(&app);

    app.agents_settings.loading = true;
    app.agents_settings.save_error = Some("unable to save".to_string());
    let _: iced::Element<'_, crate::app::Message> = super::view(&app);
}

#[test]
fn agent_config_tabs_render_each_existing_settings_page() {
    let (mut app, _) = crate::app::App::new();

    for tab in [
        crate::app::state::AgentConfigTab::Agents,
        crate::app::state::AgentConfigTab::Channels,
        crate::app::state::AgentConfigTab::Runtime,
        crate::app::state::AgentConfigTab::Acp,
        crate::app::state::AgentConfigTab::Mcp,
        crate::app::state::AgentConfigTab::Skills,
        crate::app::state::AgentConfigTab::Plugins,
        crate::app::state::AgentConfigTab::Reliability,
        crate::app::state::AgentConfigTab::DialogueFlow,
        crate::app::state::AgentConfigTab::QuickMessages,
        crate::app::state::AgentConfigTab::Heartbeat,
    ] {
        app.agents_settings.active_config_tab = tab;
        let _: iced::Element<'_, crate::app::Message> = super::view(&app);
    }
}

#[test]
fn agents_error_banner_follows_agent_tabs() {
    let (mut app, _) = crate::app::App::new();
    app.agents_settings.active_config_tab = crate::app::state::AgentConfigTab::Agents;

    assert_eq!(super::view(&app).as_widget().children().len(), 2);

    app.agents_settings.save_error = Some("agents save failed".to_string());
    assert_eq!(super::active_tab_save_error(&app), Some("agents save failed"));
    assert_eq!(super::view(&app).as_widget().children().len(), 3);

    app.runtime_settings.save_error = Some("runtime save failed".to_string());
    app.acp_settings.save_error = Some("acp save failed".to_string());
    app.reliability_settings.save_error = Some("reliability save failed".to_string());
    app.heartbeat_settings.save_error = Some("heartbeat save failed".to_string());
    for (tab, expected) in [
        (crate::app::state::AgentConfigTab::Runtime, Some("runtime save failed")),
        (crate::app::state::AgentConfigTab::Acp, Some("acp save failed")),
        (
            crate::app::state::AgentConfigTab::Reliability,
            Some("reliability save failed"),
        ),
        (crate::app::state::AgentConfigTab::Heartbeat, Some("heartbeat save failed")),
        (crate::app::state::AgentConfigTab::Channels, None),
        (crate::app::state::AgentConfigTab::Mcp, None),
        (crate::app::state::AgentConfigTab::Skills, None),
        (crate::app::state::AgentConfigTab::Plugins, None),
        (crate::app::state::AgentConfigTab::DialogueFlow, None),
        (crate::app::state::AgentConfigTab::QuickMessages, None),
    ] {
        app.agents_settings.active_config_tab = tab;
        assert_eq!(super::active_tab_save_error(&app), expected);
    }
}

#[test]
fn agent_project_options_put_current_first_and_deduplicate_recent_paths() {
    let (mut app, _) = crate::app::App::new();
    app.project_path = Some("/repo/current".to_string());
    app.recent_projects = vec![
        "/repo/other".to_string(),
        "/repo/current".to_string(),
        "  /repo/third  ".to_string(),
        String::new(),
    ];

    assert_eq!(
        super::agent_project_options(&app),
        vec!["/repo/current".to_string(), "/repo/other".to_string(), "/repo/third".to_string(),]
    );
}

#[test]
fn agents_view_builds_empty_entries_fallback() {
    let (mut app, _) = crate::app::App::new();
    app.agents_settings.config_ready = true;
    app.agents_settings.entries.clear();
    app.agents_settings.selected_agent = "missing".to_string();

    let _: iced::Element<'_, crate::app::Message> = super::view(&app);
}

#[test]
fn agents_view_routes_each_detail_tab() {
    let (mut app, _) = crate::app::App::new();
    app.agents_settings.config_ready = true;

    for tab in [
        crate::app::state::AGENT_DETAIL_BASIC_TAB,
        crate::app::state::AGENT_DETAIL_IDENTITY_TAB,
        crate::app::state::AGENT_DETAIL_TOOLS_TAB,
        crate::app::state::AGENT_DETAIL_SKILLS_TAB,
    ] {
        app.agents_settings.active_detail_tab = tab.to_string();
        let _: iced::Element<'_, crate::app::Message> = super::view(&app);
    }
}

#[test]
fn agents_view_builds_main_enabled_disabled_and_model_metadata() {
    let (mut app, _) = crate::app::App::new();
    app.agents_settings.config_ready = true;
    let mut main = app.agents_settings.entries[0].clone();
    main.key = "main".to_string();
    main.label = "Main".to_string();
    main.enabled = true;
    main.model.clear();

    let mut enabled = main.clone();
    enabled.key = "enabled-worker".to_string();
    enabled.label = "Enabled worker".to_string();
    enabled.kind = crate::app::state::AgentSettingsEntryKind::BuiltinWorker;
    enabled.model = "provider/model".to_string();

    let mut disabled = main.clone();
    disabled.key = "disabled-custom".to_string();
    disabled.label = "Disabled custom".to_string();
    disabled.kind = crate::app::state::AgentSettingsEntryKind::Custom;
    disabled.enabled = false;
    disabled.model = "   ".to_string();

    app.agents_settings.entries = vec![main, enabled, disabled];
    app.agents_settings.selected_agent = "enabled-worker".to_string();

    let element = super::agents_view(&app);

    assert_eq!(element.as_widget().children().len(), 4);
}
