use crate::app::App;
use crate::app::state::{AgentSettingsEntryKind, ProviderSummary};

fn root_child_count(app: &App) -> usize {
    super::view(app).as_widget().children().len()
}

#[test]
fn agents_view_loading_state_adds_status_row() {
    let (mut app, _) = App::new();

    assert_eq!(root_child_count(&app), 4);

    app.agents_settings.loading = true;

    assert_eq!(root_child_count(&app), 5);
}

#[test]
fn agents_view_builds_sidebar_for_main_enabled_disabled_and_model_variants() {
    let (mut app, _) = App::new();
    let mut main = app
        .agents_settings
        .entries
        .iter()
        .find(|entry| entry.key == "main")
        .cloned()
        .expect("default settings should contain the main agent");
    main.model = "   ".to_string();

    let mut enabled_worker = app
        .agents_settings
        .entries
        .iter()
        .find(|entry| entry.key != "main")
        .cloned()
        .expect("default settings should contain a worker agent");
    enabled_worker.enabled = true;
    enabled_worker.model = "worker-model".to_string();

    let mut disabled_custom = enabled_worker.clone();
    disabled_custom.key = "custom-disabled".to_string();
    disabled_custom.label = "Disabled custom".to_string();
    disabled_custom.kind = AgentSettingsEntryKind::Custom;
    disabled_custom.enabled = false;
    disabled_custom.model.clear();

    app.agents_settings.entries = vec![main, enabled_worker, disabled_custom];
    app.agents_settings.selected_agent = "custom-disabled".to_string();

    assert_eq!(root_child_count(&app), 4);
}

#[test]
fn agents_view_missing_selection_uses_first_entry_and_unknown_tab_falls_back() {
    let (mut app, _) = App::new();
    let first_key = app
        .agents_settings
        .entries
        .first()
        .expect("default settings should contain agents")
        .key
        .clone();
    app.agents_settings.selected_agent = "missing-agent".to_string();
    app.agents_settings.active_detail_tab = "future-detail-tab".to_string();
    app.agents_settings.providers = vec![ProviderSummary {
        id: "provider-a".to_string(),
        name: "Provider A".to_string(),
        source_label: "test".to_string(),
        connected: true,
    }];

    let selected = super::shared::selected_entry(&app)
        .expect("a missing selection should fall back to the first entry");
    assert_eq!(selected.key, first_key);
    assert_eq!(root_child_count(&app), 4);
}
