//! 员工编排设置测试，验证网关配置投影、未保存变更检测和能力绑定往返。
//! 转换时仅修剪可选文本并报告非法数值，不应静默接受不完整成员配置。

use super::StaffSettingsState;
use crate::app::App;
use vw_config_types::staff::{
    StaffCapabilitiesConfig, StaffChannelBindingConfig, StaffConfig, StaffMemberConfig,
    StaffProjectStateConfig, StaffStageAssignmentConfig, StaffWorkflowStageConfig,
};

#[test]
fn staff_settings_state_builds_from_gateway_config() {
    let config = StaffConfig {
        members: vec![StaffMemberConfig {
            id: "engineer".to_string(),
            name: "Engineer".to_string(),
            role: "implementation".to_string(),
            level: "senior".to_string(),
            status: "online".to_string(),
            enabled: true,
            capacity: 2,
            delegate_agent: Some("coder".to_string()),
            acp_agent: None,
            ipc_agent_id: None,
            channel_bindings: Vec::new(),
            capabilities: StaffCapabilitiesConfig {
                agentic: true,
                allowed_tools: vec!["shell".to_string(), "apply_patch".to_string()],
                allowed_skills: vec!["rustcodegraph".to_string()],
                max_iterations: 12,
                max_parallel_tasks: Some(3),
                max_active_minutes: Some(45),
            },
        }],
        workflow_stages: vec![StaffWorkflowStageConfig {
            id: "running".to_string(),
            name: "Running".to_string(),
            task_status: "running".to_string(),
            role: Some("implementation".to_string()),
            enabled: true,
            display_only: false,
        }],
        workflow_orchestrations: Default::default(),
        project_state: StaffProjectStateConfig {
            capabilities: Vec::new(),
            stage_assignments: vec![StaffStageAssignmentConfig {
                stage_id: "running".to_string(),
                primary_member_id: Some("engineer".to_string()),
                backup_member_ids: Vec::new(),
                manager_member_ids: Vec::new(),
                autonomous: true,
            }],
            stage_workflow_bindings: Vec::new(),
            stage_actions: Vec::new(),
            member_task_assignments: Vec::new(),
        },
    };

    let state = StaffSettingsState::from_config(config.clone());

    assert!(!state.loading);
    assert_eq!(state.config, config);
    assert_eq!(state.selected_member_id.as_deref(), Some("engineer"));
    assert_eq!(state.member_drafts[0].delegate_agent.as_deref(), Some("coder"));
    assert_eq!(state.member_drafts[0].capacity_input, "2");
    assert!(state.capability_drafts[0].agentic);
    assert_eq!(state.capability_drafts[0].allowed_tools, vec!["shell", "apply_patch"]);
    assert_eq!(state.capability_drafts[0].allowed_skills, vec!["rustcodegraph"]);
    assert_eq!(state.capability_drafts[0].max_iterations_input, "12");
    assert_eq!(state.capability_drafts[0].max_parallel_tasks_input, "3");
    assert_eq!(state.capability_drafts[0].max_active_minutes_input, "45");
    assert!(state.save_error.is_none());
    assert!(!state.save_success);
    assert!(!state.has_unsaved_changes());
}

#[test]
fn staff_settings_state_detects_unsaved_changes() {
    let config = StaffConfig {
        members: vec![StaffMemberConfig {
            id: "engineer".to_string(),
            name: "Engineer".to_string(),
            role: "implementation".to_string(),
            level: "senior".to_string(),
            status: "online".to_string(),
            enabled: true,
            capacity: 2,
            delegate_agent: Some("coder".to_string()),
            acp_agent: None,
            ipc_agent_id: None,
            channel_bindings: Vec::new(),
            capabilities: StaffCapabilitiesConfig::default(),
        }],
        ..StaffConfig::default()
    }
    .normalize()
    .expect("config normalizes");
    let mut state = StaffSettingsState::from_config(config.clone());

    assert!(!state.has_unsaved_changes());

    state.member_drafts[0].capacity_input = "5".to_string();
    assert!(state.has_unsaved_changes());

    state.replace_config(config);
    assert!(!state.has_unsaved_changes());
}

#[test]
fn staff_settings_state_treats_invalid_draft_as_changed() {
    let mut state = StaffSettingsState::from_config(StaffConfig::default());
    state.member_drafts[0].capacity_input = "0".to_string();

    assert!(state.has_unsaved_changes());
}

#[test]
fn staff_capability_draft_saves_independent_staff_capabilities() {
    let (mut app, _) = App::new();
    let agents_snapshot = app
        .agents_settings
        .entries
        .iter()
        .map(|entry| {
            (
                entry.key.clone(),
                entry.agentic,
                entry.allowed_tools.clone(),
                entry.allowed_skills.clone(),
                entry.max_iterations,
            )
        })
        .collect::<Vec<_>>();

    app.staff_settings = StaffSettingsState::from_config(StaffConfig {
        members: vec![StaffMemberConfig {
            id: "engineer".to_string(),
            name: "Engineer".to_string(),
            role: "implementation".to_string(),
            level: "senior".to_string(),
            status: "online".to_string(),
            enabled: true,
            capacity: 2,
            delegate_agent: Some("coder".to_string()),
            acp_agent: None,
            ipc_agent_id: None,
            channel_bindings: Vec::new(),
            capabilities: StaffCapabilitiesConfig::default(),
        }],
        ..StaffConfig::default()
    });

    let draft = &mut app.staff_settings.capability_drafts[0];
    draft.agentic = true;
    draft.allowed_tools = vec![
        " shell ".to_string(),
        "apply_patch".to_string(),
        "shell".to_string(),
        " ".to_string(),
    ];
    draft.allowed_skills = vec![
        " rustcodegraph ".to_string(),
        "do-task-plans".to_string(),
        "rustcodegraph".to_string(),
        "".to_string(),
    ];
    draft.max_iterations_input = "12".to_string();
    draft.max_parallel_tasks_input = "3".to_string();
    draft.max_active_minutes_input = "45".to_string();

    assert_eq!(
        super::normalized_capability_allowlist_for_save(&draft.allowed_tools),
        vec!["apply_patch", "shell"],
    );

    let config = app.staff_settings.to_config().expect("staff config saves");
    let capabilities = &config.members[0].capabilities;
    assert!(capabilities.agentic);
    assert_eq!(capabilities.allowed_tools, vec!["apply_patch", "shell"]);
    assert_eq!(capabilities.allowed_skills, vec!["do-task-plans", "rustcodegraph"]);
    assert_eq!(capabilities.max_iterations, 12);
    assert_eq!(capabilities.max_parallel_tasks, Some(3));
    assert_eq!(capabilities.max_active_minutes, Some(45));

    let agents_after = app
        .agents_settings
        .entries
        .iter()
        .map(|entry| {
            (
                entry.key.clone(),
                entry.agentic,
                entry.allowed_tools.clone(),
                entry.allowed_skills.clone(),
                entry.max_iterations,
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(agents_after, agents_snapshot);
}

#[test]
fn staff_channel_binding_staff_settings_state_round_trips_channel_bindings() {
    let config = StaffConfig {
        members: vec![StaffMemberConfig {
            id: "planner".to_string(),
            name: "Planner".to_string(),
            role: "planner".to_string(),
            level: "manager".to_string(),
            status: "online".to_string(),
            enabled: true,
            capacity: 1,
            delegate_agent: Some("coder".to_string()),
            acp_agent: None,
            ipc_agent_id: None,
            channel_bindings: vec![
                StaffChannelBindingConfig {
                    channel_type: "matrix".to_string(),
                    instance_id: "planner".to_string(),
                },
                StaffChannelBindingConfig {
                    channel_type: "feishu".to_string(),
                    instance_id: "planner-app".to_string(),
                },
            ],
            capabilities: StaffCapabilitiesConfig::default(),
        }],
        ..StaffConfig::default()
    }
    .normalize()
    .expect("config normalizes");

    let mut state = StaffSettingsState::from_config(config);

    assert_eq!(state.member_drafts[0].channel_bindings.len(), 2);
    assert_eq!(state.member_drafts[0].channel_bindings[0].channel_type, "matrix");
    assert_eq!(state.member_drafts[0].channel_bindings[0].instance_id, "planner");

    state.member_drafts[0].channel_bindings.push(super::StaffSettingsChannelBindingDraft {
        channel_type: " dingtalk ".to_string(),
        instance_id: " ops ".to_string(),
    });

    let saved = state.to_config().expect("staff config saves");
    assert_eq!(
        saved.members[0].channel_bindings,
        vec![
            StaffChannelBindingConfig {
                channel_type: "matrix".to_string(),
                instance_id: "planner".to_string(),
            },
            StaffChannelBindingConfig {
                channel_type: "feishu".to_string(),
                instance_id: "planner-app".to_string(),
            },
            StaffChannelBindingConfig {
                channel_type: "dingtalk".to_string(),
                instance_id: "ops".to_string(),
            },
        ]
    );
}

#[test]
fn staff_settings_helpers_cover_tabs_builtin_members_and_binding_requirements() {
    for (tab, key) in [
        (super::StaffSettingsTab::Roster, "roster"),
        (super::StaffSettingsTab::Capabilities, "capabilities"),
        (super::StaffSettingsTab::Teams, "teams"),
        (super::StaffSettingsTab::Workers, "workers"),
    ] {
        assert_eq!(tab.key(), key);
        assert_eq!(super::StaffSettingsTab::from_key(key), Some(tab));
    }
    assert_eq!(super::StaffSettingsTab::from_key("unknown"), None);

    for member_id in
        ["staff-orchestrator", "staff-planner", "staff-executor", "staff-reviewer", "staff-merger"]
    {
        assert!(super::is_builtin_staff_member_id(member_id));
    }
    assert!(!super::is_builtin_staff_member_id("staff-observer"));

    let mut draft = super::StaffSettingsMemberDraft::new("custom".to_string());
    draft.delegate_agent = Some("  ".to_string());
    assert!(super::staff_member_needs_agent_binding(&draft));
    draft.acp_agent = Some(" codex ".to_string());
    assert!(!super::staff_member_needs_agent_binding(&draft));
    draft.enabled = false;
    draft.acp_agent = None;
    assert!(!super::staff_member_needs_agent_binding(&draft));
}

#[test]
fn staff_settings_state_handles_empty_config_and_new_member_defaults() {
    let empty = StaffConfig { members: Vec::new(), ..StaffConfig::default() };
    let state = StaffSettingsState::from_config(empty);

    assert!(state.selected_member_id.is_none());
    assert!(state.member_drafts.is_empty());
    assert!(state.capability_drafts.is_empty());
    assert_eq!(state.active_tab, super::StaffSettingsTab::Roster);

    let draft = super::StaffSettingsMemberDraft::new("analyst".to_string());
    assert_eq!(draft.id, "analyst");
    assert_eq!(draft.name, "analyst");
    assert_eq!(draft.role, "member");
    assert_eq!(draft.capacity_input, "1");
    assert_eq!(draft.delegate_agent.as_deref(), Some("main"));
    assert!(draft.channel_bindings.is_empty());

    let binding = super::StaffSettingsChannelBindingDraft::new();
    assert_eq!(binding.channel_type, "matrix");
    assert_eq!(binding.instance_id, "default");
}

#[test]
fn staff_settings_to_config_trims_optional_agent_fields_and_fills_missing_capabilities() {
    let mut state = StaffSettingsState::from_config(StaffConfig {
        members: vec![StaffMemberConfig {
            id: " engineer ".to_string(),
            name: " Engineer ".to_string(),
            role: " implementation ".to_string(),
            level: " senior ".to_string(),
            status: " online ".to_string(),
            enabled: true,
            capacity: 2,
            delegate_agent: Some(" coder ".to_string()),
            acp_agent: Some(" ".to_string()),
            ipc_agent_id: Some(" ipc-1 ".to_string()),
            channel_bindings: Vec::new(),
            capabilities: StaffCapabilitiesConfig::default(),
        }],
        ..StaffConfig::default()
    });
    state.capability_drafts.clear();

    let config = state.to_config().expect("staff config saves");
    let member = &config.members[0];

    assert_eq!(member.id, "engineer");
    assert_eq!(member.name, "Engineer");
    assert_eq!(member.role, "implementation");
    assert_eq!(member.level, "senior");
    assert_eq!(member.status, "online");
    assert_eq!(member.delegate_agent.as_deref(), Some("coder"));
    assert_eq!(member.acp_agent, None);
    assert_eq!(member.ipc_agent_id.as_deref(), Some("ipc-1"));
    assert_eq!(member.capabilities, StaffCapabilitiesConfig::default());
}

#[test]
fn staff_settings_to_config_reports_invalid_numeric_inputs() {
    let mut state = StaffSettingsState::from_config(StaffConfig::default());

    state.member_drafts[0].capacity_input = "not-a-number".to_string();
    assert_eq!(state.to_config().unwrap_err(), "staff capacity must be a positive integer");

    state.member_drafts[0].capacity_input = "1".to_string();
    state.capability_drafts[0].max_iterations_input = "many".to_string();
    assert_eq!(state.to_config().unwrap_err(), "staff max iterations must be a positive integer");

    state.capability_drafts[0].max_iterations_input = "1".to_string();
    state.capability_drafts[0].max_parallel_tasks_input = "many".to_string();
    assert_eq!(
        state.to_config().unwrap_err(),
        "staff max parallel tasks must be a positive integer"
    );

    state.capability_drafts[0].max_parallel_tasks_input.clear();
    state.capability_drafts[0].max_active_minutes_input = "many".to_string();
    assert_eq!(
        state.to_config().unwrap_err(),
        "staff max active minutes must be a positive integer"
    );
}

#[test]
fn optional_and_numeric_input_helpers_trim_and_parse_boundaries() {
    assert_eq!(super::optional_trimmed(&None), None);
    assert_eq!(super::optional_trimmed(&Some("  ".to_string())), None);
    assert_eq!(super::optional_trimmed(&Some(" value ".to_string())), Some("value".to_string()));
    assert!(!super::optional_has_value(&Some("  ".to_string())));
    assert!(super::optional_has_value(&Some("value".to_string())));

    assert_eq!(super::parse_u32_input(" 7 ", "count"), Ok(7));
    assert_eq!(super::parse_usize_input(" 9 ", "iterations"), Ok(9));
    assert_eq!(super::parse_optional_u32_input(" ", "minutes"), Ok(None));
    assert_eq!(super::parse_optional_u32_input(" 15 ", "minutes"), Ok(Some(15)));
}
