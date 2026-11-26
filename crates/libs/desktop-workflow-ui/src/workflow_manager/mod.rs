//! 管理任务生命周期工作流的列表、编辑、校验与保存界面。

use std::collections::HashSet;
use std::fmt;

use anyhow::Result;
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{Space, button, container, row, scrollable, text};
use iced::{Alignment, Element, Length, Task, Theme};

use desktop_task_model::workflow::{
    TaskPoolWorkflowSnapshotDto, TaskWorkflowRegistry, default_task_workflow,
    task_workflow_from_snapshot,
};
use desktop_task_model::{
    OwnedStatusDefinition, OwnedTaskWorkflow, StatusRole, TaskStatus, TaskWorkflow,
    all_task_workflows, is_builtin_task_workflow_id, update_runtime_task_lifecycle_cache,
};
use desktop_ui_controls::system_settings_common::{
    danger_action_btn_style, primary_action_btn_style, rounded_action_btn_style,
    settings_muted_text_style, settings_panel, settings_select, settings_text_input,
    settings_value_badge,
};
use desktop_ui_controls::toast::confirm_dialog;

use vw_api_task_lifecycle::{
    TaskLifecycleStatusCategoryDto as StatusCategory, TaskLifecycleStepDto,
    TaskLifecycleStepHandoffDto as StepHandoff, TaskLifecycleWorkflowDto,
    TaskLifecycleWorkflowUpsertBody,
};

const WORKFLOW_SCROLL_RIGHT_GUTTER: f32 = 7.0;

/// 网关生命周期列表请求的统一结果类型。
pub type LifecycleWorkflowListResult = std::result::Result<Vec<TaskLifecycleWorkflowDto>, String>;

#[derive(Debug, Clone)]
/// 工作流管理器的加载、选择、编辑和确认消息。
pub enum WorkflowManagerMessage {
    RefreshLifecycles,
    LifecycleWorkflowsLoaded {
        result: LifecycleWorkflowListResult,
        notify_error: bool,
    },
    LifecycleWorkflowSaved {
        workflow_id: String,
        result: LifecycleWorkflowListResult,
    },
    LifecycleWorkflowDeleted {
        workflow_id: String,
        result: LifecycleWorkflowListResult,
    },
    LifecycleWorkflowReset {
        workflow_id: String,
        result: LifecycleWorkflowListResult,
    },
    SelectWorkflow(String),
    ViewWorkflow(String),
    CopyWorkflow(String),
    SelectTab(WorkflowManagerTab),
    ToggleStatusRoleHelp,
    ViewProjectWorkflow,
    RequestProjectWorkflow {
        path: String,
        workflow_id: String,
    },
    ApplyProjectWorkflow {
        path: String,
        workflow_id: String,
    },
    ProjectWorkflowLoaded {
        path: String,
        generation: u64,
        result: ProjectWorkflowResult,
    },
    ProjectWorkflowApplied {
        path: String,
        generation: u64,
        result: ProjectWorkflowResult,
    },
    ToggleDraftAdvanced,
    SelectCatalogPreset(Option<String>),
    ToggleCatalogAdvanced,
    EditCatalogStatus {
        workflow_id: String,
        status_key: String,
    },
    ShowAllDraftStatuses,
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
    DraftStatusKeyChanged {
        index: usize,
        key: String,
    },
    DraftStatusLabelChanged {
        index: usize,
        label: String,
    },
    DraftStatusCategoryChanged {
        index: usize,
        category: StatusCategory,
    },
    DraftStatusHandoffChanged {
        index: usize,
        handoff: StepHandoff,
    },
    DraftInitialStatusChanged(String),
    HandoffDirectoryLoaded {
        account_id: String,
        agents: Vec<HandoffIdentity>,
        result: Result<Option<HandoffIdentity>, String>,
    },
    ToggleDraftStatusEnabled(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
/// 工作流管理器的生命周期定义与状态库页签。
pub enum WorkflowManagerTab {
    #[default]
    LifecycleDefinition,
    StatusCatalog,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WorkflowListItem {
    definition: TaskWorkflow,
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
    catalog_preset_id: Option<String>,
    show_catalog_advanced: bool,
    project_workflow: Option<ProjectWorkflow>,
    project_generation: u64,
    show_project_workflow: bool,
}

pub type ProjectWorkflowResult = std::result::Result<Option<TaskPoolWorkflowSnapshotDto>, String>;

#[derive(Debug, Clone, PartialEq, Eq)]
struct ProjectWorkflow {
    path: String,
    busy: bool,
    snapshot: Option<TaskPoolWorkflowSnapshotDto>,
    definition: Option<TaskWorkflow>,
    error: Option<String>,
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
                definition: workflow,
            })
            .collect();
        let selected_workflow_id = workflows
            .iter()
            .find(|workflow| !workflow.is_builtin)
            .or_else(|| workflows.first())
            .map(|workflow| workflow.definition.id.clone());
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
            catalog_preset_id: None,
            show_catalog_advanced: false,
            project_workflow: None,
            project_generation: 0,
            show_project_workflow: true,
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
            .filter(|id| self.workflows.iter().any(|workflow| workflow.definition.id == *id))
            .or(refreshed.selected_workflow_id);
        self.catalog_preset_id = self.catalog_preset_id.take().filter(|id| {
            self.workflows.iter().any(|item| item.is_builtin && item.definition.id == *id)
        });
    }

    pub fn workflow_ids(&self) -> Vec<String> {
        self.workflows.iter().map(|workflow| workflow.definition.id.clone()).collect()
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

    pub fn begin_lifecycle_refresh(&mut self) -> bool {
        if self.lifecycles_loading {
            return false;
        }
        self.lifecycles_loading = true;
        self.lifecycles_error = None;
        self.feedback = None;
        true
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

    pub fn clear_handoff_identities(&mut self) {
        if let Some(draft) = &mut self.draft {
            draft.identities.clear();
            draft.identity_error = Some("请刷新以读取当前登录用户".into());
        }
    }

    pub fn dismiss_dialogs(&mut self) {
        self.pending_confirmation = None;
        self.show_status_role_help = false;
    }

    pub fn active_tab(&self) -> WorkflowManagerTab {
        self.active_tab
    }

    fn selected_item(&self) -> Option<&WorkflowListItem> {
        let selected = self.selected_workflow_id.as_deref()?;
        self.workflows.iter().find(|workflow| workflow.definition.id == selected)
    }

    pub fn take_pending_effect(&mut self) -> Option<WorkflowManagerEffect> {
        self.pending_effect.take()
    }

    fn project_workflow(&self, path: Option<&str>) -> Option<&ProjectWorkflow> {
        self.project_workflow.as_ref().filter(|project| Some(project.path.as_str()) == path)
    }

    pub fn begin_project_workflow_load(&mut self, path: &str) -> Option<u64> {
        if self.project_workflow(Some(path)).is_some_and(|project| project.busy) {
            return None;
        }
        if self.project_workflow(Some(path)).is_none() {
            self.show_project_workflow = true;
            if matches!(
                &self.pending_confirmation,
                Some(WorkflowManagerConfirmation::ApplyProjectWorkflow { .. })
            ) {
                self.pending_confirmation = None;
            }
        }
        self.project_generation = self.project_generation.wrapping_add(1);
        self.project_workflow = Some(ProjectWorkflow {
            path: path.to_string(),
            busy: true,
            snapshot: None,
            definition: None,
            error: None,
        });
        Some(self.project_generation)
    }

    pub fn begin_project_workflow_apply(
        &mut self,
        path: &str,
        workflow_id: &str,
    ) -> Option<(u64, Option<TaskPoolWorkflowSnapshotDto>)> {
        if !matches!(&self.pending_confirmation,
            Some(WorkflowManagerConfirmation::ApplyProjectWorkflow { path: pending_path, workflow_id: id, .. })
            if pending_path == path && id == workflow_id)
        {
            return None;
        }
        self.pending_confirmation = None;
        let project = self.project_workflow.as_mut().filter(|project| {
            project.path == path
                && !project.busy
                && project.definition.is_some()
                && project.error.is_none()
        })?;
        project.busy = true;
        self.project_generation = self.project_generation.wrapping_add(1);
        Some((self.project_generation, project.snapshot.clone()))
    }

    pub fn finish_project_workflow(
        &mut self,
        path: &str,
        generation: u64,
        result: ProjectWorkflowResult,
        applied: bool,
    ) -> bool {
        if self.project_generation != generation {
            return false;
        }
        let Some(project) = self.project_workflow.as_mut().filter(|project| project.path == path)
        else {
            return false;
        };
        project.busy = false;
        match result {
            Ok(snapshot) => {
                project.definition = match &snapshot {
                    Some(snapshot) => task_workflow_from_snapshot(snapshot),
                    None => Some(default_task_workflow().into()),
                };
                project.error =
                    project.definition.is_none().then(|| "无法读取项目工作流定义".into());
                project.snapshot = snapshot;
                if applied {
                    self.show_project_workflow = true;
                }
            }
            Err(error) => project.error = Some(error),
        }
        true
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum WorkflowManagerConfirmation {
    ResetWorkflow { workflow_id: String, workflow_label: String },
    DeleteWorkflow { workflow_id: String, workflow_label: String },
    DeleteStatus { index: usize, key: String, label: String },
    ApplyProjectWorkflow { path: String, workflow_id: String, workflow_label: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandoffIdentity {
    pub id: String,
    pub label: String,
    pub is_agent: bool,
}

impl fmt::Display for HandoffIdentity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.label)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WorkflowDraft {
    original_id: Option<String>,
    id: String,
    label: String,
    statuses: Vec<WorkflowDraftStatus>,
    focused_status: Option<usize>,
    show_advanced: bool,
    initial_status_key: String,
    identities: Vec<HandoffIdentity>,
    identity_error: Option<String>,
}

impl WorkflowDraft {
    fn empty() -> Self {
        Self {
            original_id: None,
            id: String::new(),
            label: String::new(),
            statuses: Vec::new(),
            focused_status: None,
            show_advanced: false,
            initial_status_key: String::new(),
            identities: Vec::new(),
            identity_error: None,
        }
    }

    fn from_workflow(workflow: TaskWorkflow) -> Self {
        let mut statuses = Vec::new();
        for step in &workflow.steps {
            if let Some(definition) =
                workflow.statuses.iter().find(|status| status.key == step.status_key)
            {
                let mut status = WorkflowDraftStatus::from_definition(definition.clone());
                status.handoff = step.handoff.clone();
                statuses.push(status);
            }
        }
        for definition in &workflow.statuses {
            if !workflow.steps.iter().any(|step| step.status_key == definition.key) {
                let mut status = WorkflowDraftStatus::from_definition(definition.clone());
                status.in_workflow = false;
                statuses.push(status);
            }
        }
        Self {
            original_id: Some(workflow.id.clone()),
            id: workflow.id,
            label: workflow.label,
            statuses,
            focused_status: None,
            show_advanced: false,
            initial_status_key: workflow.initial_status_key,
            identities: Vec::new(),
            identity_error: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// 编辑器中的状态分类及步骤交接草稿。
pub struct WorkflowDraftStatus {
    key: String,
    label: String,
    role: StatusRole,
    enabled: bool,
    handoff: StepHandoff,
    in_workflow: bool,
}

impl WorkflowDraftStatus {
    /// 创建启用的步骤草稿，默认保留负责人。
    pub fn new(key: &str, label: &str, role: StatusRole) -> Self {
        Self {
            key: key.to_string(),
            label: label.to_string(),
            role,
            enabled: true,
            handoff: StepHandoff::KeepAssignee,
            in_workflow: true,
        }
    }

    fn blank() -> Self {
        Self::new("", "", StatusRole::Custom)
    }

    fn from_definition(definition: OwnedStatusDefinition) -> Self {
        Self {
            key: definition.key,
            label: definition.label,
            role: match definition.role {
                StatusRole::Backlog | StatusRole::Queued => StatusRole::Backlog,
                StatusRole::Done => StatusRole::Done,
                StatusRole::Archived => StatusRole::Archived,
                _ => StatusRole::Custom,
            },
            enabled: definition.enabled,
            handoff: StepHandoff::KeepAssignee,
            in_workflow: true,
        }
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
            if state.begin_lifecycle_refresh() {
                state.pending_effect =
                    Some(WorkflowManagerEffect::LoadLifecycles { notify_error: true });
            }
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
            if state.workflows.iter().any(|workflow| workflow.definition.id == id) {
                state.selected_workflow_id = Some(id);
                state.show_project_workflow = false;
                state.feedback = None;
            }
            false
        }
        WorkflowManagerMessage::ViewWorkflow(id) => {
            if state.workflows.iter().any(|workflow| workflow.definition.id == id) {
                state.selected_workflow_id = Some(id);
                state.show_project_workflow = false;
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
        WorkflowManagerMessage::ViewProjectWorkflow => {
            state.show_project_workflow = true;
            false
        }
        WorkflowManagerMessage::RequestProjectWorkflow { path, workflow_id } => {
            if state.draft.is_none()
                && !state.lifecycles_loading
                && state.project_workflow(Some(&path)).is_some_and(|project| {
                    !project.busy && project.definition.is_some() && project.error.is_none()
                })
                && let Some(item) =
                    state.workflows.iter().find(|item| item.definition.id == workflow_id)
            {
                state.pending_confirmation =
                    Some(WorkflowManagerConfirmation::ApplyProjectWorkflow {
                        path,
                        workflow_id,
                        workflow_label: item.definition.label.clone(),
                    });
                state.feedback = None;
            }
            false
        }
        WorkflowManagerMessage::ApplyProjectWorkflow { .. }
        | WorkflowManagerMessage::ProjectWorkflowLoaded { .. }
        | WorkflowManagerMessage::ProjectWorkflowApplied { .. } => false,
        WorkflowManagerMessage::ToggleDraftAdvanced => {
            if let Some(draft) = &mut state.draft {
                draft.show_advanced = !draft.show_advanced;
            }
            false
        }
        WorkflowManagerMessage::SelectCatalogPreset(id) => {
            state.catalog_preset_id = id.filter(|id| {
                state.workflows.iter().any(|item| item.is_builtin && item.definition.id == *id)
            });
            false
        }
        WorkflowManagerMessage::ToggleCatalogAdvanced => {
            state.show_catalog_advanced = !state.show_catalog_advanced;
            false
        }
        WorkflowManagerMessage::EditCatalogStatus { workflow_id, status_key } => {
            if state.draft.is_none()
                && let Some(item) =
                    state.workflows.iter().find(|item| item.definition.id == workflow_id)
                && let Some(index) =
                    item.definition.statuses.iter().position(|status| status.key == status_key)
            {
                let mut draft = WorkflowDraft::from_workflow(item.definition.clone());
                draft.focused_status = Some(index);
                state.selected_workflow_id = Some(workflow_id);
                state.show_project_workflow = false;
                state.active_tab = WorkflowManagerTab::LifecycleDefinition;
                state.show_status_role_help = false;
                state.draft = Some(draft);
                state.feedback = None;
            }
            false
        }
        WorkflowManagerMessage::ShowAllDraftStatuses => {
            if let Some(draft) = &mut state.draft {
                draft.focused_status = None;
            }
            false
        }
        WorkflowManagerMessage::StartNew => {
            state.active_tab = WorkflowManagerTab::LifecycleDefinition;
            let mut draft = WorkflowDraft::empty();
            draft.show_advanced = true;
            state.draft = Some(draft);
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
            match registry.workflow_by_id(&selected.definition.id) {
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
                draft.focused_status = None;
                draft.show_advanced = true;
            }
            false
        }
        WorkflowManagerMessage::AddDraftStatus(status) => {
            if let Some(draft) = &mut state.draft {
                draft.statuses.push(status);
                draft.focused_status = None;
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
                draft.focused_status = None;
            }
            false
        }
        WorkflowManagerMessage::MoveDraftStatusDown(index) => {
            if let Some(draft) = &mut state.draft
                && index + 1 < draft.statuses.len()
            {
                draft.statuses.swap(index, index + 1);
                draft.focused_status = None;
            }
            false
        }
        WorkflowManagerMessage::DraftStatusKeyChanged { index, key } => {
            if let Some(status) =
                state.draft.as_mut().and_then(|draft| draft.statuses.get_mut(index))
            {
                let old_key = std::mem::replace(&mut status.key, key.clone());
                if let Some(draft) = &mut state.draft {
                    if draft.initial_status_key == old_key {
                        draft.initial_status_key = key.clone();
                    }
                    for status in &mut draft.statuses {
                        if let StepHandoff::Agent {
                            completion_status_key, return_status_key, ..
                        } = &mut status.handoff
                        {
                            for target in [completion_status_key, return_status_key] {
                                if target.as_deref() == Some(old_key.as_str()) {
                                    *target = Some(key.clone());
                                }
                            }
                        }
                    }
                }
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
        WorkflowManagerMessage::DraftInitialStatusChanged(key) => {
            if let Some(draft) = &mut state.draft {
                draft.initial_status_key = key;
            }
            false
        }
        WorkflowManagerMessage::HandoffDirectoryLoaded { agents, result, .. } => {
            if let Some(draft) = &mut state.draft {
                draft.identities = agents;
                match result {
                    Ok(member) => {
                        draft.identities.extend(member);
                        draft.identity_error = None;
                    }
                    Err(_) => {
                        draft.identity_error = Some("无法读取当前识矩登录用户，请刷新重试".into());
                    }
                }
            }
            false
        }
        WorkflowManagerMessage::DraftStatusHandoffChanged { index, handoff } => {
            if let Some(status) =
                state.draft.as_mut().and_then(|draft| draft.statuses.get_mut(index))
            {
                if !matches!(status.role, StatusRole::Done | StatusRole::Archived) {
                    status.handoff = handoff;
                }
            }
            false
        }
        WorkflowManagerMessage::DraftStatusCategoryChanged { index, category } => {
            let role = match category {
                StatusCategory::Unstarted => StatusRole::Backlog,
                StatusCategory::Started => StatusRole::Custom,
                StatusCategory::Done => StatusRole::Done,
                StatusCategory::Closed => StatusRole::Archived,
            };
            if let Some(status) =
                state.draft.as_mut().and_then(|draft| draft.statuses.get_mut(index))
            {
                status.role = role;
                if matches!(role, StatusRole::Done | StatusRole::Archived) {
                    status.handoff = StepHandoff::KeepAssignee;
                }
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
    OwnedTaskWorkflow {
        id: workflow.id,
        label: workflow.label,
        statuses: workflow.statuses,
        initial_status_key: workflow.initial_status_key,
        steps: workflow.steps,
    }
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
    let workflow_id = selected.definition.id.clone();
    let workflow_label = selected.definition.label.clone();

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
    let workflow_id = selected.definition.id.clone();
    let workflow_label = selected.definition.label.clone();

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
        WorkflowManagerConfirmation::ApplyProjectWorkflow { .. } => {}
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
            draft.focused_status = None;
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
    let statuses = draft
        .statuses
        .iter()
        .map(|status| {
            let key = status.key.trim();
            Ok(OwnedStatusDefinition {
                status: TaskStatus::parse_key(key)
                    .ok_or_else(|| anyhow::anyhow!("empty status key"))?,
                key: key.into(),
                label: status.label.trim().into(),
                role: status.role,
                enabled: status.enabled,
                is_terminal: matches!(status.role, StatusRole::Done | StatusRole::Archived),
                auto_execution_candidate: false,
                occupies_execution_slot: false,
                is_scheduling_backlog: false,
                next: None,
                transitions: Vec::new(),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let steps: Vec<_> = draft
        .statuses
        .iter()
        .filter(|status| status.in_workflow)
        .map(|status| TaskLifecycleStepDto {
            status_key: status.key.trim().into(),
            handoff: status.handoff.clone(),
        })
        .collect();
    let initial_status_key = if draft.initial_status_key.is_empty() {
        steps.first().map(|step| step.status_key.clone()).unwrap_or_default()
    } else {
        draft.initial_status_key.clone()
    };
    let workflow = OwnedTaskWorkflow {
        id: draft.id.trim().into(),
        label: draft.label.trim().into(),
        statuses,
        initial_status_key,
        steps,
    };
    lifecycle_workflow_dto_from_owned_workflow(workflow.clone())
        .validate()
        .map_err(|error| anyhow::Error::new(desktop_i18n::workflow::Feedback::new(
            "步骤配置无效：%s", vec![error],
        )))?;
    Ok(workflow)
}

fn lifecycle_workflow_dto_from_owned_workflow(
    workflow: OwnedTaskWorkflow,
) -> TaskLifecycleWorkflowDto {
    desktop_task_model::workflow::lifecycle_dto_from_task_workflow(
        TaskWorkflow::from(workflow),
        false,
    )
}

fn apply_lifecycle_workflows_loaded(
    state: &mut WorkflowManagerState,
    result: LifecycleWorkflowListResult,
    notify_error: bool,
) {
    state.lifecycles_loading = false;
    match result {
        Ok(workflows) => match apply_lifecycle_workflow_cache(state, workflows, None) {
            Ok(()) => {
                state.feedback = None;
            }
            Err(err) if notify_error => {
                state.lifecycles_error = Some(err.to_string());
                state.feedback = Some(WorkflowManagerFeedback::from_error(err));
            }
            Err(err) => {
                state.lifecycles_error = Some(err.to_string());
                state.feedback = None;
            }
        },
        Err(error) => {
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
                    state.show_project_workflow = false;
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
                        state.workflows.first().map(|workflow| workflow.definition.id.clone());
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
        && state.workflows.iter().any(|workflow| workflow.definition.id == selected_workflow_id)
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
pub type ClientResult =
    std::result::Result<gateway_task_lifecycle::GatewayTaskLifecycleClient, String>;
impl desktop_chat_view::message_view::markdown_selection::SelectionMessage for Message {
    fn none() -> Self {
        Self::None
    }
    fn copy(text: String) -> Self {
        Self::CopyCode(text)
    }
    fn context(effect: desktop_ui_controls::text_editor_context_menu::TextContextEffect) -> Self {
        use desktop_ui_controls::text_editor_context_menu::TextContextEffect;
        match effect {
            TextContextEffect::Copy(text) => Self::CopyCode(text),
            TextContextEffect::Append(text) => Self::AppendText(text),
            TextContextEffect::OpenUrl(url) => Self::OpenUrl(url),
        }
    }
    fn selection(_: String, _: usize, _: usize, _: Option<String>) -> Self {
        Self::None
    }
}
fn settings_hover_help_label<'a, 'theme>(
    i18n: desktop_i18n::I18n,
    theme: impl Into<Option<&'theme Theme>>,
    title: &'a str,
    label: iced::widget::Text<'a>,
    description: impl Into<std::borrow::Cow<'a, str>>,
) -> iced::widget::Row<'a, Message> {
    desktop_ui_controls::system_settings_common::help::settings_hover_help_label(
        label,
        desktop_chat_view::message_view::markdown_selection::selectable_titled_text(
            title,
            description,
            theme.into().map(|theme| theme.palette().text),
            i18n,
        ),
    )
}

mod view;
pub use view::{view, view_content, with_dialog};

mod effects;
pub use effects::{load_lifecycle_workflows_task, workflow_manager_effect_task};

mod status_catalog;
