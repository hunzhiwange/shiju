//! 维护员工设置中的 Worker 与 Team 项目资源状态。

use std::collections::HashSet;

use vw_gateway_client::vw_api_types::agent_scheduler::{
    AgentCapabilityDto, AgentSchedulerUpsertTeamRequest, AgentSchedulerUpsertWorkerRequest,
    AgentTeamDto, AgentTeamMemberDto, AgentTeamStatusDto, AgentWorkerDto, AgentWorkerStatusDto,
};
use vw_gateway_client::vw_api_types::common::TimestampMs;

#[derive(Debug, Clone, Default)]
pub(crate) struct StaffWorkerResourcesState {
    pub(crate) loading: bool,
    pub(crate) workers: Vec<AgentWorkerDto>,
    pub(crate) selected_worker_id: Option<String>,
    pub(crate) draft: Option<StaffWorkerDraft>,
    pub(crate) saving: bool,
    pub(crate) error: Option<String>,
    pub(crate) success: Option<String>,
}

impl StaffWorkerResourcesState {
    pub(crate) fn replace_workers(&mut self, workers: Vec<AgentWorkerDto>) {
        let selected_worker_id = self
            .selected_worker_id
            .as_deref()
            .filter(|selected| workers.iter().any(|worker| worker.id == *selected))
            .map(str::to_string)
            .or_else(|| workers.first().map(|worker| worker.id.clone()));
        let draft = selected_worker_id
            .as_deref()
            .and_then(|selected| workers.iter().find(|worker| worker.id == selected))
            .map(StaffWorkerDraft::from_worker);

        self.loading = false;
        self.workers = workers;
        self.selected_worker_id = selected_worker_id;
        self.draft = draft;
        self.saving = false;
        self.error = None;
        self.success = None;
    }

    pub(crate) fn select(&mut self, worker_id: &str) {
        let Some(worker) = self.workers.iter().find(|worker| worker.id == worker_id) else {
            return;
        };
        self.selected_worker_id = Some(worker.id.clone());
        self.draft = Some(StaffWorkerDraft::from_worker(worker));
        self.error = None;
        self.success = None;
    }

    pub(crate) fn start_new(&mut self) {
        self.selected_worker_id = None;
        self.draft = Some(StaffWorkerDraft::new(next_resource_id(
            "worker",
            self.workers.iter().map(|worker| worker.id.as_str()),
        )));
        self.error = None;
        self.success = None;
    }

    pub(crate) fn request(&self) -> Result<AgentSchedulerUpsertWorkerRequest, String> {
        self.draft.as_ref().ok_or_else(|| "worker draft is not selected".to_string())?.to_request()
    }

    pub(crate) fn apply_saved(&mut self, worker: AgentWorkerDto) {
        if let Some(existing) = self.workers.iter_mut().find(|existing| existing.id == worker.id) {
            *existing = worker.clone();
        } else {
            self.workers.push(worker.clone());
            self.workers.sort_by(|left, right| left.id.cmp(&right.id));
        }
        self.selected_worker_id = Some(worker.id.clone());
        self.draft = Some(StaffWorkerDraft::from_worker(&worker));
        self.loading = false;
        self.saving = false;
        self.error = None;
        self.success = Some(format!("Worker `{}` saved", worker.id));
    }
}

#[derive(Debug, Clone)]
pub(crate) struct StaffWorkerDraft {
    pub(crate) worker_id: String,
    pub(crate) runtime: String,
    pub(crate) deployment: String,
    pub(crate) model: String,
    pub(crate) model_provider: String,
    pub(crate) team_id: String,
    pub(crate) worker_role: String,
    pub(crate) matrix_user_id: String,
    pub(crate) room_id: String,
    pub(crate) capabilities: Vec<AgentCapabilityDto>,
    pub(crate) status: Option<AgentWorkerStatusDto>,
    pub(crate) desired_state: Option<String>,
    pub(crate) container_state: Option<String>,
    pub(crate) idle_since_ms: Option<TimestampMs>,
    pub(crate) last_seen_at_ms: Option<TimestampMs>,
    pub(crate) last_heartbeat_at: Option<String>,
    pub(crate) updated_at_ms: Option<TimestampMs>,
}

impl StaffWorkerDraft {
    fn new(worker_id: String) -> Self {
        Self {
            worker_id,
            runtime: String::new(),
            deployment: String::new(),
            model: "auto".to_string(),
            model_provider: String::new(),
            team_id: String::new(),
            worker_role: "worker".to_string(),
            matrix_user_id: String::new(),
            room_id: String::new(),
            capabilities: Vec::new(),
            status: None,
            desired_state: None,
            container_state: None,
            idle_since_ms: None,
            last_seen_at_ms: None,
            last_heartbeat_at: None,
            updated_at_ms: None,
        }
    }

    fn from_worker(worker: &AgentWorkerDto) -> Self {
        Self {
            worker_id: worker.id.clone(),
            runtime: worker.runtime.clone().unwrap_or_default(),
            deployment: worker.deployment.clone().unwrap_or_default(),
            model: worker.model.clone(),
            model_provider: worker.model_provider.clone().unwrap_or_default(),
            team_id: worker.team_id.clone().unwrap_or_default(),
            worker_role: worker.worker_role.clone().unwrap_or_default(),
            matrix_user_id: worker.matrix_user_id.clone().unwrap_or_default(),
            room_id: worker.room_id.clone().unwrap_or_default(),
            capabilities: worker.capabilities.clone(),
            status: Some(worker.status.clone()),
            desired_state: worker.desired_state.clone(),
            container_state: worker.container_state.clone(),
            idle_since_ms: worker.idle_since_ms,
            last_seen_at_ms: worker.last_seen_at_ms,
            last_heartbeat_at: worker.last_heartbeat_at.clone(),
            updated_at_ms: Some(worker.updated_at_ms),
        }
    }

    fn to_request(&self) -> Result<AgentSchedulerUpsertWorkerRequest, String> {
        let worker_id = required_trimmed(&self.worker_id, "worker id")?;
        let model = required_trimmed(&self.model, "worker model")?;
        Ok(AgentSchedulerUpsertWorkerRequest {
            worker_id,
            runtime: optional_trimmed(&self.runtime),
            deployment: optional_trimmed(&self.deployment),
            model: Some(model),
            model_provider: optional_trimmed(&self.model_provider),
            status: self.status.clone(),
            desired_state: self.desired_state.clone(),
            team_id: optional_trimmed(&self.team_id),
            worker_role: optional_trimmed(&self.worker_role),
            matrix_user_id: optional_trimmed(&self.matrix_user_id),
            room_id: optional_trimmed(&self.room_id),
            container_state: self.container_state.clone(),
            idle_since_ms: self.idle_since_ms,
            last_seen_at_ms: self.last_seen_at_ms,
            last_heartbeat_at: self.last_heartbeat_at.clone(),
            capabilities: self.capabilities.clone(),
            updated_at_ms: self.updated_at_ms,
        })
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct StaffTeamResourcesState {
    pub(crate) loading: bool,
    pub(crate) teams: Vec<AgentTeamDto>,
    pub(crate) selected_team_id: Option<String>,
    pub(crate) draft: Option<StaffTeamDraft>,
    pub(crate) saving: bool,
    pub(crate) error: Option<String>,
    pub(crate) success: Option<String>,
}

impl StaffTeamResourcesState {
    pub(crate) fn replace_teams(&mut self, teams: Vec<AgentTeamDto>) {
        let selected_team_id = self
            .selected_team_id
            .as_deref()
            .filter(|selected| teams.iter().any(|team| team.id == *selected))
            .map(str::to_string)
            .or_else(|| teams.first().map(|team| team.id.clone()));
        let draft = selected_team_id
            .as_deref()
            .and_then(|selected| teams.iter().find(|team| team.id == selected))
            .map(StaffTeamDraft::from_team);

        self.loading = false;
        self.teams = teams;
        self.selected_team_id = selected_team_id;
        self.draft = draft;
        self.saving = false;
        self.error = None;
        self.success = None;
    }

    pub(crate) fn select(&mut self, team_id: &str) {
        let Some(team) = self.teams.iter().find(|team| team.id == team_id) else {
            return;
        };
        self.selected_team_id = Some(team.id.clone());
        self.draft = Some(StaffTeamDraft::from_team(team));
        self.error = None;
        self.success = None;
    }

    pub(crate) fn start_new(&mut self) {
        self.selected_team_id = None;
        self.draft = Some(StaffTeamDraft::new(next_resource_id(
            "team",
            self.teams.iter().map(|team| team.id.as_str()),
        )));
        self.error = None;
        self.success = None;
    }

    pub(crate) fn request(
        &self,
        workers: &[AgentWorkerDto],
    ) -> Result<AgentSchedulerUpsertTeamRequest, String> {
        self.draft
            .as_ref()
            .ok_or_else(|| "team draft is not selected".to_string())?
            .to_request(workers)
    }

    pub(crate) fn apply_saved(&mut self, team: AgentTeamDto) {
        if let Some(existing) = self.teams.iter_mut().find(|existing| existing.id == team.id) {
            *existing = team.clone();
        } else {
            self.teams.push(team.clone());
            self.teams.sort_by(|left, right| left.id.cmp(&right.id));
        }
        self.selected_team_id = Some(team.id.clone());
        self.draft = Some(StaffTeamDraft::from_team(&team));
        self.loading = false;
        self.saving = false;
        self.error = None;
        self.success = Some(format!("Team `{}` saved", team.id));
    }
}

#[derive(Debug, Clone)]
pub(crate) struct StaffTeamDraft {
    pub(crate) team_id: String,
    pub(crate) name: String,
    pub(crate) status: AgentTeamStatusDto,
    pub(crate) leader_id: String,
    pub(crate) team_room_id: String,
    pub(crate) leader_dm_room_id: String,
    pub(crate) members: Vec<StaffTeamMemberDraft>,
    pub(crate) updated_at_ms: Option<TimestampMs>,
}

impl StaffTeamDraft {
    fn new(team_id: String) -> Self {
        Self {
            name: team_id.clone(),
            team_id,
            status: AgentTeamStatusDto::Pending,
            leader_id: String::new(),
            team_room_id: String::new(),
            leader_dm_room_id: String::new(),
            members: Vec::new(),
            updated_at_ms: None,
        }
    }

    fn from_team(team: &AgentTeamDto) -> Self {
        Self {
            team_id: team.id.clone(),
            name: team.name.clone(),
            status: team.status.clone(),
            leader_id: team.leader_id.clone(),
            team_room_id: team.team_room_id.clone().unwrap_or_default(),
            leader_dm_room_id: team.leader_dm_room_id.clone().unwrap_or_default(),
            members: team.members.iter().map(StaffTeamMemberDraft::from_member).collect(),
            updated_at_ms: Some(team.updated_at_ms),
        }
    }

    fn to_request(
        &self,
        workers: &[AgentWorkerDto],
    ) -> Result<AgentSchedulerUpsertTeamRequest, String> {
        let team_id = required_trimmed(&self.team_id, "team id")?;
        let name = required_trimmed(&self.name, "team name")?;
        let leader_id = required_trimmed(&self.leader_id, "team leader")?;
        let loaded_worker_ids =
            workers.iter().map(|worker| worker.id.as_str()).collect::<HashSet<_>>();
        let mut member_ids = HashSet::with_capacity(self.members.len());
        let mut members = Vec::with_capacity(self.members.len());

        for member in &self.members {
            let member = member.to_member()?;
            if !member_ids.insert(member.worker_id.clone()) {
                return Err("team member worker ids must be unique".to_string());
            }
            if !loaded_worker_ids.contains(member.worker_id.as_str()) {
                return Err(format!(
                    "team member `{}` must reference a loaded worker",
                    member.worker_id
                ));
            }
            members.push(member);
        }

        let Some(leader) = members.iter().find(|member| member.worker_id == leader_id) else {
            return Err("team leader must belong to team members".to_string());
        };
        let leader_ready = leader.ready;
        let ready_workers = members.iter().filter(|member| member.ready).count() as u32;
        let total_workers = members.len() as u32;

        Ok(AgentSchedulerUpsertTeamRequest {
            team_id,
            name,
            status: Some(self.status.clone()),
            leader_id,
            team_room_id: optional_trimmed(&self.team_room_id),
            leader_dm_room_id: optional_trimmed(&self.leader_dm_room_id),
            leader_ready,
            ready_workers,
            total_workers,
            members,
            updated_at_ms: self.updated_at_ms,
        })
    }
}

#[derive(Debug, Clone)]
pub(crate) struct StaffTeamMemberDraft {
    pub(crate) worker_id: String,
    pub(crate) runtime_name: String,
    pub(crate) role: String,
    pub(crate) room_id: String,
    pub(crate) matrix_user_id: String,
    pub(crate) ready: bool,
}

impl StaffTeamMemberDraft {
    pub(crate) fn new(worker_id: String) -> Self {
        Self {
            worker_id,
            runtime_name: String::new(),
            role: "member".to_string(),
            room_id: String::new(),
            matrix_user_id: String::new(),
            ready: false,
        }
    }

    fn from_member(member: &AgentTeamMemberDto) -> Self {
        Self {
            worker_id: member.worker_id.clone(),
            runtime_name: member.runtime_name.clone().unwrap_or_default(),
            role: member.role.clone(),
            room_id: member.room_id.clone().unwrap_or_default(),
            matrix_user_id: member.matrix_user_id.clone().unwrap_or_default(),
            ready: member.ready,
        }
    }

    fn to_member(&self) -> Result<AgentTeamMemberDto, String> {
        Ok(AgentTeamMemberDto {
            worker_id: required_trimmed(&self.worker_id, "team member worker id")?,
            runtime_name: optional_trimmed(&self.runtime_name),
            role: required_trimmed(&self.role, "team member role")?,
            room_id: optional_trimmed(&self.room_id),
            matrix_user_id: optional_trimmed(&self.matrix_user_id),
            ready: self.ready,
        })
    }
}

pub(crate) fn resource_project_path(project_path: Option<&str>) -> Result<String, String> {
    project_path
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .map(str::to_string)
        .ok_or_else(|| "project is required for staff resources".to_string())
}

fn next_resource_id<'a>(prefix: &str, existing: impl Iterator<Item = &'a str>) -> String {
    let existing = existing.collect::<HashSet<_>>();
    if !existing.contains(prefix) {
        return prefix.to_string();
    }
    (2..)
        .map(|index| format!("{prefix}-{index}"))
        .find(|candidate| !existing.contains(candidate.as_str()))
        .expect("resource id sequence is finite")
}

fn required_trimmed(value: &str, field: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() { Err(format!("{field} is required")) } else { Ok(value.to_string()) }
}

fn optional_trimmed(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() { None } else { Some(value.to_string()) }
}

#[cfg(test)]
#[path = "staff_resources_tests.rs"]
mod staff_resources_tests;
