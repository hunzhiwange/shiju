use super::*;
use vw_config_types::staff::{StaffCapabilitiesConfig, StaffConfig, StaffMemberConfig};

fn app() -> App {
    let mut app = App::new().0;
    app.staff_settings.member_drafts.clear();
    app.staff_settings.capability_drafts.clear();
    app.staff_settings.selected_member_id = None;
    app.staff_settings.save_error = Some("stale error".to_string());
    app.staff_settings.save_success = true;
    app
}

fn staff_message(message: StaffMessage) -> SettingsMessage {
    SettingsMessage::Staff(message)
}

fn member_config(id: &str) -> StaffMemberConfig {
    StaffMemberConfig {
        id: id.to_string(),
        name: format!("{id} name"),
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
    }
}

fn staff_config_with_member(id: &str) -> StaffConfig {
    StaffConfig { members: vec![member_config(id)], ..StaffConfig::default() }
}

#[test]
fn next_member_id_uses_base_then_first_available_suffix() {
    let mut app = app();
    assert_eq!(next_member_id(&app), "staff-member");

    for id in ["staff-member", "staff-member-2", "staff-member-4"] {
        app.staff_settings.member_drafts.push(StaffSettingsMemberDraft::new(id.to_string()));
    }

    assert_eq!(next_member_id(&app), "staff-member-3");
}

#[test]
fn update_string_list_trims_sorts_deduplicates_and_removes_values() {
    let mut values = vec!["zeta".to_string()];

    update_string_list(&mut values, "  alpha  ".to_string(), true);
    update_string_list(&mut values, "alpha".to_string(), true);
    update_string_list(&mut values, "   ".to_string(), true);
    assert_eq!(values, vec!["alpha", "zeta"]);

    update_string_list(&mut values, " alpha ".to_string(), false);
    assert_eq!(values, vec!["zeta"]);
}

#[test]
fn refresh_and_non_staff_messages_preserve_feedback_as_expected() {
    let mut app = app();

    let _task = update(&mut app, staff_message(StaffMessage::Refresh));

    assert!(app.staff_settings.loading);
    assert_eq!(app.staff_settings.save_error, None);
    assert!(!app.staff_settings.save_success);

    app.staff_settings.loading = false;
    app.staff_settings.save_error = Some("stale error".to_string());
    app.staff_settings.save_success = true;

    let _task = update(&mut app, SettingsMessage::ProvidersRefresh);

    assert!(!app.staff_settings.loading);
    assert_eq!(app.staff_settings.save_error.as_deref(), Some("stale error"));
    assert!(app.staff_settings.save_success);
}

#[test]
fn loaded_success_replaces_config_and_error_records_unreachable_message() {
    let mut app = app();
    let config = staff_config_with_member("engineer");

    let _task = update(&mut app, staff_message(StaffMessage::Loaded(Ok(config.clone()))));

    assert!(!app.staff_settings.loading);
    assert_eq!(app.staff_settings.config, config);
    assert_eq!(app.staff_settings.member_drafts.len(), 1);
    assert_eq!(app.staff_settings.selected_member_id.as_deref(), Some("engineer"));
    assert_eq!(app.staff_settings.save_error, None);
    assert!(!app.staff_settings.save_success);

    let _task =
        update(&mut app, staff_message(StaffMessage::Loaded(Err("gateway offline".to_string()))));

    assert!(!app.staff_settings.loading);
    assert!(
        app.staff_settings
            .save_error
            .as_deref()
            .is_some_and(|error| error.contains("gateway offline"))
    );
    assert!(!app.staff_settings.save_success);
}

#[test]
fn tab_and_member_selection_ignore_unknown_targets_but_clear_feedback() {
    let mut app = app();
    app.staff_settings.member_drafts.push(StaffSettingsMemberDraft::new("member".to_string()));

    let _task =
        update(&mut app, staff_message(StaffMessage::TabChanged("capabilities".to_string())));
    assert_eq!(app.staff_settings.active_tab, StaffSettingsTab::Capabilities);

    let _task = update(&mut app, staff_message(StaffMessage::TabChanged("missing".to_string())));
    assert_eq!(app.staff_settings.active_tab, StaffSettingsTab::Capabilities);

    let _task = update(&mut app, staff_message(StaffMessage::SelectMember("member".to_string())));
    assert_eq!(app.staff_settings.selected_member_id.as_deref(), Some("member"));

    let _task = update(&mut app, staff_message(StaffMessage::SelectMember("missing".to_string())));
    assert_eq!(app.staff_settings.selected_member_id.as_deref(), Some("member"));
    assert_eq!(app.staff_settings.save_error, None);
    assert!(!app.staff_settings.save_success);
}

#[test]
fn reset_draft_restores_config_and_keeps_selection_only_when_member_survives() {
    let mut app = app();
    let config = staff_config_with_member("kept");
    app.staff_settings.replace_config(config);
    app.staff_settings.member_drafts.push(StaffSettingsMemberDraft::new("discarded".to_string()));
    app.staff_settings.selected_member_id = Some("kept".to_string());

    let _task = update(&mut app, staff_message(StaffMessage::ResetDraftRequested));

    assert_eq!(app.staff_settings.member_drafts.len(), 1);
    assert_eq!(app.staff_settings.selected_member_id.as_deref(), Some("kept"));
    assert_eq!(app.staff_settings.save_error, None);
    assert!(!app.staff_settings.save_success);

    app.staff_settings.member_drafts.push(StaffSettingsMemberDraft::new("discarded".to_string()));
    app.staff_settings.selected_member_id = Some("discarded".to_string());

    let _task = update(&mut app, staff_message(StaffMessage::ResetDraftRequested));

    assert_eq!(app.staff_settings.member_drafts.len(), 1);
    assert_eq!(app.staff_settings.selected_member_id.as_deref(), Some("kept"));
}

#[test]
fn add_member_creates_matching_capability_draft_and_selects_member() {
    let mut app = app();

    let _task = update(&mut app, staff_message(StaffMessage::AddMemberRequested));

    assert_eq!(app.staff_settings.member_drafts.len(), 1);
    assert_eq!(app.staff_settings.member_drafts[0].id, "staff-member");
    assert_eq!(app.staff_settings.capability_drafts.len(), 1);
    assert_eq!(app.staff_settings.capability_drafts[0].member_id, "staff-member");
    assert_eq!(app.staff_settings.selected_member_id.as_deref(), Some("staff-member"));
    assert_eq!(app.staff_settings.save_error, None);
    assert!(!app.staff_settings.save_success);
}

#[test]
fn member_field_messages_update_existing_member_and_clear_feedback() {
    let mut app = app();
    app.staff_settings.member_drafts.push(StaffSettingsMemberDraft::new("member".to_string()));

    for message in [
        StaffMessage::MemberNameChanged("member".to_string(), "Ada".to_string()),
        StaffMessage::MemberRoleChanged("member".to_string(), "review".to_string()),
        StaffMessage::MemberLevelChanged("member".to_string(), "principal".to_string()),
        StaffMessage::MemberStatusChanged("member".to_string(), "busy".to_string()),
        StaffMessage::MemberEnabledToggled("member".to_string(), false),
        StaffMessage::MemberCapacityChanged("member".to_string(), "5".to_string()),
        StaffMessage::MemberDelegateAgentChanged("member".to_string(), None),
        StaffMessage::MemberAcpAgentChanged("member".to_string(), Some("acp".to_string())),
    ] {
        let _task = update(&mut app, staff_message(message));
    }

    let draft = &app.staff_settings.member_drafts[0];
    assert_eq!(draft.name, "Ada");
    assert_eq!(draft.role, "review");
    assert_eq!(draft.level, "principal");
    assert_eq!(draft.status, "busy");
    assert!(!draft.enabled);
    assert_eq!(draft.capacity_input, "5");
    assert_eq!(draft.delegate_agent, None);
    assert_eq!(draft.acp_agent.as_deref(), Some("acp"));
    assert_eq!(app.staff_settings.save_error, None);
    assert!(!app.staff_settings.save_success);
}

#[test]
fn member_id_change_updates_member_capability_and_selection() {
    let mut app = app();
    app.staff_settings.member_drafts.push(StaffSettingsMemberDraft::new("old-id".to_string()));
    app.staff_settings.capability_drafts.push(StaffSettingsCapabilitiesDraft::for_member("old-id"));
    app.staff_settings.selected_member_id = Some("old-id".to_string());

    let _task = update(
        &mut app,
        staff_message(StaffMessage::MemberIdChanged("old-id".to_string(), "new-id".to_string())),
    );

    assert_eq!(app.staff_settings.member_drafts[0].id, "new-id");
    assert_eq!(app.staff_settings.capability_drafts[0].member_id, "new-id");
    assert_eq!(app.staff_settings.selected_member_id.as_deref(), Some("new-id"));
    assert_eq!(app.staff_settings.save_error, None);
    assert!(!app.staff_settings.save_success);
}

#[test]
fn save_requested_records_validation_error_without_pending_config() {
    let mut app = app();
    let mut draft = StaffSettingsMemberDraft::new("member".to_string());
    draft.delegate_agent = None;
    draft.acp_agent = None;
    app.staff_settings.member_drafts.push(draft);

    let _task = update(&mut app, staff_message(StaffMessage::SaveRequested));

    assert!(!app.staff_settings.loading);
    assert!(app.staff_settings.pending_config.is_none());
    assert_eq!(
        app.staff_settings.save_error.as_deref(),
        Some("enabled staff member `member` must bind a delegate agent or ACP agent")
    );
    assert!(!app.staff_settings.save_success);
}

#[test]
fn saved_result_applies_pending_config_or_reports_error() {
    let mut app = app();
    let config = staff_config_with_member("engineer");
    app.staff_settings.pending_config = Some(config.clone());
    app.staff_settings.loading = true;

    let _task = update(&mut app, staff_message(StaffMessage::Saved(Ok(()))));

    assert!(!app.staff_settings.loading);
    assert_eq!(app.staff_settings.config, config);
    assert!(app.staff_settings.pending_config.is_none());
    assert_eq!(app.staff_settings.save_error, None);
    assert!(app.staff_settings.save_success);

    app.staff_settings.pending_config = Some(staff_config_with_member("discarded"));
    app.staff_settings.loading = true;

    let _task =
        update(&mut app, staff_message(StaffMessage::Saved(Err("write failed".to_string()))));

    assert!(!app.staff_settings.loading);
    assert!(app.staff_settings.pending_config.is_none());
    assert!(
        app.staff_settings
            .save_error
            .as_deref()
            .is_some_and(|error| error.contains("write failed"))
    );
    assert!(!app.staff_settings.save_success);
}

#[test]
fn builtin_member_delete_is_rejected_without_removing_drafts() {
    let mut app = app();
    let member_id = "staff-planner";
    app.staff_settings.member_drafts.push(StaffSettingsMemberDraft::new(member_id.to_string()));
    app.staff_settings
        .capability_drafts
        .push(StaffSettingsCapabilitiesDraft::for_member(member_id));
    app.staff_settings.selected_member_id = Some(member_id.to_string());

    let _task =
        update(&mut app, staff_message(StaffMessage::DeleteMemberRequested(member_id.to_string())));

    assert_eq!(app.staff_settings.member_drafts.len(), 1);
    assert_eq!(app.staff_settings.capability_drafts.len(), 1);
    assert_eq!(app.staff_settings.selected_member_id.as_deref(), Some(member_id));
    assert_eq!(app.staff_settings.save_error.as_deref(), Some("内置默认员工不能直接删除。"));
    assert!(!app.staff_settings.save_success);
}

#[test]
fn custom_member_delete_removes_related_drafts_and_selects_first_remaining() {
    let mut app = app();
    for id in ["remaining", "custom"] {
        app.staff_settings.member_drafts.push(StaffSettingsMemberDraft::new(id.to_string()));
        app.staff_settings.capability_drafts.push(StaffSettingsCapabilitiesDraft::for_member(id));
    }
    app.staff_settings.selected_member_id = Some("custom".to_string());

    let _task =
        update(&mut app, staff_message(StaffMessage::DeleteMemberRequested("custom".to_string())));

    assert_eq!(app.staff_settings.member_drafts.len(), 1);
    assert_eq!(app.staff_settings.member_drafts[0].id, "remaining");
    assert_eq!(app.staff_settings.capability_drafts.len(), 1);
    assert_eq!(app.staff_settings.capability_drafts[0].member_id, "remaining");
    assert_eq!(app.staff_settings.selected_member_id.as_deref(), Some("remaining"));
    assert_eq!(app.staff_settings.save_error, None);
    assert!(!app.staff_settings.save_success);
}

#[test]
fn channel_binding_messages_add_edit_remove_and_ignore_invalid_index() {
    let mut app = app();
    app.staff_settings.member_drafts.push(StaffSettingsMemberDraft::new("member".to_string()));

    let _task =
        update(&mut app, staff_message(StaffMessage::ChannelBindingAdded("member".to_string())));
    let _task = update(
        &mut app,
        staff_message(StaffMessage::ChannelBindingChannelTypeChanged(
            "member".to_string(),
            0,
            "slack".to_string(),
        )),
    );
    let _task = update(
        &mut app,
        staff_message(StaffMessage::ChannelBindingInstanceIdChanged(
            "member".to_string(),
            0,
            "workspace".to_string(),
        )),
    );
    let _task = update(
        &mut app,
        staff_message(StaffMessage::ChannelBindingRemoved("member".to_string(), 4)),
    );

    let bindings = &app.staff_settings.member_drafts[0].channel_bindings;
    assert_eq!(bindings.len(), 1);
    assert_eq!(bindings[0].channel_type, "slack");
    assert_eq!(bindings[0].instance_id, "workspace");

    let _task = update(
        &mut app,
        staff_message(StaffMessage::ChannelBindingRemoved("member".to_string(), 0)),
    );
    assert!(app.staff_settings.member_drafts[0].channel_bindings.is_empty());
}

#[test]
fn capability_messages_create_missing_draft_and_update_limits_and_allowlists() {
    let mut app = app();

    let messages = [
        StaffMessage::CapabilityAgenticToggled("member".to_string(), true),
        StaffMessage::CapabilityAllowedToolToggled(
            "member".to_string(),
            " shell ".to_string(),
            true,
        ),
        StaffMessage::CapabilityAllowedSkillToggled(
            "member".to_string(),
            " rust ".to_string(),
            true,
        ),
        StaffMessage::CapabilityMaxIterationsChanged("member".to_string(), "12".to_string()),
        StaffMessage::CapabilityMaxParallelTasksChanged("member".to_string(), "3".to_string()),
        StaffMessage::CapabilityMaxActiveMinutesChanged("member".to_string(), "45".to_string()),
    ];
    for message in messages {
        let _task = update(&mut app, staff_message(message));
    }

    assert_eq!(app.staff_settings.capability_drafts.len(), 1);
    let draft = &app.staff_settings.capability_drafts[0];
    assert_eq!(draft.member_id, "member");
    assert!(draft.agentic);
    assert_eq!(draft.allowed_tools, vec!["shell"]);
    assert_eq!(draft.allowed_skills, vec!["rust"]);
    assert_eq!(draft.max_iterations_input, "12");
    assert_eq!(draft.max_parallel_tasks_input, "3");
    assert_eq!(draft.max_active_minutes_input, "45");
}
