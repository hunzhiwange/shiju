//! 处理系统设置页中的员工配置状态消息。

mod resources;

#[cfg(test)]
#[path = "staff_tests.rs"]
mod staff_tests;

use crate::app::message::settings::{SettingsMessage, StaffMessage};
use crate::app::state::{
    StaffSettingsCapabilitiesDraft, StaffSettingsChannelBindingDraft, StaffSettingsMemberDraft,
    StaffSettingsTab, is_builtin_staff_member_id,
};
use crate::app::{App, Message};
use iced::Task;

fn refresh_task() -> Task<Message> {
    Task::perform(async { crate::app::config::load_staff_config_result() }, |result| {
        Message::Settings(SettingsMessage::Staff(StaffMessage::Loaded(result)))
    })
}

fn find_member_draft_mut<'a>(
    app: &'a mut App,
    member_id: &str,
) -> Option<&'a mut StaffSettingsMemberDraft> {
    app.staff_settings.member_drafts.iter_mut().find(|draft| draft.id == member_id)
}

fn find_capability_draft_mut<'a>(
    app: &'a mut App,
    member_id: &str,
) -> Option<&'a mut StaffSettingsCapabilitiesDraft> {
    app.staff_settings.capability_drafts.iter_mut().find(|draft| draft.member_id == member_id)
}

fn ensure_capability_draft<'a>(
    app: &'a mut App,
    member_id: &str,
) -> &'a mut StaffSettingsCapabilitiesDraft {
    if !app.staff_settings.capability_drafts.iter().any(|draft| draft.member_id == member_id) {
        app.staff_settings
            .capability_drafts
            .push(StaffSettingsCapabilitiesDraft::for_member(member_id));
    }
    find_capability_draft_mut(app, member_id).expect("capability draft exists")
}

fn next_member_id(app: &App) -> String {
    let base = "staff-member";
    if !app.staff_settings.member_drafts.iter().any(|draft| draft.id == base) {
        return base.to_string();
    }

    let mut index = 2;
    loop {
        let candidate = format!("{base}-{index}");
        if !app.staff_settings.member_drafts.iter().any(|draft| draft.id == candidate) {
            return candidate;
        }
        index += 1;
    }
}

fn update_string_list(values: &mut Vec<String>, value: String, enabled: bool) {
    let value = value.trim().to_string();
    if value.is_empty() {
        return;
    }

    if enabled {
        if !values.iter().any(|candidate| candidate == &value) {
            values.push(value);
            values.sort();
        }
    } else {
        values.retain(|candidate| candidate != &value);
    }
}

fn persist_staff_settings(app: &mut App) -> Task<Message> {
    let config = match app.staff_settings.to_config() {
        Ok(config) => config,
        Err(error) => {
            app.staff_settings.save_error = Some(error);
            app.staff_settings.save_success = false;
            return Task::none();
        }
    };

    app.staff_settings.loading = true;
    app.staff_settings.save_error = None;
    app.staff_settings.save_success = false;
    app.staff_settings.pending_config = Some(config.clone());
    crate::app::config::update_staff_config_async(move |staff| {
        *staff = config;
    })
}

pub(super) fn apply_saved_result(app: &mut App, result: Result<(), String>) {
    app.staff_settings.loading = false;
    match result {
        Ok(()) => {
            if let Some(config) = app.staff_settings.pending_config.take() {
                app.staff_settings.config = config;
            }
            app.staff_settings.save_error = None;
            app.staff_settings.save_success = true;
            sync_task_board_staff_projection(app);
        }
        Err(error) => {
            app.staff_settings.pending_config = None;
            app.staff_settings.save_error =
                Some(crate::app::config::server_config_unreachable_error(error));
            app.staff_settings.save_success = false;
        }
    }
}

pub(super) fn sync_task_board_staff_projection(app: &mut App) {
    match crate::app::task::staff::task_board_staff_config_from_staff_config_for_workflow_id(
        app.staff_settings.config.clone(),
        &app.task_board_settings.workflow_id,
    ) {
        Ok(config) => {
            app.task_board_staff_config = config;
            app.task_board_staff_config_workflow_id =
                Some(app.task_board_settings.workflow_id.clone());
            match crate::app::task::staff::build_staff_mode_projection(
                &app.task_board_tasks,
                &app.task_board_staff_config,
                &app.task_board_executor.running_tasks,
                &app.task_board_settings.workflow_id,
            ) {
                Ok(projection) => {
                    app.task_board_staff_projection = projection;
                    app.task_board_staff_config_load_error = None;
                }
                Err(error) => {
                    app.task_board_staff_projection = Default::default();
                    app.task_board_staff_config_load_error = Some(error);
                }
            }
        }
        Err(error) => {
            app.task_board_staff_config_load_error = Some(error);
        }
    }
}

pub fn update(app: &mut App, message: SettingsMessage) -> Task<Message> {
    let SettingsMessage::Staff(message) = message else {
        return Task::none();
    };

    match message {
        StaffMessage::Resources(message) => resources::update(app, message),
        StaffMessage::Refresh => {
            app.staff_settings.loading = true;
            app.staff_settings.save_error = None;
            app.staff_settings.save_success = false;
            refresh_task()
        }
        StaffMessage::Loaded(result) => {
            app.staff_settings.loading = false;
            match result {
                Ok(config) => {
                    app.staff_settings.replace_config(config);
                    sync_task_board_staff_projection(app);
                }
                Err(error) => {
                    app.staff_settings.save_error =
                        Some(crate::app::config::server_config_unreachable_error(error));
                    app.staff_settings.save_success = false;
                }
            }
            Task::none()
        }
        StaffMessage::TabChanged(tab_key) => {
            if let Some(tab) = StaffSettingsTab::from_key(&tab_key) {
                app.staff_settings.active_tab = tab;
            }
            app.staff_settings.save_error = None;
            app.staff_settings.save_success = false;
            Task::none()
        }
        StaffMessage::SelectMember(member_id) => {
            if app.staff_settings.member_drafts.iter().any(|draft| draft.id == member_id) {
                app.staff_settings.selected_member_id = Some(member_id);
            }
            app.staff_settings.save_error = None;
            app.staff_settings.save_success = false;
            Task::none()
        }
        StaffMessage::AddMemberRequested => {
            let member_id = next_member_id(app);
            app.staff_settings.member_drafts.push(StaffSettingsMemberDraft::new(member_id.clone()));
            app.staff_settings
                .capability_drafts
                .push(StaffSettingsCapabilitiesDraft::for_member(&member_id));
            app.staff_settings.selected_member_id = Some(member_id);
            app.staff_settings.save_error = None;
            app.staff_settings.save_success = false;
            Task::none()
        }
        StaffMessage::DeleteMemberRequested(member_id) => {
            if is_builtin_staff_member_id(&member_id) {
                app.staff_settings.save_error = Some("内置默认员工不能直接删除。".to_string());
                app.staff_settings.save_success = false;
                return Task::none();
            }
            app.staff_settings.member_drafts.retain(|draft| draft.id != member_id);
            app.staff_settings.capability_drafts.retain(|draft| draft.member_id != member_id);
            if app.staff_settings.selected_member_id.as_deref() == Some(member_id.as_str()) {
                app.staff_settings.selected_member_id =
                    app.staff_settings.member_drafts.first().map(|draft| draft.id.clone());
            }
            app.staff_settings.save_error = None;
            app.staff_settings.save_success = false;
            Task::none()
        }
        StaffMessage::ResetDraftRequested => {
            let selected_member_id = app.staff_settings.selected_member_id.clone();
            let config = app.staff_settings.config.clone();
            app.staff_settings.replace_config(config);
            if let Some(member_id) = selected_member_id
                && app.staff_settings.member_drafts.iter().any(|draft| draft.id == member_id)
            {
                app.staff_settings.selected_member_id = Some(member_id);
            }
            app.staff_settings.save_error = None;
            app.staff_settings.save_success = false;
            Task::none()
        }
        StaffMessage::MemberIdChanged(member_id, value) => {
            if let Some(draft) = find_member_draft_mut(app, &member_id) {
                draft.id = value.clone();
            }
            if let Some(draft) = find_capability_draft_mut(app, &member_id) {
                draft.member_id = value.clone();
            }
            if app.staff_settings.selected_member_id.as_deref() == Some(member_id.as_str()) {
                app.staff_settings.selected_member_id = Some(value);
            }
            app.staff_settings.save_error = None;
            app.staff_settings.save_success = false;
            Task::none()
        }
        StaffMessage::MemberNameChanged(member_id, value) => {
            if let Some(draft) = find_member_draft_mut(app, &member_id) {
                draft.name = value;
            }
            app.staff_settings.save_error = None;
            app.staff_settings.save_success = false;
            Task::none()
        }
        StaffMessage::MemberRoleChanged(member_id, value) => {
            if let Some(draft) = find_member_draft_mut(app, &member_id) {
                draft.role = value;
            }
            app.staff_settings.save_error = None;
            app.staff_settings.save_success = false;
            Task::none()
        }
        StaffMessage::MemberLevelChanged(member_id, value) => {
            if let Some(draft) = find_member_draft_mut(app, &member_id) {
                draft.level = value;
            }
            app.staff_settings.save_error = None;
            app.staff_settings.save_success = false;
            Task::none()
        }
        StaffMessage::MemberStatusChanged(member_id, value) => {
            if let Some(draft) = find_member_draft_mut(app, &member_id) {
                draft.status = value;
            }
            app.staff_settings.save_error = None;
            app.staff_settings.save_success = false;
            Task::none()
        }
        StaffMessage::MemberEnabledToggled(member_id, enabled) => {
            if let Some(draft) = find_member_draft_mut(app, &member_id) {
                draft.enabled = enabled;
            }
            app.staff_settings.save_error = None;
            app.staff_settings.save_success = false;
            Task::none()
        }
        StaffMessage::MemberCapacityChanged(member_id, value) => {
            if let Some(draft) = find_member_draft_mut(app, &member_id) {
                draft.capacity_input = value;
            }
            app.staff_settings.save_error = None;
            app.staff_settings.save_success = false;
            Task::none()
        }
        StaffMessage::MemberDelegateAgentChanged(member_id, value) => {
            if let Some(draft) = find_member_draft_mut(app, &member_id) {
                draft.delegate_agent = value;
            }
            app.staff_settings.save_error = None;
            app.staff_settings.save_success = false;
            Task::none()
        }
        StaffMessage::MemberAcpAgentChanged(member_id, value) => {
            if let Some(draft) = find_member_draft_mut(app, &member_id) {
                draft.acp_agent = value;
            }
            app.staff_settings.save_error = None;
            app.staff_settings.save_success = false;
            Task::none()
        }
        StaffMessage::ChannelBindingAdded(member_id) => {
            if let Some(draft) = find_member_draft_mut(app, &member_id) {
                draft.channel_bindings.push(StaffSettingsChannelBindingDraft::new());
            }
            app.staff_settings.save_error = None;
            app.staff_settings.save_success = false;
            Task::none()
        }
        StaffMessage::ChannelBindingRemoved(member_id, index) => {
            if let Some(draft) = find_member_draft_mut(app, &member_id)
                && index < draft.channel_bindings.len()
            {
                draft.channel_bindings.remove(index);
            }
            app.staff_settings.save_error = None;
            app.staff_settings.save_success = false;
            Task::none()
        }
        StaffMessage::ChannelBindingChannelTypeChanged(member_id, index, value) => {
            if let Some(draft) = find_member_draft_mut(app, &member_id)
                && let Some(binding) = draft.channel_bindings.get_mut(index)
            {
                binding.channel_type = value;
            }
            app.staff_settings.save_error = None;
            app.staff_settings.save_success = false;
            Task::none()
        }
        StaffMessage::ChannelBindingInstanceIdChanged(member_id, index, value) => {
            if let Some(draft) = find_member_draft_mut(app, &member_id)
                && let Some(binding) = draft.channel_bindings.get_mut(index)
            {
                binding.instance_id = value;
            }
            app.staff_settings.save_error = None;
            app.staff_settings.save_success = false;
            Task::none()
        }
        StaffMessage::CapabilityAgenticToggled(member_id, enabled) => {
            ensure_capability_draft(app, &member_id).agentic = enabled;
            app.staff_settings.save_error = None;
            app.staff_settings.save_success = false;
            Task::none()
        }
        StaffMessage::CapabilityAllowedToolToggled(member_id, tool, enabled) => {
            update_string_list(
                &mut ensure_capability_draft(app, &member_id).allowed_tools,
                tool,
                enabled,
            );
            app.staff_settings.save_error = None;
            app.staff_settings.save_success = false;
            Task::none()
        }
        StaffMessage::CapabilityAllowedSkillToggled(member_id, skill, enabled) => {
            update_string_list(
                &mut ensure_capability_draft(app, &member_id).allowed_skills,
                skill,
                enabled,
            );
            app.staff_settings.save_error = None;
            app.staff_settings.save_success = false;
            Task::none()
        }
        StaffMessage::CapabilityMaxIterationsChanged(member_id, value) => {
            ensure_capability_draft(app, &member_id).max_iterations_input = value;
            app.staff_settings.save_error = None;
            app.staff_settings.save_success = false;
            Task::none()
        }
        StaffMessage::CapabilityMaxParallelTasksChanged(member_id, value) => {
            ensure_capability_draft(app, &member_id).max_parallel_tasks_input = value;
            app.staff_settings.save_error = None;
            app.staff_settings.save_success = false;
            Task::none()
        }
        StaffMessage::CapabilityMaxActiveMinutesChanged(member_id, value) => {
            ensure_capability_draft(app, &member_id).max_active_minutes_input = value;
            app.staff_settings.save_error = None;
            app.staff_settings.save_success = false;
            Task::none()
        }
        StaffMessage::SaveRequested => persist_staff_settings(app),
        StaffMessage::Saved(result) => {
            apply_saved_result(app, result);
            Task::none()
        }
    }
}
