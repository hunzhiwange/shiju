//! 处理员工设置中的项目级 Worker 与 Team 资源消息。

use crate::app::message::settings::{
    SettingsMessage, StaffMessage, StaffResourceMessage, StaffTeamDraftField,
    StaffTeamMemberDraftField, StaffWorkerDraftField,
};
use crate::app::state::{StaffTeamMemberDraft, resource_project_path};
use crate::app::{App, Message};
use iced::Task;
use vw_gateway_client::vw_api_types::agent_scheduler::{AgentTeamDto, AgentWorkerDto};
use vw_gateway_client::vw_api_types::project::ResolveProjectRequest;

pub(super) fn update(app: &mut App, message: StaffResourceMessage) -> Task<Message> {
    match message {
        StaffResourceMessage::Refresh => refresh(app),
        StaffResourceMessage::WorkersLoaded(result) => {
            app.staff_settings.workers.loading = false;
            match result {
                Ok(workers) => app.staff_settings.workers.replace_workers(workers),
                Err(error) => {
                    app.staff_settings.workers.error = Some(resource_error(error));
                    app.staff_settings.workers.success = None;
                }
            }
            Task::none()
        }
        StaffResourceMessage::WorkerSelected(worker_id) => {
            app.staff_settings.workers.select(&worker_id);
            Task::none()
        }
        StaffResourceMessage::WorkerNewRequested => {
            app.staff_settings.workers.start_new();
            Task::none()
        }
        StaffResourceMessage::WorkerFieldChanged(field, value) => {
            if let Some(draft) = app.staff_settings.workers.draft.as_mut() {
                match field {
                    StaffWorkerDraftField::WorkerId => draft.worker_id = value,
                    StaffWorkerDraftField::Runtime => draft.runtime = value,
                    StaffWorkerDraftField::Deployment => draft.deployment = value,
                    StaffWorkerDraftField::Model => draft.model = value,
                    StaffWorkerDraftField::ModelProvider => draft.model_provider = value,
                    StaffWorkerDraftField::TeamId => draft.team_id = value,
                    StaffWorkerDraftField::WorkerRole => draft.worker_role = value,
                    StaffWorkerDraftField::MatrixUserId => draft.matrix_user_id = value,
                    StaffWorkerDraftField::RoomId => draft.room_id = value,
                }
                clear_worker_feedback(app);
            }
            Task::none()
        }
        StaffResourceMessage::WorkerCapabilitiesChanged(capabilities) => {
            if let Some(draft) = app.staff_settings.workers.draft.as_mut() {
                draft.capabilities = capabilities;
                clear_worker_feedback(app);
            }
            Task::none()
        }
        StaffResourceMessage::WorkerSaveRequested => save_worker(app),
        StaffResourceMessage::WorkerSaved(result) => {
            app.staff_settings.workers.saving = false;
            match result {
                Ok(worker) => {
                    let message = format!("Worker `{}` saved", worker.id);
                    app.staff_settings.workers.apply_saved(worker);
                    app.show_success_toast(message)
                }
                Err(error) => {
                    let error = resource_error(error);
                    app.staff_settings.workers.error = Some(error.clone());
                    app.staff_settings.workers.success = None;
                    app.show_error_toast(error)
                }
            }
        }
        StaffResourceMessage::TeamsLoaded(result) => {
            app.staff_settings.teams.loading = false;
            match result {
                Ok(teams) => app.staff_settings.teams.replace_teams(teams),
                Err(error) => {
                    app.staff_settings.teams.error = Some(resource_error(error));
                    app.staff_settings.teams.success = None;
                }
            }
            Task::none()
        }
        StaffResourceMessage::TeamSelected(team_id) => {
            app.staff_settings.teams.select(&team_id);
            Task::none()
        }
        StaffResourceMessage::TeamNewRequested => {
            app.staff_settings.teams.start_new();
            Task::none()
        }
        StaffResourceMessage::TeamFieldChanged(field, value) => {
            if let Some(draft) = app.staff_settings.teams.draft.as_mut() {
                match field {
                    StaffTeamDraftField::TeamId => draft.team_id = value,
                    StaffTeamDraftField::Name => draft.name = value,
                    StaffTeamDraftField::LeaderId => draft.leader_id = value,
                    StaffTeamDraftField::TeamRoomId => draft.team_room_id = value,
                    StaffTeamDraftField::LeaderDmRoomId => draft.leader_dm_room_id = value,
                }
                clear_team_feedback(app);
            }
            Task::none()
        }
        StaffResourceMessage::TeamStatusChanged(status) => {
            if let Some(draft) = app.staff_settings.teams.draft.as_mut() {
                draft.status = status;
                clear_team_feedback(app);
            }
            Task::none()
        }
        StaffResourceMessage::TeamMemberAdded(worker_id) => {
            if let Some(draft) = app.staff_settings.teams.draft.as_mut() {
                draft.members.push(StaffTeamMemberDraft::new(worker_id));
                clear_team_feedback(app);
            }
            Task::none()
        }
        StaffResourceMessage::TeamMemberRemoved(index) => {
            if let Some(draft) = app.staff_settings.teams.draft.as_mut()
                && index < draft.members.len()
            {
                draft.members.remove(index);
                clear_team_feedback(app);
            }
            Task::none()
        }
        StaffResourceMessage::TeamMemberFieldChanged(index, field, value) => {
            if let Some(member) = app
                .staff_settings
                .teams
                .draft
                .as_mut()
                .and_then(|draft| draft.members.get_mut(index))
            {
                match field {
                    StaffTeamMemberDraftField::WorkerId => member.worker_id = value,
                    StaffTeamMemberDraftField::RuntimeName => member.runtime_name = value,
                    StaffTeamMemberDraftField::Role => member.role = value,
                    StaffTeamMemberDraftField::RoomId => member.room_id = value,
                    StaffTeamMemberDraftField::MatrixUserId => member.matrix_user_id = value,
                }
                clear_team_feedback(app);
            }
            Task::none()
        }
        StaffResourceMessage::TeamMemberReadyToggled(index, ready) => {
            if let Some(member) = app
                .staff_settings
                .teams
                .draft
                .as_mut()
                .and_then(|draft| draft.members.get_mut(index))
            {
                member.ready = ready;
                clear_team_feedback(app);
            }
            Task::none()
        }
        StaffResourceMessage::TeamSaveRequested => save_team(app),
        StaffResourceMessage::TeamSaved(result) => {
            app.staff_settings.teams.saving = false;
            match result {
                Ok(team) => {
                    let message = format!("Team `{}` saved", team.id);
                    app.staff_settings.teams.apply_saved(team);
                    app.show_success_toast(message)
                }
                Err(error) => {
                    let error = resource_error(error);
                    app.staff_settings.teams.error = Some(error.clone());
                    app.staff_settings.teams.success = None;
                    app.show_error_toast(error)
                }
            }
        }
    }
}

fn refresh(app: &mut App) -> Task<Message> {
    let project_path = match resource_project_path(app.project_path.as_deref()) {
        Ok(project_path) => project_path,
        Err(error) => {
            app.staff_settings.workers.loading = false;
            app.staff_settings.workers.error = Some(error.clone());
            app.staff_settings.workers.success = None;
            app.staff_settings.teams.loading = false;
            app.staff_settings.teams.error = Some(error);
            app.staff_settings.teams.success = None;
            return Task::none();
        }
    };

    app.staff_settings.workers.loading = true;
    app.staff_settings.workers.error = None;
    app.staff_settings.workers.success = None;
    app.staff_settings.teams.loading = true;
    app.staff_settings.teams.error = None;
    app.staff_settings.teams.success = None;
    Task::batch(vec![load_workers_task(project_path.clone()), load_teams_task(project_path)])
}

fn save_worker(app: &mut App) -> Task<Message> {
    if app.staff_settings.workers.saving {
        return Task::none();
    }
    let project_path = match resource_project_path(app.project_path.as_deref()) {
        Ok(project_path) => project_path,
        Err(error) => {
            app.staff_settings.workers.error = Some(error);
            app.staff_settings.workers.success = None;
            return Task::none();
        }
    };
    let request = match app.staff_settings.workers.request() {
        Ok(request) => request,
        Err(error) => {
            app.staff_settings.workers.error = Some(error);
            app.staff_settings.workers.success = None;
            return Task::none();
        }
    };

    app.staff_settings.workers.saving = true;
    app.staff_settings.workers.error = None;
    app.staff_settings.workers.success = None;
    Task::perform(
        async move {
            let client = crate::app::config::gateway_client()?;
            let scope = resolve_project_scope(&client, project_path).await?;
            client.agent_scheduler_upsert_worker(&scope, &request).await.map(|result| result.worker)
        },
        |result| {
            Message::Settings(SettingsMessage::Staff(StaffMessage::Resources(
                StaffResourceMessage::WorkerSaved(result),
            )))
        },
    )
}

fn save_team(app: &mut App) -> Task<Message> {
    if app.staff_settings.teams.saving {
        return Task::none();
    }
    let project_path = match resource_project_path(app.project_path.as_deref()) {
        Ok(project_path) => project_path,
        Err(error) => {
            app.staff_settings.teams.error = Some(error);
            app.staff_settings.teams.success = None;
            return Task::none();
        }
    };
    let request = match app.staff_settings.teams.request(&app.staff_settings.workers.workers) {
        Ok(request) => request,
        Err(error) => {
            app.staff_settings.teams.error = Some(error);
            app.staff_settings.teams.success = None;
            return Task::none();
        }
    };

    app.staff_settings.teams.saving = true;
    app.staff_settings.teams.error = None;
    app.staff_settings.teams.success = None;
    Task::perform(
        async move {
            let client = crate::app::config::gateway_client()?;
            let scope = resolve_project_scope(&client, project_path).await?;
            client.agent_scheduler_upsert_team(&scope, &request).await.map(|result| result.team)
        },
        |result| {
            Message::Settings(SettingsMessage::Staff(StaffMessage::Resources(
                StaffResourceMessage::TeamSaved(result),
            )))
        },
    )
}

fn load_workers_task(project_path: String) -> Task<Message> {
    Task::perform(load_workers(project_path), |result| {
        Message::Settings(SettingsMessage::Staff(StaffMessage::Resources(
            StaffResourceMessage::WorkersLoaded(result),
        )))
    })
}

fn load_teams_task(project_path: String) -> Task<Message> {
    Task::perform(load_teams(project_path), |result| {
        Message::Settings(SettingsMessage::Staff(StaffMessage::Resources(
            StaffResourceMessage::TeamsLoaded(result),
        )))
    })
}

async fn load_workers(project_path: String) -> Result<Vec<AgentWorkerDto>, String> {
    let client = crate::app::config::gateway_client()?;
    let scope = resolve_project_scope(&client, project_path).await?;
    client.agent_scheduler_workers(&scope).await.map(|result| result.workers)
}

async fn load_teams(project_path: String) -> Result<Vec<AgentTeamDto>, String> {
    let client = crate::app::config::gateway_client()?;
    let scope = resolve_project_scope(&client, project_path).await?;
    client.agent_scheduler_teams(&scope).await.map(|result| result.teams)
}

async fn resolve_project_scope(
    client: &vw_gateway_client::GatewayClient,
    project_path: String,
) -> Result<vw_gateway_client::vw_api_types::agent_scheduler::AgentSchedulerScopeDto, String> {
    let project = client
        .project_resolve(
            &crate::app::session_gateway::active_tenant_scope(),
            &ResolveProjectRequest { directory: project_path, create_if_missing: false },
        )
        .await?
        .project;
    crate::app::session_gateway::active_agent_scheduler_scope(project.id)
}

fn clear_worker_feedback(app: &mut App) {
    app.staff_settings.workers.error = None;
    app.staff_settings.workers.success = None;
}

fn clear_team_feedback(app: &mut App) {
    app.staff_settings.teams.error = None;
    app.staff_settings.teams.success = None;
}

fn resource_error(error: String) -> String {
    crate::app::config::server_config_unreachable_error(error)
}

#[cfg(test)]
#[path = "resources_tests.rs"]
mod resources_tests;
