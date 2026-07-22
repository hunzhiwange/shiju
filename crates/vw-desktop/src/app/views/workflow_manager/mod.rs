mod capabilities;

use std::collections::HashSet;
use std::fmt;

use anyhow::{Result, anyhow, bail};
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{
    Space, button, column, container, pick_list, row, scrollable, text, text_input,
};
use iced::{Alignment, Element, Length, Task, Theme};

use crate::app::components::system_settings_common::{
    danger_action_btn_style, primary_action_btn_style, rounded_action_btn_style,
    settings_muted_text_style, settings_panel, settings_pick_list_menu_style,
    settings_pick_list_style, settings_text_input_style, settings_value_badge,
};
use crate::app::components::toast::confirm_dialog;
use crate::app::message::TaskBoardMessage;
#[cfg(test)]
use crate::app::task::staff::models::TaskBoardStageAction;
#[cfg(test)]
use crate::app::task::staff::stage_capabilities::SAVED_WORKFLOW_CAPABILITY_ID;
use crate::app::task::staff::stage_capabilities::StageCapability;
use crate::app::task::staff::stage_capabilities::stage_capability_registry;
#[cfg(test)]
use crate::app::task::task_workflow_by_id;
use crate::app::task::workflow::TaskWorkflowRegistry;
use crate::app::task::{
    OwnedStatusDefinition, OwnedStatusTransition, OwnedTaskWorkflow, StatusOutcome, StatusRole,
    TaskStatus, TaskWorkflow, all_task_workflows, is_builtin_task_workflow_id,
    update_runtime_task_lifecycle_cache,
};
use crate::app::{App, Message};
#[cfg(test)]
use vw_gateway_client::WorkflowRecordSummary;
use vw_gateway_client::vw_api_types::task_lifecycle::{
    TaskLifecycleStatusDto, TaskLifecycleStatusRoleDto, TaskLifecycleStatusTransitionDto,
    TaskLifecycleWorkflowDto, TaskLifecycleWorkflowUpsertBody,
};

const WORKFLOW_SCROLL_RIGHT_GUTTER: f32 = 12.0;
const WORKFLOW_STATUS_INDEX_WIDTH: f32 = 20.0;

pub type LifecycleWorkflowListResult = std::result::Result<Vec<TaskLifecycleWorkflowDto>, String>;

#[derive(Debug, Clone)]
pub enum WorkflowManagerMessage {
    RefreshLifecycles,
    LifecycleWorkflowsLoaded { result: LifecycleWorkflowListResult, notify_error: bool },
    LifecycleWorkflowSaved { workflow_id: String, result: LifecycleWorkflowListResult },
    LifecycleWorkflowDeleted { workflow_id: String, result: LifecycleWorkflowListResult },
    LifecycleWorkflowReset { workflow_id: String, result: LifecycleWorkflowListResult },
    SelectWorkflow(String),
    ViewWorkflow(String),
    CopyWorkflow(String),
    SelectTab(WorkflowManagerTab),
    ToggleStatusRoleHelp,
    ToggleStageActionHelp,
    StartNew,
    StartEditSelected,
    DeleteSelected,
    ResetSelected,
    ConfirmPendingAction,
    CancelPendingConfirmation,
    SaveDraft,
    CancelDraft,
    DraftIdChanged(String),
    DraftLabelChanged(String),
    AddBlankStatus,
    AddDraftStatus(WorkflowDraftStatus),
    RemoveDraftStatus(usize),
    MoveDraftStatusUp(usize),
    MoveDraftStatusDown(usize),
    DraftStatusKeyChanged { index: usize, key: String },
    DraftStatusLabelChanged { index: usize, label: String },
    DraftStatusRoleChanged { index: usize, role: StatusRole },
    ToggleDraftStatusEnabled(usize),
    AddDraftStatusRoute(usize),
    RemoveDraftStatusRoute { status_index: usize, route_index: usize },
    DraftStatusRouteOutcomeChanged { status_index: usize, route_index: usize, outcome_key: String },
    DraftStatusRouteLabelChanged { status_index: usize, route_index: usize, label: String },
    SetDraftStatusRoute { status_index: usize, route_index: usize, target_key: Option<String> },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WorkflowManagerTab {
    #[default]
    LifecycleDefinition,
    StageActions,
    CapabilityManagement,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WorkflowListItem {
    id: String,
    label: String,
    is_builtin: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowManagerState {
    workflows: Vec<WorkflowListItem>,
    selected_workflow_id: Option<String>,
    draft: Option<WorkflowDraft>,
    feedback: Option<WorkflowManagerFeedback>,
    lifecycles_loading: bool,
    lifecycles_loaded: bool,
    lifecycles_error: Option<String>,
    pending_effect: Option<WorkflowManagerEffect>,
    pending_confirmation: Option<WorkflowManagerConfirmation>,
    active_tab: WorkflowManagerTab,
    show_status_role_help: bool,
    show_stage_action_help: bool,
}

impl Default for WorkflowManagerState {
    fn default() -> Self {
        Self::from_runtime_cache(false)
    }
}

impl WorkflowManagerState {
    #[cfg(test)]
    pub(crate) fn from_registry(registry: &TaskWorkflowRegistry) -> Self {
        Self::from_workflows(registry.all_workflows(), true)
    }

    fn from_runtime_cache(lifecycles_loaded: bool) -> Self {
        Self::from_workflows(all_task_workflows(), lifecycles_loaded)
    }

    fn from_workflows(workflows: Vec<TaskWorkflow>, lifecycles_loaded: bool) -> Self {
        let workflows: Vec<WorkflowListItem> = workflows
            .into_iter()
            .map(|workflow| WorkflowListItem {
                is_builtin: is_builtin_task_workflow_id(&workflow.id),
                id: workflow.id,
                label: workflow.label,
            })
            .collect();
        let selected_workflow_id = workflows
            .iter()
            .find(|workflow| !workflow.is_builtin)
            .or_else(|| workflows.first())
            .map(|workflow| workflow.id.clone());
        Self {
            workflows,
            selected_workflow_id,
            draft: None,
            feedback: None,
            lifecycles_loading: false,
            lifecycles_loaded,
            lifecycles_error: None,
            pending_effect: None,
            pending_confirmation: None,
            active_tab: WorkflowManagerTab::LifecycleDefinition,
            show_status_role_help: false,
            show_stage_action_help: false,
        }
    }

    fn refresh_from_runtime_cache(&mut self) {
        self.refresh_from_workflows(all_task_workflows());
    }

    fn refresh_from_workflows(&mut self, workflows: Vec<TaskWorkflow>) {
        let selected = self.selected_workflow_id.clone();
        let refreshed = Self::from_workflows(workflows, self.lifecycles_loaded);
        self.workflows = refreshed.workflows;
        self.selected_workflow_id = selected
            .filter(|id| self.workflows.iter().any(|workflow| workflow.id == *id))
            .or(refreshed.selected_workflow_id);
    }

    pub fn workflow_ids(&self) -> Vec<String> {
        self.workflows.iter().map(|workflow| workflow.id.clone()).collect()
    }

    pub fn selected_workflow_id(&self) -> Option<&str> {
        self.selected_workflow_id.as_deref()
    }

    pub fn feedback(&self) -> Option<&WorkflowManagerFeedback> {
        self.feedback.as_ref()
    }

    pub fn lifecycles_loading(&self) -> bool {
        self.lifecycles_loading
    }

    pub fn lifecycles_loaded(&self) -> bool {
        self.lifecycles_loaded
    }

    pub fn lifecycles_error(&self) -> Option<&str> {
        self.lifecycles_error.as_deref()
    }

    pub fn status_role_help_visible(&self) -> bool {
        self.show_status_role_help
    }

    pub fn stage_action_help_visible(&self) -> bool {
        self.show_stage_action_help
    }

    pub fn active_tab(&self) -> WorkflowManagerTab {
        self.active_tab
    }

    fn selected_item(&self) -> Option<&WorkflowListItem> {
        let selected = self.selected_workflow_id.as_deref()?;
        self.workflows.iter().find(|workflow| workflow.id == selected)
    }

    fn take_pending_effect(&mut self) -> Option<WorkflowManagerEffect> {
        self.pending_effect.take()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum WorkflowManagerConfirmation {
    ResetWorkflow { workflow_id: String, workflow_label: String },
    DeleteWorkflow { workflow_id: String, workflow_label: String },
    DeleteStatus { index: usize, key: String, label: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WorkflowDraft {
    original_id: Option<String>,
    id: String,
    label: String,
    statuses: Vec<WorkflowDraftStatus>,
}

impl WorkflowDraft {
    fn empty() -> Self {
        Self { original_id: None, id: String::new(), label: String::new(), statuses: Vec::new() }
    }

    fn from_workflow(workflow: TaskWorkflow) -> Self {
        Self {
            original_id: Some(workflow.id.clone()),
            id: workflow.id,
            label: workflow.label,
            statuses: workflow
                .statuses
                .into_iter()
                .map(WorkflowDraftStatus::from_definition)
                .collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowDraftStatus {
    key: String,
    label: String,
    role: StatusRole,
    enabled: bool,
    routes: Vec<WorkflowDraftRoute>,
    success_overridden: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WorkflowDraftRoute {
    outcome_key: String,
    label: String,
    target_key: Option<String>,
}

impl WorkflowDraftRoute {
    fn new(outcome: StatusOutcome, target_key: Option<String>) -> Self {
        Self { outcome_key: outcome.key().to_string(), label: outcome.label(), target_key }
    }

    fn outcome(&self) -> StatusOutcome {
        StatusOutcome::new(self.outcome_key.as_str())
    }

    fn is_builtin(&self) -> bool {
        let outcome = self.outcome();
        outcome.is_succeed() || outcome.is_failed() || outcome.is_retry()
    }
}

impl WorkflowDraftStatus {
    pub fn new(key: &str, label: &str, role: StatusRole) -> Self {
        Self {
            key: key.to_string(),
            label: label.to_string(),
            role,
            enabled: true,
            routes: Self::default_routes(),
            success_overridden: false,
        }
    }

    fn blank() -> Self {
        Self::new("", "", StatusRole::Custom)
    }

    fn from_definition(definition: OwnedStatusDefinition) -> Self {
        let transitions = definition.outgoing_transitions();
        let mut draft = Self {
            key: definition.key,
            label: definition.label,
            role: definition.role,
            enabled: definition.enabled,
            routes: Self::default_routes(),
            success_overridden: true,
        };
        for transition in transitions {
            draft.set_or_push_route(
                transition.outcome,
                transition.label,
                Some(transition.target.to_string_key().to_string()),
            );
        }
        draft
    }

    fn default_routes() -> Vec<WorkflowDraftRoute> {
        vec![
            WorkflowDraftRoute::new(StatusOutcome::succeed(), None),
            WorkflowDraftRoute::new(StatusOutcome::failed(), None),
            WorkflowDraftRoute::new(StatusOutcome::retry(), None),
        ]
    }

    fn route_target_key(
        &self,
        route_index: usize,
        default_success_key: Option<String>,
    ) -> Option<String> {
        let route = self.routes.get(route_index)?;
        if route.outcome().is_succeed() && !self.success_overridden {
            return route.target_key.clone().or(default_success_key);
        }
        route.target_key.clone()
    }

    fn set_route_target_key(&mut self, route_index: usize, target_key: Option<String>) {
        if let Some(route) = self.routes.get_mut(route_index) {
            if route.outcome().is_succeed() {
                self.success_overridden = true;
            }
            route.target_key = target_key;
        }
    }

    fn set_route_outcome_key(&mut self, route_index: usize, outcome_key: String) {
        if let Some(route) = self.routes.get_mut(route_index)
            && !route.is_builtin()
        {
            route.outcome_key = outcome_key;
        }
    }

    fn set_route_label(&mut self, route_index: usize, label: String) {
        if let Some(route) = self.routes.get_mut(route_index)
            && !route.is_builtin()
        {
            route.label = label;
        }
    }

    fn add_route(&mut self) {
        let mut index = 1;
        loop {
            let key = if index == 1 {
                "manual_review".to_string()
            } else {
                format!("manual_review_{index}")
            };
            if !self.routes.iter().any(|route| route.outcome().key() == key) {
                self.routes.push(WorkflowDraftRoute {
                    outcome_key: key,
                    label: "人工复核".to_string(),
                    target_key: None,
                });
                return;
            }
            index += 1;
        }
    }

    fn remove_route(&mut self, route_index: usize) {
        if self.routes.get(route_index).is_some_and(|route| !route.is_builtin()) {
            self.routes.remove(route_index);
        }
    }

    fn set_or_push_route(
        &mut self,
        outcome: StatusOutcome,
        label: Option<String>,
        target_key: Option<String>,
    ) {
        if let Some(route) = self.routes.iter_mut().find(|route| route.outcome() == outcome) {
            if let Some(label) = label {
                route.label = label;
            }
            route.target_key = target_key;
            return;
        }
        let mut route = WorkflowDraftRoute::new(outcome, target_key);
        if let Some(label) = label {
            route.label = label;
        }
        self.routes.push(route);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowManagerFeedback {
    kind: WorkflowManagerFeedbackKind,
    message: String,
}

impl WorkflowManagerFeedback {
    fn success(message: impl Into<String>) -> Self {
        Self { kind: WorkflowManagerFeedbackKind::Success, message: message.into() }
    }

    fn error(message: impl Into<String>) -> Self {
        Self { kind: WorkflowManagerFeedbackKind::Error, message: message.into() }
    }

    pub fn is_success(&self) -> bool {
        self.kind == WorkflowManagerFeedbackKind::Success
    }

    pub fn is_error(&self) -> bool {
        self.kind == WorkflowManagerFeedbackKind::Error
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WorkflowManagerFeedbackKind {
    Success,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum WorkflowManagerEffect {
    LoadLifecycles {
        notify_error: bool,
    },
    SaveLifecycle {
        workflow_id: String,
        original_id: Option<String>,
        body: TaskLifecycleWorkflowUpsertBody,
    },
    DeleteLifecycle {
        workflow_id: String,
    },
    ResetLifecycle {
        workflow_id: String,
    },
}

pub(crate) fn load_lifecycle_workflows_task(notify_error: bool) -> Task<Message> {
    Task::perform(load_lifecycle_workflows_via_gateway(), move |result| {
        Message::WorkflowManager(WorkflowManagerMessage::LifecycleWorkflowsLoaded {
            result,
            notify_error,
        })
    })
}

fn workflow_manager_effect_task(effect: WorkflowManagerEffect) -> Task<Message> {
    match effect {
        WorkflowManagerEffect::LoadLifecycles { notify_error } => {
            load_lifecycle_workflows_task(notify_error)
        }
        WorkflowManagerEffect::SaveLifecycle { workflow_id, original_id, body } => {
            let message_workflow_id = workflow_id.clone();
            Task::perform(save_lifecycle_workflow_via_gateway(original_id, body), move |result| {
                Message::WorkflowManager(WorkflowManagerMessage::LifecycleWorkflowSaved {
                    workflow_id: message_workflow_id.clone(),
                    result,
                })
            })
        }
        WorkflowManagerEffect::DeleteLifecycle { workflow_id } => {
            let message_workflow_id = workflow_id.clone();
            Task::perform(delete_lifecycle_workflow_via_gateway(workflow_id), move |result| {
                Message::WorkflowManager(WorkflowManagerMessage::LifecycleWorkflowDeleted {
                    workflow_id: message_workflow_id.clone(),
                    result,
                })
            })
        }
        WorkflowManagerEffect::ResetLifecycle { workflow_id } => {
            let message_workflow_id = workflow_id.clone();
            Task::perform(reset_lifecycle_workflow_via_gateway(workflow_id), move |result| {
                Message::WorkflowManager(WorkflowManagerMessage::LifecycleWorkflowReset {
                    workflow_id: message_workflow_id.clone(),
                    result,
                })
            })
        }
    }
}

async fn load_lifecycle_workflows_via_gateway() -> LifecycleWorkflowListResult {
    let client = crate::app::config::gateway_client()?;
    client.task_lifecycle_workflows_list().await
}

async fn save_lifecycle_workflow_via_gateway(
    original_id: Option<String>,
    body: TaskLifecycleWorkflowUpsertBody,
) -> LifecycleWorkflowListResult {
    let client = crate::app::config::gateway_client()?;
    save_lifecycle_workflow_with_client(&client, original_id.as_deref(), &body).await?;
    client.task_lifecycle_workflows_list().await
}

async fn save_lifecycle_workflow_with_client(
    client: &vw_gateway_client::GatewayClient,
    original_id: Option<&str>,
    body: &TaskLifecycleWorkflowUpsertBody,
) -> std::result::Result<(), String> {
    let workflow_id = body.workflow.id.as_str();
    match original_id.map(str::trim).filter(|id| !id.is_empty()) {
        Some(original_id) if original_id == workflow_id => {
            client.task_lifecycle_workflows_update(original_id, body).await?;
        }
        Some(original_id) => {
            client.task_lifecycle_workflows_create(body).await?;
            client.task_lifecycle_workflows_delete(original_id).await?;
        }
        None => {
            client.task_lifecycle_workflows_create(body).await?;
        }
    }
    Ok(())
}

async fn delete_lifecycle_workflow_via_gateway(workflow_id: String) -> LifecycleWorkflowListResult {
    if is_builtin_task_workflow_id(&workflow_id) {
        return Err(format!("内置生命周期不能删除: {workflow_id}"));
    }
    let client = crate::app::config::gateway_client()?;
    client.task_lifecycle_workflows_delete(&workflow_id).await?;
    client.task_lifecycle_workflows_list().await
}

async fn reset_lifecycle_workflow_via_gateway(workflow_id: String) -> LifecycleWorkflowListResult {
    if !is_builtin_task_workflow_id(&workflow_id) {
        return Err(format!("只有内置生命周期可以重置: {workflow_id}"));
    }
    let client = crate::app::config::gateway_client()?;
    client.task_lifecycle_workflows_reset(&workflow_id).await?;
    client.task_lifecycle_workflows_list().await
}

pub(crate) fn update(
    state: &mut WorkflowManagerState,
    registry: &mut TaskWorkflowRegistry,
    message: WorkflowManagerMessage,
) -> bool {
    match message {
        WorkflowManagerMessage::RefreshLifecycles => {
            state.lifecycles_loading = true;
            state.feedback = None;
            state.pending_effect =
                Some(WorkflowManagerEffect::LoadLifecycles { notify_error: true });
            false
        }
        WorkflowManagerMessage::LifecycleWorkflowsLoaded { result, notify_error } => {
            apply_lifecycle_workflows_loaded(state, result, notify_error);
            false
        }
        WorkflowManagerMessage::LifecycleWorkflowSaved { workflow_id, result } => {
            apply_lifecycle_workflow_saved(state, workflow_id, result);
            false
        }
        WorkflowManagerMessage::LifecycleWorkflowDeleted { workflow_id, result } => {
            apply_lifecycle_workflow_deleted(state, workflow_id, result);
            false
        }
        WorkflowManagerMessage::LifecycleWorkflowReset { workflow_id, result } => {
            apply_lifecycle_workflow_reset(state, workflow_id, result);
            false
        }
        WorkflowManagerMessage::SelectWorkflow(id) => {
            if state.workflows.iter().any(|workflow| workflow.id == id) {
                state.selected_workflow_id = Some(id);
                state.feedback = None;
            }
            false
        }
        WorkflowManagerMessage::ViewWorkflow(id) => {
            if state.workflows.iter().any(|workflow| workflow.id == id) {
                state.selected_workflow_id = Some(id);
                state.draft = None;
                state.feedback = None;
            }
            false
        }
        WorkflowManagerMessage::CopyWorkflow(id) => copy_workflow_to_draft(state, registry, &id),
        WorkflowManagerMessage::SelectTab(tab) => {
            state.active_tab = tab;
            state.feedback = None;
            false
        }
        WorkflowManagerMessage::ToggleStatusRoleHelp => {
            state.show_status_role_help = !state.show_status_role_help;
            false
        }
        WorkflowManagerMessage::ToggleStageActionHelp => {
            state.show_stage_action_help = !state.show_stage_action_help;
            false
        }
        WorkflowManagerMessage::StartNew => {
            state.active_tab = WorkflowManagerTab::LifecycleDefinition;
            state.draft = Some(WorkflowDraft::empty());
            state.feedback = None;
            false
        }
        WorkflowManagerMessage::StartEditSelected => {
            let Some(selected) = state.selected_item() else {
                state.feedback = Some(WorkflowManagerFeedback::error("请先选择工作流"));
                return false;
            };
            match registry.workflow_by_id(&selected.id) {
                Some(workflow) => {
                    state.active_tab = WorkflowManagerTab::LifecycleDefinition;
                    state.draft = Some(WorkflowDraft::from_workflow(workflow));
                    state.feedback = None;
                }
                None => {
                    state.feedback = Some(WorkflowManagerFeedback::error("选中的工作流不存在"));
                }
            }
            false
        }
        WorkflowManagerMessage::DeleteSelected => request_delete_selected_workflow(state),
        WorkflowManagerMessage::ResetSelected => request_reset_selected_workflow(state),
        WorkflowManagerMessage::ConfirmPendingAction => confirm_pending_action(state),
        WorkflowManagerMessage::CancelPendingConfirmation => {
            state.pending_confirmation = None;
            false
        }
        WorkflowManagerMessage::SaveDraft => save_draft(state, registry),
        WorkflowManagerMessage::CancelDraft => {
            state.draft = None;
            state.feedback = None;
            false
        }
        WorkflowManagerMessage::DraftIdChanged(id) => {
            if let Some(draft) = &mut state.draft {
                draft.id = id;
            }
            false
        }
        WorkflowManagerMessage::DraftLabelChanged(label) => {
            if let Some(draft) = &mut state.draft {
                draft.label = label;
            }
            false
        }
        WorkflowManagerMessage::AddBlankStatus => {
            if let Some(draft) = &mut state.draft {
                draft.statuses.push(WorkflowDraftStatus::blank());
            }
            false
        }
        WorkflowManagerMessage::AddDraftStatus(status) => {
            if let Some(draft) = &mut state.draft {
                draft.statuses.push(status);
            }
            false
        }
        WorkflowManagerMessage::RemoveDraftStatus(index) => {
            request_remove_draft_status(state, index)
        }
        WorkflowManagerMessage::MoveDraftStatusUp(index) => {
            if let Some(draft) = &mut state.draft
                && index > 0
                && index < draft.statuses.len()
            {
                draft.statuses.swap(index - 1, index);
            }
            false
        }
        WorkflowManagerMessage::MoveDraftStatusDown(index) => {
            if let Some(draft) = &mut state.draft
                && index + 1 < draft.statuses.len()
            {
                draft.statuses.swap(index, index + 1);
            }
            false
        }
        WorkflowManagerMessage::DraftStatusKeyChanged { index, key } => {
            if let Some(status) =
                state.draft.as_mut().and_then(|draft| draft.statuses.get_mut(index))
            {
                status.key = key;
            }
            false
        }
        WorkflowManagerMessage::DraftStatusLabelChanged { index, label } => {
            if let Some(status) =
                state.draft.as_mut().and_then(|draft| draft.statuses.get_mut(index))
            {
                status.label = label;
            }
            false
        }
        WorkflowManagerMessage::DraftStatusRoleChanged { index, role } => {
            if let Some(status) =
                state.draft.as_mut().and_then(|draft| draft.statuses.get_mut(index))
            {
                status.role = role;
            }
            false
        }
        WorkflowManagerMessage::ToggleDraftStatusEnabled(index) => {
            if let Some(status) =
                state.draft.as_mut().and_then(|draft| draft.statuses.get_mut(index))
            {
                status.enabled = !status.enabled;
            }
            false
        }
        WorkflowManagerMessage::AddDraftStatusRoute(index) => {
            if let Some(status) =
                state.draft.as_mut().and_then(|draft| draft.statuses.get_mut(index))
            {
                status.add_route();
            }
            false
        }
        WorkflowManagerMessage::RemoveDraftStatusRoute { status_index, route_index } => {
            if let Some(status) =
                state.draft.as_mut().and_then(|draft| draft.statuses.get_mut(status_index))
            {
                status.remove_route(route_index);
            }
            false
        }
        WorkflowManagerMessage::DraftStatusRouteOutcomeChanged {
            status_index,
            route_index,
            outcome_key,
        } => {
            if let Some(status) =
                state.draft.as_mut().and_then(|draft| draft.statuses.get_mut(status_index))
            {
                status.set_route_outcome_key(route_index, outcome_key);
            }
            false
        }
        WorkflowManagerMessage::DraftStatusRouteLabelChanged {
            status_index,
            route_index,
            label,
        } => {
            if let Some(status) =
                state.draft.as_mut().and_then(|draft| draft.statuses.get_mut(status_index))
            {
                status.set_route_label(route_index, label);
            }
            false
        }
        WorkflowManagerMessage::SetDraftStatusRoute { status_index, route_index, target_key } => {
            if let Some(status) =
                state.draft.as_mut().and_then(|draft| draft.statuses.get_mut(status_index))
            {
                status.set_route_target_key(route_index, target_key);
            }
            false
        }
    }
}

pub fn update_app(app: &mut App, message: WorkflowManagerMessage) -> Task<Message> {
    let selected_workflow_changed = matches!(message, WorkflowManagerMessage::SelectWorkflow(_));
    let load_stage_workflow_options =
        matches!(message, WorkflowManagerMessage::SelectTab(WorkflowManagerTab::StageActions));
    let load_capability_catalog_for_tab = matches!(
        message,
        WorkflowManagerMessage::SelectTab(
            WorkflowManagerTab::StageActions | WorkflowManagerTab::CapabilityManagement
        )
    );
    let mut registry = registry_from_runtime();
    let _changed = update(&mut app.workflow_manager, &mut registry, message);
    let workflow_id = workflow_manager_stage_action_workflow_id(app);
    let active_tab = app.workflow_manager.active_tab();
    let reload_staff_config = load_stage_workflow_options
        || (selected_workflow_changed && active_tab == WorkflowManagerTab::StageActions);
    let capability_catalog_matches = app.task_board_staff_capability_catalog_workflow_id.as_deref()
        == Some(workflow_id.as_str())
        && !app.task_board_staff_capability_catalog.is_empty();
    let load_capability_catalog = (load_capability_catalog_for_tab && !capability_catalog_matches)
        || (selected_workflow_changed
            && matches!(
                active_tab,
                WorkflowManagerTab::StageActions | WorkflowManagerTab::CapabilityManagement
            ));
    if reload_staff_config {
        app.task_board_staff_config_load_error = None;
    }
    if load_capability_catalog {
        app.task_board_staff_capability_catalog_error = None;
    }
    let stage_workflow_options_task = if load_stage_workflow_options {
        Some(Task::done(Message::TaskBoard(TaskBoardMessage::LoadStaffStageWorkflowOptions)))
    } else {
        None
    };
    let staff_config_reload_task = if reload_staff_config {
        Some(Task::done(Message::TaskBoard(TaskBoardMessage::ReloadStaffConfigForWorkflow {
            workflow_id: workflow_id.clone(),
        })))
    } else {
        None
    };
    let capability_catalog_task = if load_capability_catalog {
        Some(Task::done(Message::TaskBoard(
            TaskBoardMessage::LoadStaffCapabilityCatalogForWorkflow {
                workflow_id: workflow_id.clone(),
            },
        )))
    } else {
        None
    };
    if let Some(effect) = app.workflow_manager.take_pending_effect() {
        let effect_task = workflow_manager_effect_task(effect);
        let mut tasks = vec![effect_task];
        if let Some(task) = stage_workflow_options_task {
            tasks.push(task);
        }
        if let Some(task) = staff_config_reload_task {
            tasks.push(task);
        }
        if let Some(task) = capability_catalog_task {
            tasks.push(task);
        }
        return Task::batch(tasks);
    }
    let feedback_task = if let Some(feedback) = app.workflow_manager.feedback().cloned() {
        if feedback.is_error() {
            app.show_error_toast(feedback.message)
        } else if feedback.is_success() {
            app.show_success_toast(feedback.message)
        } else {
            Task::none()
        }
    } else {
        Task::none()
    };
    let mut tasks = vec![feedback_task];
    if let Some(task) = stage_workflow_options_task {
        tasks.push(task);
    }
    if let Some(task) = staff_config_reload_task {
        tasks.push(task);
    }
    if let Some(task) = capability_catalog_task {
        tasks.push(task);
    }
    Task::batch(tasks)
}

fn registry_from_runtime() -> TaskWorkflowRegistry {
    let mut registry = TaskWorkflowRegistry::new();
    if registry.replace_custom_workflows(runtime_custom_workflows()).is_err() {
        TaskWorkflowRegistry::new()
    } else {
        registry
    }
}

fn runtime_custom_workflows() -> Vec<OwnedTaskWorkflow> {
    all_task_workflows().into_iter().map(owned_workflow_from_snapshot).collect()
}

fn custom_workflows_from_registry(registry: &TaskWorkflowRegistry) -> Vec<OwnedTaskWorkflow> {
    registry.all_workflows().into_iter().map(owned_workflow_from_snapshot).collect()
}

fn owned_workflow_from_snapshot(workflow: TaskWorkflow) -> OwnedTaskWorkflow {
    OwnedTaskWorkflow { id: workflow.id, label: workflow.label, statuses: workflow.statuses }
}

fn request_delete_selected_workflow(state: &mut WorkflowManagerState) -> bool {
    let Some(selected) = state.selected_item() else {
        state.feedback = Some(WorkflowManagerFeedback::error("请先选择工作流"));
        return false;
    };
    if selected.is_builtin {
        state.feedback = Some(WorkflowManagerFeedback::error("内置工作流不能删除"));
        return false;
    }
    let workflow_id = selected.id.clone();
    let workflow_label = selected.label.clone();

    state.pending_confirmation =
        Some(WorkflowManagerConfirmation::DeleteWorkflow { workflow_id, workflow_label });
    state.feedback = None;
    false
}

fn request_reset_selected_workflow(state: &mut WorkflowManagerState) -> bool {
    let Some(selected) = state.selected_item() else {
        state.feedback = Some(WorkflowManagerFeedback::error("请先选择工作流"));
        return false;
    };
    if !selected.is_builtin {
        state.feedback = Some(WorkflowManagerFeedback::error("只有内置工作流可以重置"));
        return false;
    }
    let workflow_id = selected.id.clone();
    let workflow_label = selected.label.clone();

    state.pending_confirmation =
        Some(WorkflowManagerConfirmation::ResetWorkflow { workflow_id, workflow_label });
    state.feedback = None;
    false
}

fn request_remove_draft_status(state: &mut WorkflowManagerState, index: usize) -> bool {
    let Some(status) = state.draft.as_ref().and_then(|draft| draft.statuses.get(index)) else {
        state.feedback = Some(WorkflowManagerFeedback::error("要删除的阶段不存在"));
        return false;
    };
    state.pending_confirmation = Some(WorkflowManagerConfirmation::DeleteStatus {
        index,
        key: status.key.clone(),
        label: status.label.clone(),
    });
    state.feedback = None;
    false
}

fn confirm_pending_action(state: &mut WorkflowManagerState) -> bool {
    let Some(confirmation) = state.pending_confirmation.take() else {
        return false;
    };
    match confirmation {
        WorkflowManagerConfirmation::ResetWorkflow { workflow_id, .. } => {
            state.lifecycles_loading = true;
            state.feedback = None;
            state.pending_effect = Some(WorkflowManagerEffect::ResetLifecycle { workflow_id });
        }
        WorkflowManagerConfirmation::DeleteWorkflow { workflow_id, .. } => {
            state.lifecycles_loading = true;
            state.feedback = None;
            state.pending_effect = Some(WorkflowManagerEffect::DeleteLifecycle { workflow_id });
        }
        WorkflowManagerConfirmation::DeleteStatus { index, key, .. } => {
            let Some(draft) = state.draft.as_mut() else {
                state.feedback = Some(WorkflowManagerFeedback::error("没有可编辑草稿"));
                return false;
            };
            let remove_index = if draft.statuses.get(index).is_some_and(|status| status.key == key)
            {
                Some(index)
            } else if key.is_empty() {
                (index < draft.statuses.len()).then_some(index)
            } else {
                draft.statuses.iter().position(|status| status.key == key)
            };
            let Some(remove_index) = remove_index else {
                state.feedback = Some(WorkflowManagerFeedback::error("要删除的阶段不存在"));
                return false;
            };
            draft.statuses.remove(remove_index);
            state.feedback = None;
        }
    }
    false
}

fn copy_workflow_to_draft(
    state: &mut WorkflowManagerState,
    registry: &TaskWorkflowRegistry,
    workflow_id: &str,
) -> bool {
    let Some(workflow) = registry.workflow_by_id(workflow_id) else {
        state.feedback = Some(WorkflowManagerFeedback::error("要复制的工作流不存在"));
        return false;
    };
    let used_ids: HashSet<String> =
        registry.all_workflows().into_iter().map(|workflow| workflow.id).collect();
    let mut draft = WorkflowDraft::from_workflow(workflow);
    draft.original_id = None;
    draft.id = unique_copy_id(draft.id.as_str(), &used_ids);
    draft.label = format!("{} 副本", draft.label);
    state.selected_workflow_id = Some(workflow_id.to_string());
    state.active_tab = WorkflowManagerTab::LifecycleDefinition;
    state.draft = Some(draft);
    state.feedback = None;
    false
}

fn unique_copy_id(source_id: &str, used_ids: &HashSet<String>) -> String {
    let base = format!("{}_copy", source_id.trim());
    if !used_ids.contains(&base) {
        return base;
    }
    let mut index = 2;
    loop {
        let candidate = format!("{base}_{index}");
        if !used_ids.contains(&candidate) {
            return candidate;
        }
        index += 1;
    }
}

fn save_draft(state: &mut WorkflowManagerState, registry: &mut TaskWorkflowRegistry) -> bool {
    let Some(draft) = state.draft.clone() else {
        state.feedback = Some(WorkflowManagerFeedback::error("没有正在编辑的工作流"));
        return false;
    };
    match lifecycle_save_command_from_draft(&draft, registry) {
        Ok(command) => {
            state.lifecycles_loading = true;
            state.feedback = None;
            state.pending_effect = Some(WorkflowManagerEffect::SaveLifecycle {
                workflow_id: command.workflow_id,
                original_id: command.original_id,
                body: command.body,
            });
            false
        }
        Err(err) => {
            state.feedback = Some(WorkflowManagerFeedback::error(err.to_string()));
            false
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WorkflowLifecycleSaveCommand {
    workflow_id: String,
    original_id: Option<String>,
    body: TaskLifecycleWorkflowUpsertBody,
}

fn lifecycle_save_command_from_draft(
    draft: &WorkflowDraft,
    registry: &TaskWorkflowRegistry,
) -> Result<WorkflowLifecycleSaveCommand> {
    let workflow = owned_workflow_from_draft(draft)?;
    if let Some(original_id) = draft.original_id.as_deref()
        && is_builtin_task_workflow_id(original_id)
        && workflow.id != original_id
    {
        bail!("内置工作流 ID 不能修改，请复制后另存");
    }
    let existing_custom_workflows = custom_workflows_from_registry(registry);
    let original_id = draft.original_id.clone().or_else(|| {
        existing_custom_workflows
            .iter()
            .any(|existing| existing.id == workflow.id)
            .then(|| workflow.id.clone())
    });
    let remove_id = original_id.as_deref().unwrap_or(workflow.id.as_str());
    let mut workflows: Vec<OwnedTaskWorkflow> = existing_custom_workflows
        .into_iter()
        .filter(|existing| existing.id != remove_id && existing.id != workflow.id)
        .collect();
    workflows.push(workflow.clone());
    let mut validation_registry = TaskWorkflowRegistry::new();
    validation_registry.replace_custom_workflows(workflows)?;
    Ok(WorkflowLifecycleSaveCommand {
        workflow_id: workflow.id.clone(),
        original_id,
        body: TaskLifecycleWorkflowUpsertBody {
            workflow: lifecycle_workflow_dto_from_owned_workflow(workflow),
        },
    })
}

fn owned_workflow_from_draft(draft: &WorkflowDraft) -> Result<OwnedTaskWorkflow> {
    let id = draft.id.trim();
    if id.is_empty() {
        bail!("自定义工作流 id 不能为空");
    }
    let label = draft.label.trim();
    if label.is_empty() {
        bail!("自定义工作流 label 不能为空: {id}");
    }

    let mut statuses = Vec::with_capacity(draft.statuses.len());
    for (index, status) in draft.statuses.iter().enumerate() {
        let key = status.key.trim();
        let task_status =
            TaskStatus::parse_key(key).ok_or_else(|| anyhow!("工作流 {id} 状态 key 不能为空"))?;
        let default_success_key =
            draft.statuses.get(index + 1).map(|next| next.key.trim().to_string());
        let mut transitions = Vec::new();
        let mut route_keys = HashSet::new();
        for (route_index, route) in status.routes.iter().enumerate() {
            let outcome = StatusOutcome::new(route.outcome_key.trim());
            if outcome.key().is_empty() {
                bail!("工作流 {id} 状态 {key} 的结果 key 不能为空");
            }
            if !route_keys.insert(outcome.key().to_string()) {
                bail!("工作流 {id} 状态 {key} 的结果 key 重复: {}", outcome.key());
            }
            let target = parse_route_target(
                id,
                key,
                &outcome,
                status.route_target_key(route_index, default_success_key.clone()).as_deref(),
            )?;
            if let Some(target) = target {
                let label = normalized_route_label(&route.label, &outcome);
                transitions.push(OwnedStatusTransition { outcome, label, target });
            }
        }
        let next = transitions
            .iter()
            .find(|transition| transition.outcome.is_succeed())
            .map(|transition| transition.target);
        statuses.push(OwnedStatusDefinition {
            status: task_status,
            key: key.to_string(),
            label: status.label.trim().to_string(),
            role: status.role,
            enabled: status.enabled,
            is_terminal: transitions.is_empty(),
            auto_execution_candidate: matches!(
                status.role,
                StatusRole::Backlog | StatusRole::Queued | StatusRole::Planning
            ),
            occupies_execution_slot: matches!(
                status.role,
                StatusRole::Planning | StatusRole::Executing
            ),
            is_scheduling_backlog: matches!(
                status.role,
                StatusRole::Backlog | StatusRole::Queued | StatusRole::Planning
            ),
            next,
            transitions,
        });
    }

    Ok(OwnedTaskWorkflow { id: id.to_string(), label: label.to_string(), statuses })
}

fn parse_route_target(
    workflow_id: &str,
    status_key: &str,
    outcome: &StatusOutcome,
    target_key: Option<&str>,
) -> Result<Option<TaskStatus>> {
    let Some(target_key) = target_key.map(str::trim).filter(|key| !key.is_empty()) else {
        return Ok(None);
    };
    TaskStatus::parse_key(target_key).map(Some).ok_or_else(|| {
        anyhow!("工作流 {workflow_id} 状态 {status_key} 的 {} 目标为空", outcome.label())
    })
}

fn normalized_route_label(label: &str, outcome: &StatusOutcome) -> Option<String> {
    let label = label.trim();
    if label.is_empty() || label == outcome.label() { None } else { Some(label.to_string()) }
}

fn lifecycle_workflow_dto_from_owned_workflow(
    workflow: OwnedTaskWorkflow,
) -> TaskLifecycleWorkflowDto {
    let statuses = workflow.statuses;
    TaskLifecycleWorkflowDto {
        id: workflow.id,
        label: workflow.label,
        built_in: false,
        statuses: statuses
            .iter()
            .map(|status| TaskLifecycleStatusDto {
                key: status.key.clone(),
                label: status.label.clone(),
                role: lifecycle_status_role_from_local(status.role),
                enabled: status.enabled,
                terminal: status.is_terminal,
                auto_execution_candidate: status.auto_execution_candidate,
                occupies_execution_slot: status.occupies_execution_slot,
                is_scheduling_backlog: status.is_scheduling_backlog,
                next: status.next.map(|target| lifecycle_status_key_for_target(&statuses, target)),
                transitions: status
                    .transitions
                    .iter()
                    .map(|transition| TaskLifecycleStatusTransitionDto {
                        outcome: transition.outcome.key().to_string(),
                        label: transition.label.clone(),
                        target: lifecycle_status_key_for_target(&statuses, transition.target),
                    })
                    .collect(),
            })
            .collect(),
    }
}

fn lifecycle_status_key_for_target(
    statuses: &[OwnedStatusDefinition],
    target: TaskStatus,
) -> String {
    statuses
        .iter()
        .find(|definition| definition.status == target)
        .map(|definition| definition.key.clone())
        .unwrap_or_else(|| target.to_string_key().to_string())
}

fn lifecycle_status_role_from_local(role: StatusRole) -> TaskLifecycleStatusRoleDto {
    match role {
        StatusRole::Backlog => TaskLifecycleStatusRoleDto::Backlog,
        StatusRole::Queued => TaskLifecycleStatusRoleDto::Queued,
        StatusRole::Planning => TaskLifecycleStatusRoleDto::Planning,
        StatusRole::Executing => TaskLifecycleStatusRoleDto::Executing,
        StatusRole::Failed => TaskLifecycleStatusRoleDto::Failed,
        StatusRole::Paused => TaskLifecycleStatusRoleDto::Paused,
        StatusRole::ReviewReady => TaskLifecycleStatusRoleDto::ReviewReady,
        StatusRole::Review => TaskLifecycleStatusRoleDto::Review,
        StatusRole::Merging => TaskLifecycleStatusRoleDto::Merging,
        StatusRole::Done => TaskLifecycleStatusRoleDto::Done,
        StatusRole::Archived => TaskLifecycleStatusRoleDto::Archived,
        StatusRole::Custom => TaskLifecycleStatusRoleDto::Custom,
    }
}

fn apply_lifecycle_workflows_loaded(
    state: &mut WorkflowManagerState,
    result: LifecycleWorkflowListResult,
    notify_error: bool,
) {
    match result {
        Ok(workflows) => match apply_lifecycle_workflow_cache(state, workflows, None) {
            Ok(()) => {
                state.feedback = None;
            }
            Err(err) if notify_error => {
                state.feedback = Some(WorkflowManagerFeedback::error(err.to_string()));
            }
            Err(err) => {
                state.lifecycles_error = Some(err.to_string());
                state.feedback = None;
            }
        },
        Err(error) => {
            state.lifecycles_loading = false;
            state.lifecycles_error = Some(error.clone());
            if notify_error {
                state.feedback =
                    Some(WorkflowManagerFeedback::error(format!("生命周期列表加载失败：{error}")));
            } else {
                state.feedback = None;
            }
        }
    }
}

fn apply_lifecycle_workflow_saved(
    state: &mut WorkflowManagerState,
    workflow_id: String,
    result: LifecycleWorkflowListResult,
) {
    match result {
        Ok(workflows) => {
            match apply_lifecycle_workflow_cache(state, workflows, Some(workflow_id)) {
                Ok(()) => {
                    state.draft = None;
                    state.feedback = Some(WorkflowManagerFeedback::success("工作流已保存"));
                }
                Err(err) => {
                    state.feedback = Some(WorkflowManagerFeedback::error(err.to_string()));
                }
            }
        }
        Err(error) => {
            state.lifecycles_loading = false;
            state.lifecycles_error = Some(error.clone());
            state.feedback =
                Some(WorkflowManagerFeedback::error(format!("工作流保存失败：{error}")));
        }
    }
}

fn apply_lifecycle_workflow_deleted(
    state: &mut WorkflowManagerState,
    workflow_id: String,
    result: LifecycleWorkflowListResult,
) {
    match result {
        Ok(workflows) => match apply_lifecycle_workflow_cache(state, workflows, None) {
            Ok(()) => {
                if state.selected_workflow_id.as_deref() == Some(workflow_id.as_str()) {
                    state.selected_workflow_id =
                        state.workflows.first().map(|workflow| workflow.id.clone());
                }
                state.draft = None;
                state.feedback = Some(WorkflowManagerFeedback::success("工作流已删除"));
            }
            Err(err) => {
                state.feedback = Some(WorkflowManagerFeedback::error(err.to_string()));
            }
        },
        Err(error) => {
            state.lifecycles_loading = false;
            state.lifecycles_error = Some(error.clone());
            state.feedback =
                Some(WorkflowManagerFeedback::error(format!("工作流删除失败：{error}")));
        }
    }
}

fn apply_lifecycle_workflow_reset(
    state: &mut WorkflowManagerState,
    workflow_id: String,
    result: LifecycleWorkflowListResult,
) {
    match result {
        Ok(workflows) => {
            match apply_lifecycle_workflow_cache(state, workflows, Some(workflow_id)) {
                Ok(()) => {
                    state.draft = None;
                    state.feedback = Some(WorkflowManagerFeedback::success("工作流已重置"));
                }
                Err(err) => {
                    state.feedback = Some(WorkflowManagerFeedback::error(err.to_string()));
                }
            }
        }
        Err(error) => {
            state.lifecycles_loading = false;
            state.lifecycles_error = Some(error.clone());
            state.feedback =
                Some(WorkflowManagerFeedback::error(format!("工作流重置失败：{error}")));
        }
    }
}

fn apply_lifecycle_workflow_cache(
    state: &mut WorkflowManagerState,
    workflows: Vec<TaskLifecycleWorkflowDto>,
    selected_workflow_id: Option<String>,
) -> Result<()> {
    update_runtime_task_lifecycle_cache(workflows)?;
    state.lifecycles_loading = false;
    state.lifecycles_loaded = true;
    state.lifecycles_error = None;
    state.refresh_from_runtime_cache();
    if let Some(selected_workflow_id) = selected_workflow_id
        && state.workflows.iter().any(|workflow| workflow.id == selected_workflow_id)
    {
        state.selected_workflow_id = Some(selected_workflow_id);
    }
    Ok(())
}

pub fn view(app: &App) -> Element<'_, Message> {
    let state = &app.workflow_manager;
    let body = match state.active_tab {
        WorkflowManagerTab::LifecycleDefinition => {
            let list = build_workflow_list(state);
            let editor = build_editor(state);
            row![
                container(list).width(Length::Fixed(340.0)).height(Length::Fill),
                container(editor).width(Length::Fill).height(Length::Fill),
            ]
            .spacing(16)
            .align_y(Alignment::Start)
            .into()
        }
        WorkflowManagerTab::StageActions => build_stage_actions_view(app),
        WorkflowManagerTab::CapabilityManagement => {
            capabilities::build_capability_management_view(app)
        }
    };

    let help_panel = match state.active_tab {
        WorkflowManagerTab::LifecycleDefinition if state.show_status_role_help => {
            status_role_help_panel()
        }
        WorkflowManagerTab::StageActions if state.show_stage_action_help => {
            stage_action_help_panel()
        }
        WorkflowManagerTab::CapabilityManagement if state.show_stage_action_help => {
            stage_action_help_panel()
        }
        _ => Space::new().height(Length::Shrink).into(),
    };

    let help_button = match state.active_tab {
        WorkflowManagerTab::LifecycleDefinition => button(text("语义角色帮助").size(13))
            .padding([9, 14])
            .style(rounded_action_btn_style)
            .on_press(Message::WorkflowManager(WorkflowManagerMessage::ToggleStatusRoleHelp)),
        WorkflowManagerTab::StageActions => button(text("阶段能力帮助").size(13))
            .padding([9, 14])
            .style(rounded_action_btn_style)
            .on_press(Message::WorkflowManager(WorkflowManagerMessage::ToggleStageActionHelp)),
        WorkflowManagerTab::CapabilityManagement => button(text("能力帮助").size(13))
            .padding([9, 14])
            .style(rounded_action_btn_style)
            .on_press(Message::WorkflowManager(WorkflowManagerMessage::ToggleStageActionHelp)),
    };

    let header_actions = row![
        button(text("刷新").size(13))
            .padding([9, 14])
            .style(rounded_action_btn_style)
            .on_press(Message::WorkflowManager(WorkflowManagerMessage::RefreshLifecycles)),
        help_button,
        button(text("新建").size(13))
            .padding([9, 14])
            .style(primary_action_btn_style)
            .on_press(Message::WorkflowManager(WorkflowManagerMessage::StartNew)),
    ]
    .spacing(12)
    .align_y(Alignment::Center);

    let content = column![
        settings_panel(
            column![
                row![
                    column![
                        text("任务编排").size(20),
                        text("生命周期定义负责状态与流向；阶段能力负责项目阶段执行动作。")
                            .size(12)
                            .style(settings_muted_text_style),
                    ]
                    .spacing(4),
                    Space::new().width(Length::Fill),
                    header_actions,
                ]
                .spacing(16)
                .align_y(Alignment::Center),
                workflow_manager_tabs(state.active_tab),
            ]
            .spacing(12)
        ),
        help_panel,
        body,
    ]
    .spacing(16)
    .padding([18, 24])
    .width(Length::Fill)
    .height(Length::Fill);

    let page: Element<'_, Message> = container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .style(|theme: &Theme| {
            let palette = theme.extended_palette();
            iced::widget::container::Style {
                background: Some(palette.background.base.color.into()),
                ..Default::default()
            }
        })
        .into();

    if let Some(confirmation) = &state.pending_confirmation {
        iced::widget::stack![page, workflow_confirmation_dialog(confirmation)].into()
    } else {
        page
    }
}

fn workflow_confirmation_dialog(
    confirmation: &WorkflowManagerConfirmation,
) -> Element<'static, Message> {
    let (title, body, confirm_label) = match confirmation {
        WorkflowManagerConfirmation::ResetWorkflow { workflow_label, .. } => (
            "确认重置生命周期".to_string(),
            format!(
                "将把内置生命周期「{workflow_label}」恢复到默认值，当前保存的自定义修改会被丢弃。"
            ),
            "确认重置",
        ),
        WorkflowManagerConfirmation::DeleteWorkflow { workflow_label, .. } => (
            "确认删除生命周期".to_string(),
            format!("将删除生命周期「{workflow_label}」。此操作不可撤销。"),
            "确认删除",
        ),
        WorkflowManagerConfirmation::DeleteStatus { key, label, .. } => {
            let display = if label.trim().is_empty() { key.as_str() } else { label.as_str() };
            (
                "确认删除阶段".to_string(),
                format!("将从当前草稿中删除阶段「{display}」({key})。保存前可以取消编辑撤销。"),
                "确认删除",
            )
        }
    };
    confirm_dialog(
        title,
        body,
        confirm_label,
        "取消",
        Message::WorkflowManager(WorkflowManagerMessage::ConfirmPendingAction),
        Message::WorkflowManager(WorkflowManagerMessage::CancelPendingConfirmation),
    )
}

fn workflow_manager_tabs(active_tab: WorkflowManagerTab) -> Element<'static, Message> {
    row![
        workflow_manager_tab_button(
            "生命周期定义",
            WorkflowManagerTab::LifecycleDefinition,
            active_tab
        ),
        workflow_manager_tab_button("阶段能力", WorkflowManagerTab::StageActions, active_tab),
        workflow_manager_tab_button(
            "能力管理",
            WorkflowManagerTab::CapabilityManagement,
            active_tab
        ),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .into()
}

fn workflow_manager_tab_button(
    label: &'static str,
    tab: WorkflowManagerTab,
    active_tab: WorkflowManagerTab,
) -> Element<'static, Message> {
    let selected = tab == active_tab;
    let style: fn(&Theme, button::Status) -> button::Style =
        if selected { primary_action_btn_style } else { rounded_action_btn_style };
    button(text(label).size(13))
        .padding([8, 12])
        .style(style)
        .on_press(Message::WorkflowManager(WorkflowManagerMessage::SelectTab(tab)))
        .into()
}

fn build_workflow_list(state: &WorkflowManagerState) -> Element<'_, Message> {
    let mut items = column![].spacing(8);
    for workflow in &state.workflows {
        let selected = state.selected_workflow_id.as_deref() == Some(workflow.id.as_str());
        let badge = if workflow.is_builtin { "内置" } else { "用户" };
        let label = format!("{} ({})", workflow.label, workflow.id);
        let button_style: fn(&Theme, button::Status) -> button::Style =
            if selected { primary_action_btn_style } else { rounded_action_btn_style };
        items = items.push(
            row![
                button(
                    row![
                        column![
                            text(label).size(13),
                            text(badge).size(11).style(settings_muted_text_style),
                        ]
                        .spacing(3),
                        Space::new().width(Length::Fill),
                    ]
                    .align_y(Alignment::Center),
                )
                .width(Length::Fill)
                .padding([10, 12])
                .style(button_style)
                .on_press(Message::WorkflowManager(
                    WorkflowManagerMessage::SelectWorkflow(workflow.id.clone())
                )),
                button(text("查看").size(12))
                    .padding([10, 12])
                    .style(rounded_action_btn_style)
                    .on_press(Message::WorkflowManager(WorkflowManagerMessage::ViewWorkflow(
                        workflow.id.clone()
                    ))),
                button(text("复制").size(12))
                    .padding([10, 12])
                    .style(rounded_action_btn_style)
                    .on_press(Message::WorkflowManager(WorkflowManagerMessage::CopyWorkflow(
                        workflow.id.clone()
                    ))),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        );
    }

    settings_panel(
        column![
            text("预设").size(15),
            scrollable(
                container(items)
                    .width(Length::Fill)
                    .padding(iced::Padding::default().right(WORKFLOW_SCROLL_RIGHT_GUTTER))
            )
            .height(Length::Fill)
            .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4))),
        ]
        .spacing(12),
    )
    .height(Length::Fill)
    .into()
}

fn build_editor(state: &WorkflowManagerState) -> Element<'_, Message> {
    if let Some(draft) = &state.draft {
        return build_draft_editor(draft);
    }

    let selected = state.selected_item();
    let title = selected.map(|workflow| workflow.label.as_str()).unwrap_or("请选择工作流");
    let id = selected.map(|workflow| workflow.id.as_str()).unwrap_or("-");
    let is_builtin = selected.is_some_and(|workflow| workflow.is_builtin);
    let selected_workflow = selected
        .and_then(|item| all_task_workflows().into_iter().find(|workflow| workflow.id == item.id));

    let mut actions = row![].spacing(10).align_y(Alignment::Center);
    if let Some(selected) = selected {
        actions = actions.push(
            button(text("复制").size(13))
                .padding([9, 14])
                .style(rounded_action_btn_style)
                .on_press(Message::WorkflowManager(WorkflowManagerMessage::CopyWorkflow(
                    selected.id.clone(),
                ))),
        );
    }
    if selected.is_some() {
        actions = actions.push(
            button(text("编辑").size(13))
                .padding([9, 14])
                .style(rounded_action_btn_style)
                .on_press(Message::WorkflowManager(WorkflowManagerMessage::StartEditSelected)),
        );
        if is_builtin {
            actions = actions.push(
                button(text("重置").size(13))
                    .padding([9, 14])
                    .style(danger_action_btn_style)
                    .on_press(Message::WorkflowManager(WorkflowManagerMessage::ResetSelected)),
            );
        } else {
            actions = actions.push(
                button(text("删除").size(13))
                    .padding([9, 14])
                    .style(danger_action_btn_style)
                    .on_press(Message::WorkflowManager(WorkflowManagerMessage::DeleteSelected)),
            );
        }
    }

    let header = settings_panel(
        column![
            row![
                column![text(title).size(18), text(id).size(12).style(settings_muted_text_style)]
                    .spacing(4),
                Space::new().width(Length::Fill),
                actions,
            ]
            .align_y(Alignment::Center)
            .spacing(10),
            selected
                .map(|workflow| {
                    if workflow.is_builtin {
                        settings_value_badge("内置生命周期，可编辑，可重置")
                    } else {
                        settings_value_badge("用户预设，可编辑")
                    }
                })
                .unwrap_or_else(|| text("未选择").style(settings_muted_text_style).into()),
        ]
        .spacing(16),
    );

    let body = selected_workflow.map(build_workflow_view).unwrap_or_else(|| {
        settings_panel(text("请选择左侧工作流查看流程").style(settings_muted_text_style)).into()
    });

    column![header, body].spacing(14).height(Length::Fill).into()
}

fn build_workflow_view(workflow: TaskWorkflow) -> Element<'static, Message> {
    let mut flow = column![].spacing(10);
    for (index, status) in workflow.statuses.iter().enumerate() {
        let routes = workflow
            .status_transitions(status.status)
            .into_iter()
            .filter_map(|transition| {
                let label = transition.display_label();
                workflow.status_definition(transition.target).map(|target| WorkflowRouteView {
                    outcome: transition.outcome,
                    label,
                    target_label: target.label.clone(),
                    target_key: target.key.clone(),
                })
            })
            .collect();
        flow = flow.push(build_workflow_status_view(
            index,
            status.label.clone(),
            status.key.clone(),
            status.role,
            status.enabled,
            routes,
        ));
    }

    settings_panel(
        column![
            row![
                column![
                    text("流程预览").size(16),
                    text("每个状态可按结果 key 进入不同状态；succeed/failed 为内置结果。")
                        .size(12)
                        .style(settings_muted_text_style),
                ]
                .spacing(4),
                Space::new().width(Length::Fill),
                settings_value_badge(format!("{} 个状态", workflow.statuses.len())),
            ]
            .align_y(Alignment::Center),
            scrollable(
                container(flow)
                    .width(Length::Fill)
                    .padding(iced::Padding::default().right(WORKFLOW_SCROLL_RIGHT_GUTTER))
            )
            .height(Length::Fill)
            .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4))),
        ]
        .spacing(14),
    )
    .height(Length::Fill)
    .into()
}

#[cfg(test)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct StageActionRow {
    status: TaskStatus,
    capability_id: Option<String>,
    capability_label: String,
    capability_options: Vec<StageActionCapabilityOption>,
    workflow_summary: Option<String>,
}

#[cfg(test)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct StageActionCapabilityOption {
    capability_id: Option<String>,
    label: String,
}

#[cfg(test)]
impl fmt::Display for StageActionCapabilityOption {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.label)
    }
}

#[cfg(test)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct StageWorkflowOption {
    uuid: Option<String>,
    label: String,
}

#[cfg(test)]
impl fmt::Display for StageWorkflowOption {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.label)
    }
}

fn build_stage_actions_view(app: &App) -> Element<'_, Message> {
    let workflow_id = workflow_manager_stage_action_workflow_id(app);
    let workflow_list = build_workflow_list(&app.workflow_manager);
    let orchestration = if !stage_actions_view_ready(app, &workflow_id) {
        let detail = stage_actions_loading_detail(app, &workflow_id);
        settings_panel(
            column![
                text("正在加载阶段能力").size(16),
                text(detail).size(12).style(settings_muted_text_style),
            ]
            .spacing(8),
        )
        .height(Length::Fill)
        .into()
    } else {
        match app.task_board_staff_config.clone().retarget_for_workflow_id(&workflow_id) {
            Ok(config) => {
                let save_message = Some(Message::TaskBoard(
                    TaskBoardMessage::SaveStaffStageAssignmentForWorkflow {
                        workflow_id: workflow_id.clone(),
                    },
                ));
                crate::app::views::task_board::staff_mode::assignment::build_embedded_orchestration_panel(
                app,
                &config,
                &workflow_id,
                Length::Fill,
                Length::Fill,
                save_message,
            )
            }
            Err(error) => settings_panel(
                column![
                    text("无法加载阶段编排").size(16),
                    text(error).size(12).style(settings_muted_text_style),
                ]
                .spacing(8),
            )
            .height(Length::Fill)
            .into(),
        }
    };

    row![
        container(workflow_list).width(Length::Fixed(340.0)).height(Length::Fill),
        container(orchestration).width(Length::Fill).height(Length::Fill),
    ]
    .spacing(16)
    .align_y(Alignment::Start)
    .into()
}

fn stage_actions_loading_detail<'a>(app: &'a App, workflow_id: &str) -> &'a str {
    let config_error = (app.task_board_staff_config_workflow_id.as_deref() == Some(workflow_id))
        .then(|| app.task_board_staff_config_load_error.as_deref())
        .flatten();
    let catalog_error = (app.task_board_staff_capability_catalog_workflow_id.as_deref()
        == Some(workflow_id))
    .then(|| app.task_board_staff_capability_catalog_error.as_deref())
    .flatten();

    config_error.or(catalog_error).unwrap_or("正在加载当前工作流的阶段编排与能力目录")
}

fn stage_actions_view_ready(app: &App, workflow_id: &str) -> bool {
    app.task_board_staff_config_workflow_id.as_deref() == Some(workflow_id)
        && app.task_board_staff_capability_catalog_workflow_id.as_deref() == Some(workflow_id)
        && !app.task_board_staff_capability_catalog.is_empty()
}

fn workflow_manager_stage_action_workflow_id(app: &App) -> String {
    app.workflow_manager
        .selected_workflow_id()
        .map(ToString::to_string)
        .unwrap_or_else(|| app.task_board_settings.workflow_id.clone())
}

#[cfg(test)]
fn stage_action_rows(
    workflow_id: &str,
    stage_actions: &[TaskBoardStageAction],
    workflow_options: &[WorkflowRecordSummary],
    _workflow_error: Option<&str>,
) -> Vec<StageActionRow> {
    let Some(workflow) = task_workflow_by_id(workflow_id) else {
        return Vec::new();
    };

    workflow
        .statuses
        .iter()
        .map(|definition| {
            let action = stage_action_for_status(stage_actions, definition.status);
            let capability_id = action
                .map(|action| action.capability_id.trim())
                .filter(|capability_id| !capability_id.is_empty())
                .map(ToString::to_string);
            let mut capability_options = stage_action_capability_options(definition.role);
            ensure_selected_capability_option(&mut capability_options, capability_id.as_deref());
            let workflow_uuid = action.and_then(|action| {
                action
                    .workflow_uuid
                    .as_deref()
                    .map(str::trim)
                    .filter(|uuid| !uuid.is_empty())
                    .map(ToString::to_string)
            });
            let workflow_options =
                stage_workflow_options(workflow_options, workflow_uuid.as_deref());
            let workflow_summary = if capability_id.as_deref() == Some(SAVED_WORKFLOW_CAPABILITY_ID)
            {
                stage_workflow_summary(&workflow_options, workflow_uuid.as_deref())
            } else {
                None
            };
            StageActionRow {
                status: definition.status,
                capability_label: stage_action_selected_label(capability_id.as_deref()),
                capability_id,
                capability_options,
                workflow_summary,
            }
        })
        .collect()
}

#[cfg(test)]
fn stage_action_for_status(
    actions: &[TaskBoardStageAction],
    status: TaskStatus,
) -> Option<&TaskBoardStageAction> {
    actions.iter().find(|action| action.status == status && !action.capability_id.trim().is_empty())
}

#[cfg(test)]
fn stage_action_capability_options(role: StatusRole) -> Vec<StageActionCapabilityOption> {
    let mut options =
        vec![StageActionCapabilityOption { capability_id: None, label: "未配置".to_string() }];
    options.extend(
        stage_capability_registry()
            .all()
            .iter()
            .filter(|capability| capability.supported_roles.contains(&role))
            .map(|capability| StageActionCapabilityOption {
                capability_id: Some(capability.stable_id.to_string()),
                label: stage_action_capability_label(capability),
            }),
    );
    options
}

#[cfg(test)]
fn ensure_selected_capability_option(
    options: &mut Vec<StageActionCapabilityOption>,
    capability_id: Option<&str>,
) {
    let Some(capability_id) = capability_id else {
        return;
    };
    if options.iter().any(|option| option.capability_id.as_deref() == Some(capability_id)) {
        return;
    }
    options.push(StageActionCapabilityOption {
        capability_id: Some(capability_id.to_string()),
        label: stage_action_selected_label(Some(capability_id)),
    });
}

fn stage_action_capability_label(capability: &StageCapability) -> String {
    format!("{} ({})", capability.label, capability.stable_id)
}

#[cfg(test)]
fn stage_action_capability_select_message(
    status: TaskStatus,
    capability_id: Option<String>,
) -> Message {
    Message::TaskBoard(TaskBoardMessage::SelectStaffStageActionCapability { status, capability_id })
}

#[cfg(test)]
fn stage_action_selected_label(capability_id: Option<&str>) -> String {
    let Some(capability_id) = capability_id else {
        return "未配置".to_string();
    };
    selected_stage_capability(capability_id)
        .map(stage_action_capability_label)
        .unwrap_or_else(|| format!("不支持的阶段能力 ({capability_id})"))
}

#[cfg(test)]
fn selected_stage_capability(capability_id: &str) -> Option<&'static StageCapability> {
    stage_capability_registry()
        .all()
        .iter()
        .find(|capability| capability.stable_id == capability_id)
}

#[cfg(test)]
fn stage_workflow_options(
    workflow_options: &[WorkflowRecordSummary],
    selected_workflow_uuid: Option<&str>,
) -> Vec<StageWorkflowOption> {
    let mut options =
        vec![StageWorkflowOption { uuid: None, label: "未选择 Workflow".to_string() }];
    options.extend(workflow_options.iter().map(|record| StageWorkflowOption {
        uuid: Some(record.uuid.clone()),
        label: stage_workflow_option_label(record),
    }));

    if let Some(selected_workflow_uuid) = selected_workflow_uuid
        && !options.iter().any(|option| option.uuid.as_deref() == Some(selected_workflow_uuid))
    {
        options.push(StageWorkflowOption {
            uuid: Some(selected_workflow_uuid.to_string()),
            label: format!("已绑定 {selected_workflow_uuid}"),
        });
    }

    options
}

#[cfg(test)]
fn stage_workflow_option_label(record: &WorkflowRecordSummary) -> String {
    let name = record.name.trim();
    if name.is_empty() { record.uuid.clone() } else { format!("{name} · {}", record.uuid) }
}

#[cfg(test)]
fn stage_workflow_selected_option(
    options: &[StageWorkflowOption],
    workflow_uuid: Option<&str>,
) -> Option<StageWorkflowOption> {
    options
        .iter()
        .find(|option| option.uuid.as_deref() == workflow_uuid)
        .cloned()
        .or_else(|| options.first().cloned())
}

#[cfg(test)]
fn stage_workflow_summary(
    options: &[StageWorkflowOption],
    workflow_uuid: Option<&str>,
) -> Option<String> {
    if workflow_uuid.is_some() {
        stage_workflow_selected_option(options, workflow_uuid).map(|option| option.label)
    } else {
        Some("未选择 Workflow UUID".to_string())
    }
}

#[derive(Debug, Clone)]
struct WorkflowRouteView {
    outcome: StatusOutcome,
    label: String,
    target_label: String,
    target_key: String,
}

fn build_workflow_status_view(
    index: usize,
    label: String,
    key: String,
    role: StatusRole,
    enabled: bool,
    routes: Vec<WorkflowRouteView>,
) -> Element<'static, Message> {
    let is_terminal = routes.is_empty();
    let route_summary: Element<'static, Message> = if is_terminal {
        text("结束点：无后续状态").size(12).style(settings_muted_text_style).into()
    } else {
        row(routes.into_iter().map(route_badge).collect::<Vec<_>>()).spacing(6).into()
    };
    let role = status_role_display(role);

    container(
        row![
            container(text(format!("{:02}", index + 1)).size(13))
                .center_x(Length::Fixed(WORKFLOW_STATUS_INDEX_WIDTH)),
            column![
                row![
                    text(label).size(15),
                    settings_value_badge(role),
                    if enabled {
                        Space::new().width(Length::Shrink).into()
                    } else {
                        settings_value_badge("已停用")
                    },
                    if is_terminal {
                        settings_value_badge("结束")
                    } else {
                        settings_value_badge("分支")
                    },
                ]
                .spacing(8)
                .align_y(Alignment::Center),
                text(key).size(11).style(settings_muted_text_style),
                route_summary,
            ]
            .spacing(5)
            .width(Length::Fill),
        ]
        .spacing(12)
        .align_y(Alignment::Center),
    )
    .padding([12, 14])
    .style(|theme: &Theme| {
        let palette = theme.extended_palette();
        iced::widget::container::Style {
            background: Some(palette.background.weak.color.scale_alpha(0.62).into()),
            border: iced::Border {
                color: palette.background.strong.color.scale_alpha(0.28),
                width: 1.0,
                radius: 8.0.into(),
            },
            ..Default::default()
        }
    })
    .into()
}

fn route_badge(route: WorkflowRouteView) -> Element<'static, Message> {
    settings_value_badge(format!(
        "{} ({}) -> {} ({})",
        route.label,
        route.outcome.key(),
        route.target_label,
        route.target_key
    ))
}

fn role_label(role: StatusRole) -> &'static str {
    match role {
        StatusRole::Backlog => "待办",
        StatusRole::Queued => "排队",
        StatusRole::Planning => "准备",
        StatusRole::Executing => "处理中",
        StatusRole::Failed => "失败分支",
        StatusRole::Paused => "暂停",
        StatusRole::ReviewReady => "待审核",
        StatusRole::Review => "审核",
        StatusRole::Merging => "收尾",
        StatusRole::Done => "成功",
        StatusRole::Archived => "归档",
        StatusRole::Custom => "其他",
    }
}

fn role_value(role: StatusRole) -> &'static str {
    match role {
        StatusRole::Backlog => "Backlog",
        StatusRole::Queued => "Queued",
        StatusRole::Planning => "Planning",
        StatusRole::Executing => "Executing",
        StatusRole::Failed => "Failed",
        StatusRole::Paused => "Paused",
        StatusRole::ReviewReady => "ReviewReady",
        StatusRole::Review => "Review",
        StatusRole::Merging => "Merging",
        StatusRole::Done => "Done",
        StatusRole::Archived => "Archived",
        StatusRole::Custom => "Custom",
    }
}

fn status_role_display(role: StatusRole) -> String {
    format!("{} ({})", role_label(role), role_value(role))
}

fn status_role_help_panel() -> Element<'static, Message> {
    let mut rows = column![].spacing(8);
    for definition in status_role_help_definitions() {
        rows = rows.push(
            row![
                container(text(definition.value).size(12)).width(Length::Fixed(110.0)),
                container(text(definition.label).size(12)).width(Length::Fixed(78.0)),
                text(definition.description)
                    .size(12)
                    .style(settings_muted_text_style)
                    .width(Length::Fill),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        );
    }

    settings_panel(
        column![
            row![
                column![
                    text("语义角色如何决定调度").size(15),
                    text("显示用中文，保存值为英文；调度按英文角色值判断阶段行为。")
                        .size(12)
                        .style(settings_muted_text_style),
                ]
                .spacing(4),
                Space::new().width(Length::Fill),
                button(text("收起").size(12))
                    .padding([7, 10])
                    .style(rounded_action_btn_style)
                    .on_press(Message::WorkflowManager(
                        WorkflowManagerMessage::ToggleStatusRoleHelp
                    )),
            ]
            .align_y(Alignment::Center),
            rows,
        ]
        .spacing(12),
    )
    .into()
}

fn stage_action_help_panel() -> Element<'static, Message> {
    let mut rows = column![].spacing(8);
    for definition in stage_action_help_definitions() {
        rows = rows.push(
            row![
                container(text(definition.title).size(12)).width(Length::Fixed(190.0)),
                text(definition.description)
                    .size(12)
                    .style(settings_muted_text_style)
                    .width(Length::Fill),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        );
    }

    settings_panel(
        column![
            row![
                column![
                    text("阶段能力如何执行").size(15),
                    text("生命周期定义描述状态和结果边，阶段能力描述项目阶段实际运行的动作。")
                        .size(12)
                        .style(settings_muted_text_style),
                ]
                .spacing(4),
                Space::new().width(Length::Fill),
                button(text("收起").size(12))
                    .padding([7, 10])
                    .style(rounded_action_btn_style)
                    .on_press(Message::WorkflowManager(
                        WorkflowManagerMessage::ToggleStageActionHelp
                    )),
            ]
            .align_y(Alignment::Center),
            rows,
        ]
        .spacing(12),
    )
    .into()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct StatusRoleHelpDefinition {
    value: &'static str,
    label: &'static str,
    description: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct StageActionHelpDefinition {
    title: String,
    description: String,
}

fn status_role_help_definitions() -> Vec<StatusRoleHelpDefinition> {
    StatusRoleOption::all()
        .into_iter()
        .map(|role| {
            let role = role.0;
            StatusRoleHelpDefinition {
                value: role_value(role),
                label: role_label(role),
                description: role_scheduler_description(role),
            }
        })
        .collect()
}

fn role_scheduler_description(role: StatusRole) -> &'static str {
    match role {
        StatusRole::Backlog => "积压池；可被自动提升到下一阶段，不直接占用执行并发。",
        StatusRole::Queued => "待启动；员工开始或调度器可把任务交给执行路径。",
        StatusRole::Planning => "规划中；可启动拆分/准备任务，并计入执行并发。",
        StatusRole::Executing => "执行中；占用执行并发，并参与运行超时监控。",
        StatusRole::Failed => "失败分支；可重试，默认回到排队或规划阶段。",
        StatusRole::Paused => "暂停；不参与自动调度，等待人工处理或恢复。",
        StatusRole::ReviewReady => "待审核；可进入审核，若绑定阶段 Workflow 则优先运行绑定流程。",
        StatusRole::Review => "审核中；成功进入下一阶段，失败走 failed 边或暂停。",
        StatusRole::Merging => "收尾/合并；可执行合并或绑定收尾 Workflow。",
        StatusRole::Done => "完成；不再调度，可进入归档。",
        StatusRole::Archived => "归档；终态，不再调度。",
        StatusRole::Custom => "展示或自定义阶段；系统不施加自动调度语义。",
    }
}

fn stage_action_help_definitions() -> Vec<StageActionHelpDefinition> {
    let mut definitions = vec![
        StageActionHelpDefinition {
            title: "调度关系".to_string(),
            description:
                "StatusRole 决定能不能调度，阶段能力决定具体执行；项目 stage_actions 只在对应阶段被调度后生效。"
                    .to_string(),
        },
        StageActionHelpDefinition {
            title: "入参契约".to_string(),
            description:
                "阶段能力只接收必要上下文，例如任务摘要、阶段 key、成员、优先级和显式 Workflow UUID。"
                    .to_string(),
        },
        StageActionHelpDefinition {
            title: "结果契约".to_string(),
            description:
                "能力产出的结果映射到生命周期边；内置 succeed/failed 结果保持稳定，自定义结果由 workflow 定义。"
                    .to_string(),
        },
        StageActionHelpDefinition {
            title: "安全边界".to_string(),
            description:
                "内置代码能力是标准能力，已保存 Workflow 是业务自定义能力；两者都只传递最小安全上下文，不记录密钥或原始令牌。"
                    .to_string(),
        },
    ];
    definitions.extend(stage_capability_registry().all().iter().map(|capability| {
        StageActionHelpDefinition {
            title: stage_action_capability_label(capability),
            description: format!(
                "入参：{}；结果：{}；权限：{}",
                capability.input_contract, capability.result_semantics, capability.permission_hint
            ),
        }
    }));
    definitions
}

fn build_draft_editor(draft: &WorkflowDraft) -> Element<'_, Message> {
    let mut status_rows = column![].spacing(10);
    for (index, status) in draft.statuses.iter().enumerate() {
        status_rows = status_rows.push(build_status_row(index, status, draft));
    }

    let form = column![
        row![
            column![
                text("编辑工作流").size(18),
                text("为每个状态配置结果 key 与去向；succeed/failed/retry 为内置结果。")
                    .size(12)
                    .style(settings_muted_text_style),
            ]
            .spacing(4),
            Space::new().width(Length::Fill),
            button(text("取消").size(13))
                .padding([9, 14])
                .style(rounded_action_btn_style)
                .on_press(Message::WorkflowManager(WorkflowManagerMessage::CancelDraft)),
            button(text("保存").size(13))
                .padding([9, 14])
                .style(primary_action_btn_style)
                .on_press(Message::WorkflowManager(WorkflowManagerMessage::SaveDraft)),
        ]
        .align_y(Alignment::Center)
        .spacing(10),
        input_row(
            "ID",
            "小写字母、数字、下划线或连字符。",
            text_input("handoff_flow", &draft.id)
                .on_input(|value| {
                    Message::WorkflowManager(WorkflowManagerMessage::DraftIdChanged(value))
                })
                .padding([10, 12])
                .size(13)
                .style(settings_text_input_style)
                .width(Length::Fill),
        ),
        input_row(
            "名称",
            "显示在项目工作流选择和管理列表中。",
            text_input("Handoff Flow", &draft.label)
                .on_input(|value| {
                    Message::WorkflowManager(WorkflowManagerMessage::DraftLabelChanged(value))
                })
                .padding([10, 12])
                .size(13)
                .style(settings_text_input_style)
                .width(Length::Fill),
        ),
        row![
            text("状态").size(15),
            Space::new().width(Length::Fill),
            button(text("添加状态").size(13))
                .padding([9, 14])
                .style(rounded_action_btn_style)
                .on_press(Message::WorkflowManager(WorkflowManagerMessage::AddBlankStatus)),
        ]
        .align_y(Alignment::Center),
        scrollable(
            container(status_rows)
                .width(Length::Fill)
                .padding(iced::Padding::default().right(WORKFLOW_SCROLL_RIGHT_GUTTER))
        )
        .height(Length::Fill)
        .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4))),
    ]
    .spacing(14);

    settings_panel(form).height(Length::Fill).into()
}

fn input_row<'a>(
    label: &'a str,
    help: &'a str,
    control: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    row![
        column![text(label).size(13), text(help).size(11).style(settings_muted_text_style)]
            .spacing(4)
            .width(Length::Fixed(180.0)),
        control.into(),
    ]
    .spacing(14)
    .align_y(Alignment::Center)
    .into()
}

fn build_status_row<'a>(
    index: usize,
    status: &'a WorkflowDraftStatus,
    draft: &'a WorkflowDraft,
) -> Element<'a, Message> {
    let role_pick =
        pick_list(StatusRoleOption::all(), Some(StatusRoleOption(status.role)), move |role| {
            Message::WorkflowManager(WorkflowManagerMessage::DraftStatusRoleChanged {
                index,
                role: role.0,
            })
        })
        .padding([8, 10])
        .style(settings_pick_list_style)
        .menu_style(settings_pick_list_menu_style);

    let route_options = next_options(draft);
    let default_success_key = draft.statuses.get(index + 1).map(|next| next.key.trim().to_string());
    let mut route_rows = column![].spacing(8);
    for (route_index, route) in status.routes.iter().enumerate() {
        route_rows = route_rows.push(build_route_row(
            index,
            route_index,
            route,
            route_options.clone(),
            status.route_target_key(route_index, default_success_key.clone()),
        ));
    }

    settings_panel(
        column![
            row![
                text(format!("#{}", index + 1)).size(13),
                if status.enabled {
                    Space::new().width(Length::Shrink).into()
                } else {
                    settings_value_badge("已停用")
                },
                Space::new().width(Length::Fill),
                button(text(if status.enabled { "停用" } else { "启用" }).size(12))
                    .padding([7, 10])
                    .style(rounded_action_btn_style)
                    .on_press(Message::WorkflowManager(
                        WorkflowManagerMessage::ToggleDraftStatusEnabled(index)
                    )),
                button(text("上移").size(12))
                    .padding([7, 10])
                    .style(rounded_action_btn_style)
                    .on_press_maybe((index > 0).then_some(Message::WorkflowManager(
                        WorkflowManagerMessage::MoveDraftStatusUp(index),
                    ))),
                button(text("下移").size(12))
                    .padding([7, 10])
                    .style(rounded_action_btn_style)
                    .on_press_maybe((index + 1 < draft.statuses.len()).then_some(
                        Message::WorkflowManager(WorkflowManagerMessage::MoveDraftStatusDown(
                            index
                        )),
                    )),
                button(text("删除").size(12))
                    .padding([7, 10])
                    .style(danger_action_btn_style)
                    .on_press(Message::WorkflowManager(WorkflowManagerMessage::RemoveDraftStatus(
                        index,
                    ))),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
            input_row(
                "状态 key",
                "自定义状态会按 key 持久化。",
                text_input("ready_for_qa", &status.key)
                    .on_input(move |key| {
                        Message::WorkflowManager(WorkflowManagerMessage::DraftStatusKeyChanged {
                            index,
                            key,
                        })
                    })
                    .padding([10, 12])
                    .size(13)
                    .style(settings_text_input_style)
                    .width(Length::Fill),
            ),
            input_row(
                "标签",
                "用于看板列和状态展示。",
                text_input("Ready for QA", &status.label)
                    .on_input(move |label| {
                        Message::WorkflowManager(WorkflowManagerMessage::DraftStatusLabelChanged {
                            index,
                            label,
                        })
                    })
                    .padding([10, 12])
                    .size(13)
                    .style(settings_text_input_style)
                    .width(Length::Fill),
            ),
            input_row("语义角色", "调度和执行只依赖角色语义。", role_pick),
            row![
                column![
                    text("结果去向").size(13),
                    text("结果 key 可自定义；目标留空表示此结果结束。")
                        .size(11)
                        .style(settings_muted_text_style),
                ]
                .spacing(4),
                Space::new().width(Length::Fill),
                button(text("添加结果").size(12))
                    .padding([7, 10])
                    .style(rounded_action_btn_style)
                    .on_press(Message::WorkflowManager(
                        WorkflowManagerMessage::AddDraftStatusRoute(index)
                    )),
            ]
            .align_y(Alignment::Center),
            route_rows,
        ]
        .spacing(12),
    )
    .into()
}

fn next_options(draft: &WorkflowDraft) -> Vec<NextStatusOption> {
    let mut options = vec![NextStatusOption::end()];
    options.extend(draft.statuses.iter().filter_map(|status| {
        let key = status.key.trim();
        if key.is_empty() { None } else { Some(NextStatusOption::status(key, status.label.trim())) }
    }));
    options
}

fn build_route_row<'a>(
    status_index: usize,
    route_index: usize,
    route: &'a WorkflowDraftRoute,
    options: Vec<NextStatusOption>,
    selected_key: Option<String>,
) -> Element<'a, Message> {
    let outcome = route.outcome();
    let name_control: Element<'a, Message> = if route.is_builtin() {
        settings_value_badge(outcome.label())
    } else {
        text_input("平台驳回", &route.label)
            .on_input(move |label| {
                Message::WorkflowManager(WorkflowManagerMessage::DraftStatusRouteLabelChanged {
                    status_index,
                    route_index,
                    label,
                })
            })
            .padding([8, 10])
            .size(12)
            .style(settings_text_input_style)
            .width(Length::Fill)
            .into()
    };
    let key_control: Element<'a, Message> = if route.is_builtin() {
        settings_value_badge(outcome.key().to_string())
    } else {
        text_input("manual_review", &route.outcome_key)
            .on_input(move |outcome_key| {
                Message::WorkflowManager(WorkflowManagerMessage::DraftStatusRouteOutcomeChanged {
                    status_index,
                    route_index,
                    outcome_key,
                })
            })
            .padding([8, 10])
            .size(12)
            .style(settings_text_input_style)
            .width(Length::Fill)
            .into()
    };
    let remove: Element<'a, Message> = if route.is_builtin() {
        Space::new().width(Length::Fixed(70.0)).into()
    } else {
        button(text("删除").size(12))
            .padding([7, 10])
            .style(danger_action_btn_style)
            .on_press(Message::WorkflowManager(WorkflowManagerMessage::RemoveDraftStatusRoute {
                status_index,
                route_index,
            }))
            .into()
    };

    row![
        column![
            text("结果名字").size(12),
            text("给人看的名称").size(11).style(settings_muted_text_style),
        ]
        .spacing(3)
        .width(Length::Fixed(120.0)),
        container(name_control).width(Length::FillPortion(2)),
        column![
            text("结果 key").size(12),
            text("程序匹配用").size(11).style(settings_muted_text_style),
        ]
        .spacing(3)
        .width(Length::Fixed(110.0)),
        container(key_control).width(Length::FillPortion(2)),
        container(route_picker(status_index, route_index, options, selected_key))
            .width(Length::FillPortion(2)),
        remove,
    ]
    .spacing(10)
    .align_y(Alignment::Center)
    .into()
}

fn route_picker<'a>(
    status_index: usize,
    route_index: usize,
    options: Vec<NextStatusOption>,
    selected_key: Option<String>,
) -> Element<'a, Message> {
    let selected = NextStatusOption::selected(&options, selected_key);
    pick_list(options, Some(selected), move |target| {
        Message::WorkflowManager(WorkflowManagerMessage::SetDraftStatusRoute {
            status_index,
            route_index,
            target_key: target.target_key,
        })
    })
    .padding([8, 10])
    .style(settings_pick_list_style)
    .menu_style(settings_pick_list_menu_style)
    .into()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct StatusRoleOption(StatusRole);

impl StatusRoleOption {
    fn all() -> Vec<Self> {
        vec![
            Self(StatusRole::Backlog),
            Self(StatusRole::Queued),
            Self(StatusRole::Planning),
            Self(StatusRole::Executing),
            Self(StatusRole::Failed),
            Self(StatusRole::Paused),
            Self(StatusRole::ReviewReady),
            Self(StatusRole::Review),
            Self(StatusRole::Merging),
            Self(StatusRole::Done),
            Self(StatusRole::Archived),
            Self(StatusRole::Custom),
        ]
    }
}

impl fmt::Display for StatusRoleOption {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&status_role_display(self.0))
    }
}

#[derive(Debug, Clone, Eq)]
struct NextStatusOption {
    target_key: Option<String>,
    label: Option<String>,
}

impl NextStatusOption {
    fn end() -> Self {
        Self { target_key: None, label: None }
    }

    fn status(key: &str, label: &str) -> Self {
        Self {
            target_key: Some(key.to_string()),
            label: (!label.is_empty()).then(|| label.to_string()),
        }
    }

    fn selected(options: &[Self], target_key: Option<String>) -> Self {
        options
            .iter()
            .find(|option| option.target_key == target_key)
            .cloned()
            .unwrap_or(Self { target_key, label: None })
    }
}

impl PartialEq for NextStatusOption {
    fn eq(&self, other: &Self) -> bool {
        self.target_key == other.target_key
    }
}

impl fmt::Display for NextStatusOption {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.target_key.as_deref() {
            Some(key) => match self.label.as_deref() {
                Some(label) if label != key => write!(f, "{label} ({key})"),
                _ => f.write_str(key),
            },
            None => f.write_str("结束点 (End)"),
        }
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod mod_tests;
