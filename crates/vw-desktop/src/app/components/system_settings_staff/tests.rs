use crate::app::App;
use crate::app::state::{
    SkillsCatalogItem, SkillsCatalogKind, StaffSettingsCapabilitiesDraft,
    StaffSettingsChannelBindingDraft, StaffSettingsMemberDraft, StaffSettingsState,
    StaffSettingsTab,
};
use iced::widget::button;
use iced::{Background, Theme};
use vw_config_types::channels::{
    DingTalkConfig, DingTalkInstanceConfig, FeishuConfig, FeishuInstanceConfig, MatrixConfig,
    MatrixInstanceConfig,
};
use vw_config_types::staff::{StaffChannelBindingConfig, StaffConfig, StaffMemberConfig};

#[test]
fn staff_settings_view_shows_bound_agent_and_unbound_warning() {
    let (mut app, _) = App::new();
    app.agents_settings.entries.clear();
    app.acp_agents = vec!["codex".to_string()];
    let mut config = StaffConfig {
        members: vec![
            StaffMemberConfig {
                id: "builder".to_string(),
                name: "Builder".to_string(),
                role: "implementation".to_string(),
                level: "senior".to_string(),
                status: "online".to_string(),
                enabled: true,
                capacity: 2,
                delegate_agent: Some("coder".to_string()),
                acp_agent: None,
                ipc_agent_id: None,
                channel_bindings: Vec::new(),
                capabilities: Default::default(),
            },
            StaffMemberConfig {
                id: "floating".to_string(),
                name: "Floating".to_string(),
                role: "review".to_string(),
                level: "member".to_string(),
                status: "online".to_string(),
                enabled: true,
                capacity: 1,
                delegate_agent: Some("main".to_string()),
                acp_agent: None,
                ipc_agent_id: None,
                channel_bindings: Vec::new(),
                capabilities: Default::default(),
            },
        ],
        ..StaffConfig::default()
    };
    config = config.normalize().expect("staff config normalizes");
    app.staff_settings = StaffSettingsState::from_config(config);
    app.staff_settings.member_drafts[1].delegate_agent = None;
    app.staff_settings.member_drafts[1].acp_agent = None;

    let rows = super::roster_member_summaries(&app);
    assert_eq!(rows[0].binding_label, "委托代理 coder");
    assert_eq!(rows[0].role_label, "自定义：implementation");
    assert_eq!(rows[1].status_label, "在线");
    assert!(rows[1].shows_unbound_warning);
    assert!(super::selected_member_save_blocked(&StaffSettingsMemberDraft {
        delegate_agent: None,
        acp_agent: None,
        enabled: true,
        ..app.staff_settings.member_drafts[1].clone()
    }));

    let _: iced::Element<'_, crate::app::Message> = super::view(&app);
}

#[test]
fn staff_save_error_banner_follows_tabs_and_precedes_validation() {
    let (mut app, _) = App::new();

    for member in &mut app.staff_settings.member_drafts {
        member.delegate_agent = Some("main".to_string());
    }
    app.staff_settings.save_success = false;
    app.staff_settings.save_error = Some("staff save failed".to_string());
    assert_eq!(super::view(&app).as_widget().children().len(), 5);

    let member = app.staff_settings.member_drafts.first_mut().expect("default staff member");
    member.enabled = true;
    member.delegate_agent = None;
    member.acp_agent = None;
    assert_eq!(super::view(&app).as_widget().children().len(), 6);

    for tab in [StaffSettingsTab::Teams, StaffSettingsTab::Workers] {
        app.staff_settings.active_tab = tab;
        assert_eq!(super::view(&app).as_widget().children().len(), 4);
    }
}

#[test]
fn staff_channel_binding_staff_settings_view_builds_channel_binding_controls() {
    let (mut app, _) = App::new();
    app.channels_settings.matrix = Some(MatrixConfig {
        homeserver: "https://matrix.example.test".to_string(),
        access_token: "matrix-token".to_string(),
        user_id: None,
        device_id: None,
        room_id: "!default:matrix.example.test".to_string(),
        allowed_users: Vec::new(),
        mention_only: false,
    });
    app.channels_settings.matrix_instances.push(MatrixInstanceConfig {
        id: "planner".to_string(),
        config: MatrixConfig {
            homeserver: "https://matrix.example.test".to_string(),
            access_token: "matrix-planner-token".to_string(),
            user_id: None,
            device_id: None,
            room_id: "!planner:matrix.example.test".to_string(),
            allowed_users: Vec::new(),
            mention_only: false,
        },
    });
    app.channels_settings.feishu_instances.push(FeishuInstanceConfig {
        id: "planner-app".to_string(),
        config: FeishuConfig {
            app_id: "feishu-planner".to_string(),
            app_secret: "feishu-secret".to_string(),
            verification_token: None,
            encrypt_key: None,
            allowed_users: Vec::new(),
            group_reply: None,
            receive_mode: Default::default(),
            port: None,
            draft_update_interval_ms: 500,
            max_draft_edits: 5,
        },
    });
    app.channels_settings.dingtalk_instances.push(DingTalkInstanceConfig {
        id: "ops".to_string(),
        config: DingTalkConfig {
            client_id: "dingtalk-client".to_string(),
            client_secret: "dingtalk-secret".to_string(),
            allowed_users: Vec::new(),
        },
    });

    let mut config = StaffConfig::default();
    let planner = config
        .members
        .iter_mut()
        .find(|member| member.id == "staff-planner")
        .expect("default planner");
    planner.channel_bindings = vec![StaffChannelBindingConfig {
        channel_type: "matrix".to_string(),
        instance_id: "planner".to_string(),
    }];
    app.staff_settings = StaffSettingsState::from_config(config.normalize().expect("staff config"));

    let draft = app
        .staff_settings
        .member_drafts
        .iter()
        .find(|draft| draft.id == "staff-planner")
        .expect("planner draft");
    let options = super::channel_binding_profile_options(&app, draft);

    assert!(options.contains(&"matrix:default".to_string()));
    assert!(options.contains(&"matrix:planner".to_string()));
    assert!(options.contains(&"feishu:planner-app".to_string()));
    assert!(options.contains(&"dingtalk:ops".to_string()));
    assert_eq!(super::channel_binding_summary_label(draft), "Matrix planner");

    let _: iced::Element<'_, crate::app::Message> = super::view(&app);
}

#[test]
fn staff_setting_pick_options_display_chinese_and_save_english_keys() {
    assert_eq!(
        super::staff_option_label(super::STAFF_ROLE_OPTIONS, "executor", "未设置角色"),
        "执行"
    );
    assert_eq!(
        super::staff_option_label(super::STAFF_LEVEL_OPTIONS, "manager", "未设置层级"),
        "主管"
    );
    assert_eq!(
        super::staff_option_label(super::STAFF_STATUS_OPTIONS, "blocked", "未设置状态"),
        "阻塞"
    );

    assert_eq!(
        super::staff_option_value_from_label(
            super::STAFF_ROLE_OPTIONS,
            "评审".to_string(),
            "未设置角色",
        ),
        "reviewer"
    );
    assert_eq!(
        super::staff_option_value_from_label(
            super::STAFF_LEVEL_OPTIONS,
            "执行者".to_string(),
            "未设置层级",
        ),
        "executor"
    );
    assert_eq!(
        super::staff_option_value_from_label(
            super::STAFF_STATUS_OPTIONS,
            "工作中".to_string(),
            "未设置状态",
        ),
        "working"
    );
    assert_eq!(
        super::staff_option_value_from_label(
            super::STAFF_ROLE_OPTIONS,
            "未设置角色".to_string(),
            "未设置角色",
        ),
        ""
    );
}

#[test]
fn staff_setting_pick_options_preserve_custom_existing_values() {
    let labels =
        super::staff_option_labels(super::STAFF_ROLE_OPTIONS, "implementation", "未设置角色");

    assert!(labels.contains(&"编排".to_string()));
    assert!(labels.contains(&"自定义：implementation".to_string()));
    assert_eq!(
        super::staff_option_value_from_label(
            super::STAFF_ROLE_OPTIONS,
            "自定义：implementation".to_string(),
            "未设置角色",
        ),
        "implementation"
    );
}

#[test]
fn normalized_ids_trims_sorts_and_deduplicates_non_empty_values() {
    let values = vec![
        " beta ".to_string(),
        String::new(),
        "alpha".to_string(),
        "beta".to_string(),
        "   ".to_string(),
    ];

    assert_eq!(super::normalized_ids(&values), vec!["alpha", "beta"]);
}

#[test]
fn effective_selected_count_ignores_blank_duplicate_and_unavailable_ids() {
    let selected = vec![
        " tool-b ".to_string(),
        "tool-a".to_string(),
        "tool-a".to_string(),
        "missing".to_string(),
        String::new(),
    ];
    let available = vec!["tool-a".to_string(), "tool-b".to_string()];

    assert_eq!(super::effective_selected_count(&selected, &available), 2);
}

#[test]
fn agent_labels_normalize_blank_values_and_keep_unbound_first() {
    assert_eq!(super::selected_agent_label(&None), super::UNBOUND_AGENT_LABEL);
    assert_eq!(super::selected_agent_label(&Some("  coder  ".to_string())), "coder");
    assert_eq!(super::agent_value_from_label("   ".to_string()), None);
    assert_eq!(super::agent_value_from_label("  coder  ".to_string()), Some("coder".to_string()));
    assert_eq!(
        super::keep_unbound_first(vec![
            "coder".to_string(),
            super::UNBOUND_AGENT_LABEL.to_string(),
            "reviewer".to_string(),
            super::UNBOUND_AGENT_LABEL.to_string(),
        ]),
        vec![super::UNBOUND_AGENT_LABEL, "coder", "reviewer"]
    );
}

#[test]
fn staff_save_action_disabled_style_is_muted() {
    let theme = Theme::Dark;
    let active = super::save_action_btn_style(&theme, button::Status::Active);
    let disabled = super::save_action_btn_style(&theme, button::Status::Disabled);

    assert!(disabled.text_color.a < active.text_color.a);
    let Some(Background::Color(disabled_bg)) = disabled.background else {
        panic!("disabled save button should have background");
    };
    let Some(Background::Color(active_bg)) = active.background else {
        panic!("active save button should have background");
    };
    assert!(disabled_bg.a < active_bg.a);
}

#[test]
fn staff_teams_and_workers_tabs_build_independent_gateway_management_views() {
    let (mut app, _) = App::new();
    app.project_path = Some("/tmp/vibewindow-staff-resources".to_string());

    assert_eq!(super::staff_tab_label(StaffSettingsTab::Roster), "名册");
    assert_eq!(super::staff_tab_label(StaffSettingsTab::Capabilities), "能力");
    assert_eq!(super::staff_tab_label(StaffSettingsTab::Teams), "Teams");
    assert_eq!(super::staff_tab_label(StaffSettingsTab::Workers), "Workers");

    app.staff_settings.active_tab = StaffSettingsTab::Teams;
    let _: iced::Element<'_, crate::app::Message> = super::view(&app);
    app.staff_settings.teams.loading = true;
    let _: iced::Element<'_, crate::app::Message> = super::view(&app);
    app.staff_settings.teams.loading = false;
    app.staff_settings.teams.error = Some("team load failed".to_string());
    let _: iced::Element<'_, crate::app::Message> = super::view(&app);
    app.staff_settings.teams.start_new();
    let _: iced::Element<'_, crate::app::Message> = super::view(&app);

    app.staff_settings.active_tab = StaffSettingsTab::Workers;
    let _: iced::Element<'_, crate::app::Message> = super::view(&app);
    app.staff_settings.workers.loading = true;
    let _: iced::Element<'_, crate::app::Message> = super::view(&app);
    app.staff_settings.workers.loading = false;
    app.staff_settings.workers.error = Some("worker load failed".to_string());
    let _: iced::Element<'_, crate::app::Message> = super::view(&app);
    app.staff_settings.workers.start_new();
    let _: iced::Element<'_, crate::app::Message> = super::view(&app);

    assert!(app.staff_settings.teams.draft.is_some());
    assert!(app.staff_settings.workers.draft.is_some());
}

#[test]
fn selected_member_helpers_fall_back_to_first_existing_member_and_empty_detail_panel() {
    let (mut app, _) = App::new();
    let mut config = StaffConfig {
        members: vec![
            StaffMemberConfig {
                id: "alpha".to_string(),
                name: "Alpha".to_string(),
                role: "planner".to_string(),
                level: "manager".to_string(),
                status: "working".to_string(),
                enabled: false,
                capacity: 3,
                delegate_agent: None,
                acp_agent: Some("codex".to_string()),
                ipc_agent_id: None,
                channel_bindings: Vec::new(),
                capabilities: Default::default(),
            },
            StaffMemberConfig {
                id: "beta".to_string(),
                name: String::new(),
                role: String::new(),
                level: String::new(),
                status: String::new(),
                enabled: false,
                capacity: 0,
                delegate_agent: None,
                acp_agent: None,
                ipc_agent_id: None,
                channel_bindings: Vec::new(),
                capabilities: Default::default(),
            },
        ],
        ..StaffConfig::default()
    };
    config = config.normalize().expect("staff config normalizes");
    app.staff_settings = StaffSettingsState::from_config(config);
    app.staff_settings.member_drafts[1].name.clear();
    app.staff_settings.member_drafts[1].role.clear();
    app.staff_settings.member_drafts[1].status.clear();
    app.staff_settings.member_drafts[1].capacity_input.clear();
    app.staff_settings.selected_member_id = Some("missing".to_string());

    assert_eq!(super::selected_member_id(&app), Some("alpha".to_string()));
    let selected = super::selected_member_draft(&app).expect("first member selected");
    assert_eq!(selected.id, "alpha");

    let summaries = super::roster_member_summaries(&app);
    assert_eq!(summaries[0].binding_label, "ACP agent codex");
    assert_eq!(summaries[0].capacity_label, "容量 3");
    assert!(!summaries[0].enabled);
    assert_eq!(summaries[1].display_name, "beta");
    assert_eq!(summaries[1].role_label, "未设置角色");
    assert_eq!(summaries[1].status_label, "未设置状态");
    assert_eq!(summaries[1].capacity_label, "容量 0");

    app.staff_settings.member_drafts.clear();
    assert_eq!(super::selected_member_id(&app), None);
    assert!(super::selected_member_draft(&app).is_none());
    let _: iced::Element<'_, crate::app::Message> =
        super::detail_panel(&app, None, StaffSettingsTab::Roster, false, false);
    let _: iced::Element<'_, crate::app::Message> = super::view(&app);
}

#[test]
fn agent_option_lists_include_trimmed_selected_values_and_keep_unbound_first() {
    let (mut app, _) = App::new();
    app.agents_settings.entries.truncate(1);
    app.agents_settings.entries[0].key = "coder".to_string();
    app.acp_agents = vec!["codex".to_string(), "reviewer".to_string()];

    let delegate_options = super::delegate_agent_options(&app, &Some("  legacy  ".to_string()));
    assert_eq!(delegate_options.first().map(String::as_str), Some(super::UNBOUND_AGENT_LABEL));
    assert!(delegate_options.contains(&"coder".to_string()));
    assert!(delegate_options.contains(&"legacy".to_string()));

    let acp_options = super::acp_agent_options(&app, &Some("  external  ".to_string()));
    assert_eq!(acp_options.first().map(String::as_str), Some(super::UNBOUND_AGENT_LABEL));
    assert!(acp_options.contains(&"codex".to_string()));
    assert!(acp_options.contains(&"external".to_string()));
}

#[test]
fn delegate_agent_options_exclude_retired_builtin_agents() {
    let (mut app, _) = App::new();
    app.agents_settings.entries = vec![
        crate::app::state::DelegateAgentSettingsEntry::from_config("main", None),
        crate::app::state::DelegateAgentSettingsEntry::from_config("build", None),
    ];

    let options = super::delegate_agent_options(&app, &Some("build".to_string()));

    assert!(options.contains(&"main".to_string()));
    assert!(!options.contains(&"build".to_string()));
}

#[test]
fn channel_binding_profiles_skip_blank_values_and_preserve_custom_bindings() {
    let (mut app, _) = App::new();
    app.channels_settings.feishu = Some(FeishuConfig {
        app_id: "feishu-default".to_string(),
        app_secret: "feishu-secret".to_string(),
        verification_token: None,
        encrypt_key: None,
        allowed_users: Vec::new(),
        group_reply: None,
        receive_mode: Default::default(),
        port: None,
        draft_update_interval_ms: 500,
        max_draft_edits: 5,
    });

    let draft = StaffSettingsMemberDraft {
        id: "staff-custom".to_string(),
        channel_bindings: vec![
            StaffSettingsChannelBindingDraft {
                channel_type: " slack ".to_string(),
                instance_id: " prod ".to_string(),
            },
            StaffSettingsChannelBindingDraft {
                channel_type: "matrix".to_string(),
                instance_id: "   ".to_string(),
            },
        ],
        ..StaffSettingsMemberDraft::new("staff-custom".to_string())
    };

    let mut options = Vec::new();
    super::push_channel_profile_option(&mut options, " matrix ", " default ");
    super::push_channel_profile_option(&mut options, "", "default");
    super::push_channel_profile_option(&mut options, "feishu", "  ");
    assert_eq!(options, vec!["matrix:default"]);

    let profile_options = super::channel_binding_profile_options(&app, &draft);
    assert!(profile_options.contains(&"feishu:default".to_string()));
    assert!(profile_options.contains(&"slack:prod".to_string()));
    assert!(!profile_options.contains(&"matrix:".to_string()));
    assert_eq!(super::channel_binding_summary_label(&draft), "自定义：slack prod, Matrix default");
    assert_eq!(super::channel_type_display_label("  "), "自定义：");
}

#[test]
fn capability_sections_cover_empty_and_populated_tool_and_skill_catalogs() {
    let (mut app, _) = App::new();
    app.agents_settings.available_tools =
        vec!["write".to_string(), "shell".to_string(), "shell".to_string(), " ".to_string()];
    app.skills_settings.catalog = vec![
        skill_item("skill-b", "Skill B", true, true),
        skill_item("skill-disabled", "Disabled", true, false),
        skill_item("skill-a", "Skill A", true, true),
        skill_item("skill-remote", "Remote", false, true),
    ];

    let capabilities = StaffSettingsCapabilitiesDraft {
        member_id: "staff-executor".to_string(),
        agentic: true,
        allowed_tools: vec![" shell ".to_string(), "missing".to_string()],
        allowed_skills: vec!["skill-a".to_string(), "skill-remote".to_string()],
        max_iterations_input: "12".to_string(),
        max_parallel_tasks_input: String::new(),
        max_active_minutes_input: "45".to_string(),
    };

    assert_eq!(super::optional_limit_label("  "), "不限");
    assert_eq!(super::optional_limit_label(" 8 "), "8");
    assert_eq!(
        super::available_skill_items(&app)
            .into_iter()
            .map(|item| item.id.as_str())
            .collect::<Vec<_>>(),
        vec!["skill-a", "skill-b"]
    );

    let _: iced::Element<'_, crate::app::Message> =
        super::capability_tools_section(&app, &capabilities);
    let _: iced::Element<'_, crate::app::Message> =
        super::capability_skills_section(&app, &capabilities);
    let _: iced::Element<'_, crate::app::Message> =
        super::capability_detail_fields(&app, &StaffSettingsMemberDraft::new("missing".into()))
            .into();

    app.agents_settings.available_tools.clear();
    app.skills_settings.catalog.clear();
    app.skills_settings.loading = true;
    let _: iced::Element<'_, crate::app::Message> =
        super::capability_tools_section(&app, &capabilities);
    let _: iced::Element<'_, crate::app::Message> =
        super::capability_skills_section(&app, &capabilities);
}

#[test]
fn staff_view_builds_roster_and_capabilities_with_save_success_and_dirty_state() {
    let (mut app, _) = App::new();
    app.staff_settings.selected_member_id = Some("staff-executor".to_string());
    app.staff_settings.save_success = true;
    app.staff_settings.loading = true;
    app.staff_settings.member_drafts[0].name.push_str(" dirty");
    let _: iced::Element<'_, crate::app::Message> = super::view(&app);

    app.staff_settings.loading = false;
    app.staff_settings.active_tab = StaffSettingsTab::Capabilities;
    app.staff_settings.capability_drafts[0].allowed_tools.push("shell".to_string());
    let _: iced::Element<'_, crate::app::Message> = super::view(&app);
}

#[test]
fn staff_styles_reflect_theme_selection_warning_and_checked_states() {
    let dark = Theme::Dark;
    let light = Theme::Light;

    assert!(super::is_dark_theme(&dark));
    assert!(!super::is_dark_theme(&light));

    let checked = super::capability_choice_style(&dark, true);
    let unchecked = super::capability_choice_style(&dark, false);
    assert_ne!(checked.border.color, unchecked.border.color);

    let selected = super::roster_button_style(&dark, button::Status::Active, true, false);
    let warning = super::roster_button_style(&dark, button::Status::Hovered, false, true);
    let pressed = super::roster_button_style(&light, button::Status::Pressed, false, false);
    assert!(selected.shadow.blur_radius > 0.0);
    assert_ne!(warning.border.color, pressed.border.color);

    let selected_badge = super::badge_style(&dark, true);
    let normal_badge = super::badge_style(&light, false);
    assert_ne!(selected_badge.border.color, normal_badge.border.color);
}

fn skill_item(id: &str, title: &str, installed: bool, enabled: bool) -> SkillsCatalogItem {
    SkillsCatalogItem {
        id: id.to_string(),
        title: title.to_string(),
        description: String::new(),
        kind: SkillsCatalogKind::Personal,
        resource_count: 0,
        installed,
        enabled,
        source: String::new(),
        source_path: None,
    }
}
