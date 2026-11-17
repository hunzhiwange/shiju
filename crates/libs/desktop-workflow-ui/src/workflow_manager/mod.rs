//! 管理任务生命周期工作流的列表、编辑、校验与保存界面。

use std::collections::HashSet;
use std::fmt;

use anyhow::Result;
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{Space, button, container, row, scrollable, text};
use iced::{Alignment, Element, Length, Task, Theme};

use desktop_ui_controls::system_settings_common::{
    danger_action_btn_style, primary_action_btn_style, rounded_action_btn_style,
    settings_muted_text_style, settings_panel, settings_select,
    settings_text_input, settings_value_badge,
};
use desktop_ui_controls::toast::confirm_dialog;
use desktop_task_model::workflow::TaskWorkflowRegistry;
use desktop_task_model::{
    OwnedStatusDefinition, OwnedStatusTransition, OwnedTaskWorkflow, StatusOutcome, StatusRole,
    TaskStatus, TaskWorkflow, all_task_workflows, is_builtin_task_workflow_id,
    update_runtime_task_lifecycle_cache,
};

use vw_api_task_lifecycle::{
    TaskLifecycleStatusDto, TaskLifecycleStatusRoleDto, TaskLifecycleStatusTransitionDto,
    TaskLifecycleWorkflowDto, TaskLifecycleWorkflowUpsertBody,
};

const WORKFLOW_SCROLL_RIGHT_GUTTER: f32 = 12.0;
const WORKFLOW_STATUS_INDEX_WIDTH: f32 = 20.0;

/// 网关生命周期列表请求的统一结果类型。
pub type LifecycleWorkflowListResult = std::result::Result<Vec<TaskLifecycleWorkflowDto>, String>;

#[derive(Debug, Clone)]
/// 工作流管理器的加载、选择、编辑和确认消息。
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
/// 工作流管理器的生命周期、阶段动作与能力页签。
pub enum WorkflowManagerTab {
    #[default]
    LifecycleDefinition,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WorkflowListItem {
    id: String,
    label: String,
    is_builtin: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// 工作流列表、编辑草稿、反馈与待执行副作用的聚合状态。
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
}

impl Default for WorkflowManagerState {
    fn default() -> Self {
        Self::from_runtime_cache(false)
    }
}

impl WorkflowManagerState {
    /// 从当前注册表快照构建管理状态。
    pub fn from_registry(registry: &TaskWorkflowRegistry) -> Self {
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

    pub fn active_tab(&self) -> WorkflowManagerTab {
        self.active_tab
    }

    fn selected_item(&self) -> Option<&WorkflowListItem> {
        let selected = self.selected_workflow_id.as_deref()?;
        self.workflows.iter().find(|workflow| workflow.id == selected)
    }

    pub fn take_pending_effect(&mut self) -> Option<WorkflowManagerEffect> {
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
/// 编辑器中的状态定义及其结果路由草稿。
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
    /// 创建启用的状态草稿，并填入该角色的默认结果路由。
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
/// 工作流保存、删除或重置后的用户反馈。
pub struct WorkflowManagerFeedback {
    kind: WorkflowManagerFeedbackKind,
    message: String,
    pub localized: Option<desktop_i18n::workflow::Feedback>,
}

impl WorkflowManagerFeedback {
    fn error(message: impl Into<String>) -> Self {
        Self { kind: WorkflowManagerFeedbackKind::Error, message: message.into(), localized: None }
    }

    fn localized(
        kind: WorkflowManagerFeedbackKind,
        message: desktop_i18n::workflow::Feedback,
    ) -> Self {
        Self {
            kind,
            message: message.render(desktop_i18n::I18n::new(Default::default())),
            localized: Some(message),
        }
    }

    fn from_error(error: anyhow::Error) -> Self {
        match error.downcast_ref::<desktop_i18n::workflow::Feedback>() {
            Some(message) => Self::localized(WorkflowManagerFeedbackKind::Error, message.clone()),
            None => Self::error(error.to_string()),
        }
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
pub enum WorkflowManagerEffect {
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

/// 纯化更新管理器状态并记录待执行副作用；返回值表示注册表是否改变。
pub fn update(
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
        WorkflowManagerMessage::StartNew => {
            state.active_tab = WorkflowManagerTab::LifecycleDefinition;
            state.draft = Some(WorkflowDraft::empty());
            state.feedback = None;
            false
        }
        WorkflowManagerMessage::StartEditSelected => {
            let Some(selected) = state.selected_item() else {
                state.feedback = Some(WorkflowManagerFeedback::localized(
                    WorkflowManagerFeedbackKind::Error,
                    desktop_i18n::workflow::Feedback::new("请先选择工作流", vec![]),
                ));
                return false;
            };
            match registry.workflow_by_id(&selected.id) {
                Some(workflow) => {
                    state.active_tab = WorkflowManagerTab::LifecycleDefinition;
                    state.draft = Some(WorkflowDraft::from_workflow(workflow));
                    state.feedback = None;
                }
                None => {
                    state.feedback = Some(WorkflowManagerFeedback::localized(
                        WorkflowManagerFeedbackKind::Error,
                        desktop_i18n::workflow::Feedback::new("选中的工作流不存在", vec![]),
                    ));
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

pub fn registry_from_runtime() -> TaskWorkflowRegistry {
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
        state.feedback = Some(WorkflowManagerFeedback::localized(
            WorkflowManagerFeedbackKind::Error,
            desktop_i18n::workflow::Feedback::new("请先选择工作流", vec![]),
        ));
        return false;
    };
    if selected.is_builtin {
        state.feedback = Some(WorkflowManagerFeedback::localized(
            WorkflowManagerFeedbackKind::Error,
            desktop_i18n::workflow::Feedback::new("内置工作流不能删除", vec![]),
        ));
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
        state.feedback = Some(WorkflowManagerFeedback::localized(
            WorkflowManagerFeedbackKind::Error,
            desktop_i18n::workflow::Feedback::new("请先选择工作流", vec![]),
        ));
        return false;
    };
    if !selected.is_builtin {
        state.feedback = Some(WorkflowManagerFeedback::localized(
            WorkflowManagerFeedbackKind::Error,
            desktop_i18n::workflow::Feedback::new("只有内置工作流可以重置", vec![]),
        ));
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
        state.feedback = Some(WorkflowManagerFeedback::localized(
            WorkflowManagerFeedbackKind::Error,
            desktop_i18n::workflow::Feedback::new("要删除的阶段不存在", vec![]),
        ));
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
                state.feedback = Some(WorkflowManagerFeedback::localized(
                    WorkflowManagerFeedbackKind::Error,
                    desktop_i18n::workflow::Feedback::new("没有可编辑草稿", vec![]),
                ));
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
                state.feedback = Some(WorkflowManagerFeedback::localized(
                    WorkflowManagerFeedbackKind::Error,
                    desktop_i18n::workflow::Feedback::new("要删除的阶段不存在", vec![]),
                ));
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
        state.feedback = Some(WorkflowManagerFeedback::localized(
            WorkflowManagerFeedbackKind::Error,
            desktop_i18n::workflow::Feedback::new("要复制的工作流不存在", vec![]),
        ));
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
        state.feedback = Some(WorkflowManagerFeedback::localized(
            WorkflowManagerFeedbackKind::Error,
            desktop_i18n::workflow::Feedback::new("没有正在编辑的工作流", vec![]),
        ));
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
            state.feedback = Some(WorkflowManagerFeedback::from_error(err));
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
        return Err(anyhow::Error::new(desktop_i18n::workflow::Feedback::new(
            "内置工作流 ID 不能修改，请复制后另存",
            vec![],
        )));
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
    // 草稿在进入运行时前集中验证标识唯一性、角色约束和路由目标完整性。
    let id = draft.id.trim();
    if id.is_empty() {
        return Err(anyhow::Error::new(desktop_i18n::workflow::Feedback::new(
            "自定义工作流 id 不能为空",
            vec![],
        )));
    }
    let label = draft.label.trim();
    if label.is_empty() {
        return Err(anyhow::Error::new(desktop_i18n::workflow::Feedback::new(
            "自定义工作流 label 不能为空: %s",
            vec![id.to_string()],
        )));
    }

    let mut statuses = Vec::with_capacity(draft.statuses.len());
    for (index, status) in draft.statuses.iter().enumerate() {
        let key = status.key.trim();
        let task_status = TaskStatus::parse_key(key).ok_or_else(|| {
            anyhow::Error::new(desktop_i18n::workflow::Feedback::new(
                "工作流 %s 状态 key 不能为空",
                vec![id.to_string()],
            ))
        })?;
        let default_success_key =
            draft.statuses.get(index + 1).map(|next| next.key.trim().to_string());
        let mut transitions = Vec::new();
        let mut route_keys = HashSet::new();
        for (route_index, route) in status.routes.iter().enumerate() {
            let outcome = StatusOutcome::new(route.outcome_key.trim());
            if outcome.key().is_empty() {
                return Err(anyhow::Error::new(desktop_i18n::workflow::Feedback::new(
                    "工作流 %s 状态 %s 的结果 key 不能为空",
                    vec![id.to_string(), key.to_string()],
                )));
            }
            if !route_keys.insert(outcome.key().to_string()) {
                return Err(anyhow::Error::new(desktop_i18n::workflow::Feedback::new(
                    "工作流 %s 状态 %s 的结果 key 重复: %s",
                    vec![id.to_string(), key.to_string(), outcome.key().to_string()],
                )));
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
        anyhow::Error::new(desktop_i18n::workflow::Feedback::new(
            "工作流 %s 状态 %s 的 %s 目标为空",
            vec![workflow_id.to_string(), status_key.to_string(), outcome.label().to_string()],
        ))
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
                state.feedback = Some(WorkflowManagerFeedback::from_error(err));
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
                state.feedback = Some(WorkflowManagerFeedback::localized(
                    WorkflowManagerFeedbackKind::Error,
                    desktop_i18n::workflow::Feedback::new(
                        "生命周期列表加载失败：%s",
                        vec![error.to_string()],
                    ),
                ));
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
                    state.feedback = Some(WorkflowManagerFeedback::localized(
                        WorkflowManagerFeedbackKind::Success,
                        desktop_i18n::workflow::Feedback::new("工作流已保存", vec![]),
                    ));
                }
                Err(err) => {
                    state.feedback = Some(WorkflowManagerFeedback::from_error(err));
                }
            }
        }
        Err(error) => {
            state.lifecycles_loading = false;
            state.lifecycles_error = Some(error.clone());
            state.feedback = Some(WorkflowManagerFeedback::localized(
                WorkflowManagerFeedbackKind::Error,
                desktop_i18n::workflow::Feedback::new(
                    "工作流保存失败：%s",
                    vec![error.to_string()],
                ),
            ));
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
                state.feedback = Some(WorkflowManagerFeedback::localized(
                    WorkflowManagerFeedbackKind::Success,
                    desktop_i18n::workflow::Feedback::new("工作流已删除", vec![]),
                ));
            }
            Err(err) => {
                state.feedback = Some(WorkflowManagerFeedback::from_error(err));
            }
        },
        Err(error) => {
            state.lifecycles_loading = false;
            state.lifecycles_error = Some(error.clone());
            state.feedback = Some(WorkflowManagerFeedback::localized(
                WorkflowManagerFeedbackKind::Error,
                desktop_i18n::workflow::Feedback::new(
                    "工作流删除失败：%s",
                    vec![error.to_string()],
                ),
            ));
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
                    state.feedback = Some(WorkflowManagerFeedback::localized(
                        WorkflowManagerFeedbackKind::Success,
                        desktop_i18n::workflow::Feedback::new("工作流已重置", vec![]),
                    ));
                }
                Err(err) => {
                    state.feedback = Some(WorkflowManagerFeedback::from_error(err));
                }
            }
        }
        Err(error) => {
            state.lifecycles_loading = false;
            state.lifecycles_error = Some(error.clone());
            state.feedback = Some(WorkflowManagerFeedback::localized(
                WorkflowManagerFeedbackKind::Error,
                desktop_i18n::workflow::Feedback::new(
                    "工作流重置失败：%s",
                    vec![error.to_string()],
                ),
            ));
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

/// Events emitted by the lifecycle manager and its selectable help widgets.
#[derive(Debug, Clone)]
pub enum Message {
    WorkflowManager(WorkflowManagerMessage),
    CopyCode(String),
    AppendText(String),
    OpenUrl(String),
    None,
}
pub type ClientResult = std::result::Result<gateway_task_lifecycle::GatewayTaskLifecycleClient, String>;
impl desktop_chat_view::message_view::markdown_selection::SelectionMessage for Message {
    fn none() -> Self { Self::None }
    fn copy(text: String) -> Self { Self::CopyCode(text) }
    fn context(effect: desktop_ui_controls::text_editor_context_menu::TextContextEffect) -> Self {
        use desktop_ui_controls::text_editor_context_menu::TextContextEffect;
        match effect {
            TextContextEffect::Copy(text) => Self::CopyCode(text),
            TextContextEffect::Append(text) => Self::AppendText(text),
            TextContextEffect::OpenUrl(url) => Self::OpenUrl(url),
        }
    }
    fn selection(_: String, _: usize, _: usize, _: Option<String>) -> Self { Self::None }
}
fn settings_hover_help_label<'a, 'theme>(
    i18n: desktop_i18n::I18n, theme: impl Into<Option<&'theme Theme>>,
    title: &'a str, label: iced::widget::Text<'a>, description: impl Into<std::borrow::Cow<'a, str>>,
) -> iced::widget::Row<'a, Message> {
    desktop_ui_controls::system_settings_common::help::settings_hover_help_label(label,
        desktop_chat_view::message_view::markdown_selection::selectable_titled_text(
            title, description, theme.into().map(|theme| theme.palette().text), i18n))
}

mod view;
pub use view::view;

mod effects;
pub use effects::{load_lifecycle_workflows_task, workflow_manager_effect_task};
