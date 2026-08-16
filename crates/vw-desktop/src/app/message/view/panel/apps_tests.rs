#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("apps_tests"));
}

fn tab(id: &str, title: &str, screen: crate::app::Screen) -> crate::app::AppTab {
    crate::app::AppTab { id: id.to_string(), title: title.to_string(), screen, project_path: None }
}

fn ids(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

#[test]
fn open_apps_creates_and_activates_apps_tab() {
    let (mut app, _task) = crate::app::App::new();

    let _ = super::update(&mut app, crate::app::message::ViewMessage::OpenApps);

    assert_eq!(app.screen, crate::app::Screen::Apps);
    assert_eq!(app.active_tab_id.as_deref(), Some("apps"));
    assert_eq!(
        app.open_tabs.iter().find(|tab| tab.id == "apps"),
        Some(&tab("apps", "应用", crate::app::Screen::Apps))
    );
}

#[test]
fn open_apps_reuses_existing_tab_without_duplicate() {
    let (mut app, _task) = crate::app::App::new();
    app.open_tabs.push(tab("apps", "已有应用", crate::app::Screen::Apps));

    let _ = super::update(&mut app, crate::app::message::ViewMessage::OpenApps);

    assert_eq!(app.open_tabs.iter().filter(|tab| tab.id == "apps").count(), 1);
    assert_eq!(
        app.open_tabs.iter().find(|tab| tab.id == "apps").map(|tab| tab.title.as_str()),
        Some("已有应用")
    );
}

#[test]
fn open_usage_inserts_after_home_and_clears_stale_usage_state() {
    let (mut app, _task) = crate::app::App::new();
    app.active_session_id = None;
    app.usage_model_info = Some(crate::app::state::UsageModelInfo {
        provider_id: "provider".to_string(),
        provider_name: "Provider".to_string(),
        model_id: "model".to_string(),
        model_name: "Model".to_string(),
        context_limit: 1,
        input_limit: 1,
        output_limit: 1,
        limit_is_default: false,
        cost_input_per_million: 0.0,
        cost_output_per_million: 0.0,
        cost_cache_read_per_million: 0.0,
        cost_cache_write_per_million: 0.0,
    });
    app.usage_step_raw_payloads
        .insert(
            ("session".to_string(), "turn-1".to_string(), 1),
            Ok(Some(serde_json::json!({"stale": true}))),
        );
    app.usage_step_raw_loading.insert(("session".to_string(), "turn-2".to_string(), 2));
    app.usage_step_raw_modal =
        Some(crate::app::state::UsageStepRawModal { step_index: 1, content: "{}".to_string() });

    let _ = super::update(&mut app, crate::app::message::ViewMessage::OpenUsage);

    assert_eq!(app.open_tabs[1], tab("usage", "用量", crate::app::Screen::Usage));
    assert_eq!(app.active_tab_id.as_deref(), Some("usage"));
    assert_eq!(app.screen, crate::app::Screen::Usage);
    assert!(app.usage_model_info.is_none());
    assert!(app.usage_step_raw_payloads.is_empty());
    assert!(app.usage_step_raw_loading.is_empty());
    assert!(app.usage_step_raw_modal.is_none());
}

#[test]
fn open_usage_moves_existing_tab_directly_after_home() {
    let (mut app, _task) = crate::app::App::new();
    app.active_session_id = None;
    app.open_tabs.push(tab("other", "其他", crate::app::Screen::Apps));
    app.open_tabs.push(tab("usage", "已有用量", crate::app::Screen::Usage));

    let _ = super::update(&mut app, crate::app::message::ViewMessage::OpenUsage);

    let ids: Vec<&str> = app.open_tabs.iter().map(|tab| tab.id.as_str()).collect();
    assert_eq!(ids, ["home", "usage", "other"]);
    assert_eq!(app.open_tabs[1].title, "已有用量");
}

#[test]
fn open_usage_inserts_first_when_home_tab_is_absent() {
    let (mut app, _task) = crate::app::App::new();
    app.active_session_id = None;
    app.open_tabs = vec![tab("other", "其他", crate::app::Screen::Apps)];

    let _ = super::update(&mut app, crate::app::message::ViewMessage::OpenUsage);

    let ids: Vec<&str> = app.open_tabs.iter().map(|tab| tab.id.as_str()).collect();
    assert_eq!(ids, ["usage", "other"]);
    assert_eq!(app.active_tab_id.as_deref(), Some("usage"));
}

#[test]
fn usage_provider_picker_returns_none_without_candidates() {
    assert_eq!(super::pick_usage_provider_id(&[], "gpt-5"), None);
}

#[test]
fn usage_provider_picker_prefers_exact_model_prefix_before_fuzzy_match() {
    let candidates = ids(&["openrouter", "anthropic", "custom"]);

    assert_eq!(
        super::pick_usage_provider_id(&candidates, "anthropic/claude-opus"),
        Some("anthropic".to_string())
    );
}

#[test]
fn usage_provider_picker_matches_prefix_inside_provider_id_case_insensitively() {
    let candidates = ids(&["custom-anthropic-prod", "openai"]);

    assert_eq!(
        super::pick_usage_provider_id(&candidates, "Anthropic/claude-opus"),
        Some("custom-anthropic-prod".to_string())
    );
}

#[test]
fn usage_provider_picker_prefers_known_provider_families() {
    let candidates = ids(&["fallback", "team-openai", "anthropic-main", "deepseek-direct"]);

    assert_eq!(
        super::pick_usage_provider_id(&candidates, "gpt-5"),
        Some("team-openai".to_string())
    );
    assert_eq!(
        super::pick_usage_provider_id(&candidates, "claude-opus"),
        Some("anthropic-main".to_string())
    );
    assert_eq!(
        super::pick_usage_provider_id(&candidates, "deepseek-chat"),
        Some("deepseek-direct".to_string())
    );
}

#[test]
fn usage_provider_picker_prefers_openrouter_for_deepseek_when_direct_provider_is_absent() {
    let candidates = ids(&["fallback", "openrouter-models"]);

    assert_eq!(
        super::pick_usage_provider_id(&candidates, "deepseek-chat"),
        Some("openrouter-models".to_string())
    );
}

#[test]
fn usage_provider_picker_falls_back_to_first_candidate() {
    let candidates = ids(&["first", "second"]);

    assert_eq!(
        super::pick_usage_provider_id(&candidates, "unknown-model"),
        Some("first".to_string())
    );
}

#[test]
fn open_knowledge_closes_apps_and_starts_initial_loads() {
    let (mut app, _task) = crate::app::App::new();
    app.open_tabs.push(tab("apps", "应用", crate::app::Screen::Apps));

    let _ = super::update(&mut app, crate::app::message::ViewMessage::OpenKnowledge);

    assert!(!app.open_tabs.iter().any(|tab| tab.id == "apps"));
    assert_eq!(app.active_tab_id.as_deref(), Some("knowledge"));
    assert_eq!(app.screen, crate::app::Screen::Knowledge);
    assert!(app.knowledge.loading_status);
    assert!(app.knowledge.loading_embedding_status);
    assert!(app.knowledge.loading_embedding_presets);
    assert!(app.knowledge.loading_datasets);
}

#[test]
fn open_knowledge_reuses_existing_tab_without_duplicate() {
    let (mut app, _task) = crate::app::App::new();
    app.open_tabs.push(tab("apps", "应用", crate::app::Screen::Apps));
    app.open_tabs.push(tab("knowledge", "已有知识库", crate::app::Screen::Knowledge));

    let _ = super::update(&mut app, crate::app::message::ViewMessage::OpenKnowledge);

    assert_eq!(app.open_tabs.iter().filter(|tab| tab.id == "knowledge").count(), 1);
    assert_eq!(
        app.open_tabs.iter().find(|tab| tab.id == "knowledge").map(|tab| tab.title.as_str()),
        Some("已有知识库")
    );
    assert_eq!(app.active_tab_id.as_deref(), Some("knowledge"));
}

#[test]
fn open_workflow_tool_closes_apps_and_initializes_saved_apps() {
    let (mut app, _task) = crate::app::App::new();
    app.open_tabs.push(tab("apps", "应用", crate::app::Screen::Apps));

    let _ = super::update(&mut app, crate::app::message::ViewMessage::OpenWorkflowTool);

    assert!(!app.open_tabs.iter().any(|tab| tab.id == "apps"));
    assert_eq!(app.active_tab_id.as_deref(), Some(crate::apps::workflow::WORKFLOW_TOOL_TAB_ID));
    assert_eq!(app.screen, crate::app::Screen::WorkflowTool);
    assert!(app.workflow_state.saved_apps_loading);
}

#[test]
fn open_workflow_manager_resets_stage_actions_selection() {
    let (mut app, _task) = crate::app::App::new();
    app.open_tabs.push(tab("apps", "应用", crate::app::Screen::Apps));
    let _ =
        super::update(&mut app, crate::app::message::ViewMessage::OpenWorkflowManagerStageActions);

    let _ = super::update(&mut app, crate::app::message::ViewMessage::OpenWorkflowManager);

    assert!(!app.open_tabs.iter().any(|tab| tab.id == "apps"));
    assert_eq!(app.active_tab_id.as_deref(), Some("workflow_manager"));
    assert_eq!(app.screen, crate::app::Screen::WorkflowManager);
    assert_eq!(
        app.workflow_manager.active_tab(),
        crate::app::views::workflow_manager::WorkflowManagerTab::LifecycleDefinition
    );
    assert_eq!(app.open_tabs.iter().filter(|tab| tab.id == "workflow_manager").count(), 1);
}

#[test]
fn apps_open_most_recent_without_projects_keeps_navigation_unchanged() {
    let (mut app, _task) = crate::app::App::new();
    app.recent_projects.clear();
    app.open_tabs.push(tab("apps", "应用", crate::app::Screen::Apps));
    app.active_tab_id = Some("apps".to_string());
    let tabs_before = app.open_tabs.clone();

    let _ = super::update(&mut app, crate::app::message::ViewMessage::AppsOpenMostRecent);

    assert_eq!(app.open_tabs, tabs_before);
    assert_eq!(app.active_tab_id.as_deref(), Some("apps"));
}

#[test]
fn apps_open_most_recent_closes_apps_and_activates_last_remaining_tab() {
    let (mut app, _task) = crate::app::App::new();
    app.recent_projects = vec!["/project/first".to_string(), "/project/second".to_string()];
    app.open_tabs.push(tab("other", "其他", crate::app::Screen::Apps));
    app.open_tabs.push(tab("apps", "应用", crate::app::Screen::Apps));
    app.active_tab_id = Some("apps".to_string());

    let _ = super::update(&mut app, crate::app::message::ViewMessage::AppsOpenMostRecent);

    assert!(!app.open_tabs.iter().any(|tab| tab.id == "apps"));
    assert_eq!(app.active_tab_id.as_deref(), Some("other"));
    assert_eq!(app.recent_projects[0], "/project/first");
}

#[test]
fn apps_search_changed_preserves_exact_query_including_whitespace() {
    let (mut app, _task) = crate::app::App::new();

    let _ = super::update(
        &mut app,
        crate::app::message::ViewMessage::AppsSearchChanged("  工作流 🔎  ".to_string()),
    );

    assert_eq!(app.apps_search_query, "  工作流 🔎  ");
}

#[test]
fn unrelated_message_is_safely_ignored() {
    let (mut app, _task) = crate::app::App::new();
    app.apps_search_query = "unchanged".to_string();
    let tabs_before = app.open_tabs.clone();

    let _ = super::update(&mut app, crate::app::message::ViewMessage::ToggleSettingsPanel);

    assert_eq!(app.apps_search_query, "unchanged");
    assert_eq!(app.open_tabs, tabs_before);
}

#[test]
fn apps_panel_has_no_removed_tool_route() {
    let source = include_str!("apps.rs");

    assert!(!source.contains(concat!("ViewMessage::Open", "Design")));
    assert!(!source.contains(concat!("Screen", "::", "Design")));
}

#[test]
fn desktop_apps_catalog_excludes_migrated_tools() {
    let tile_source = include_str!("../../../views/apps/tiles.rs");
    let panel_source = include_str!("apps.rs");
    let migrated_titles = [
        "JSON工具",
        "JSON/YAML互转工具",
        "SQL美化工具",
        "HTML美化工具",
        "JSON比对工具",
        "Markdown编辑器",
        "随机密码生成器",
        "进制转换器",
        "时间戳转换器",
        "二维码生成器",
        "颜色转换工具",
    ];
    let migrated_routes = [
        concat!("Open", "Json", "Tool"),
        concat!("Open", "Json", "Yaml", "Tool"),
        concat!("Open", "Sql", "Tool"),
        concat!("Open", "Html", "Tool"),
        concat!("Open", "Json", "Diff", "Tool"),
        concat!("Open", "Markdown", "Tool"),
        concat!("Open", "Password", "Tool"),
        concat!("Open", "Base", "Tool"),
        concat!("Open", "Timestamp", "Tool"),
        concat!("Open", "Qr", "Tool"),
        concat!("Open", "Color", "Tool"),
    ];

    for title in migrated_titles {
        assert!(!tile_source.contains(title), "{title} remains in desktop app tiles");
    }

    for route in migrated_routes {
        assert!(!tile_source.contains(route), "{route} remains in desktop app tiles");
        assert!(!panel_source.contains(route), "{route} remains in desktop panel routes");
    }
}

#[test]
fn open_workflow_manager_stage_actions_selects_stage_actions_tab() {
    let (mut app, _task) = crate::app::App::new();

    let task =
        super::update(&mut app, crate::app::message::ViewMessage::OpenWorkflowManagerStageActions);
    let _ = std::hint::black_box(task);

    assert_eq!(app.screen, crate::app::Screen::WorkflowManager);
    assert_eq!(
        app.workflow_manager.active_tab(),
        crate::app::views::workflow_manager::WorkflowManagerTab::StageActions
    );
}
