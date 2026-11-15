use crate::state::*;
use config_worktree::WorktreeSettings;
use vw_api_project::ProjectDto;
use iced::widget::text_editor;
use vw_api_worktree::{WorktreeDeletionPreflight, DeleteManagedWorktreeResponse, RestoreWorktreeResponse};
#[derive(Debug, Clone)]
pub enum WorktreeSettingsMessage {
    Managed(ManagedMessage),
    Recovery(RecoveryMessage),
    Environment(EnvironmentMessage),
    Refresh,
    ContextChanged,
    Edit,
    Cancel,
    Save,
    TabSelected(WorktreesTab),
    RootChanged(String),
    CountChanged(String),
    FetchChanged(bool),
    CleanupChanged(bool),
    AskChanged(bool),
    Loaded {
        identity: WorktreeSettingsIdentity,
        generation: u64,
        result: Result<(WorktreeSettings, String), String>,
    },
    Saved {
        identity: WorktreeSettingsIdentity,
        generation: u64,
        result: Result<WorktreeSettings, String>,
    },
}


#[derive(Debug, Clone)]
pub enum ManagedMessage {
    Refresh,
    Retry {
        project: String,
        worktree: String,
    },
    Retried {
        identity: WorktreeSettingsIdentity,
        target: (String, String),
        result: Result<(), String>,
    },
    Toggle(String),
    Loaded {
        identity: WorktreeSettingsIdentity,
        generation: u64,
        result: Result<Vec<ManagedWorktreeGroup>, String>,
    },
    Open {
        project: String,
        worktree: String,
    },
    Prepared {
        identity: WorktreeSettingsIdentity,
        navigation: u64,
        target: (String, String),
        result: Result<(ProjectDto, String), String>,
    },
    Preflight {
        project: String,
        worktree: String,
    },
    PreflightLoaded {
        identity: WorktreeSettingsIdentity,
        generation: u64,
        target: (String, String),
        result: Result<WorktreeDeletionPreflight, String>,
    },
    Used {
        identity: WorktreeSettingsIdentity,
        result: Result<(), String>,
    },
}
#[derive(Debug, Clone)]
pub enum RecoveryMessage {
    Refresh,
    Loaded {
        identity: WorktreeSettingsIdentity,
        generation: u64,
        result: Result<Vec<WorktreeRecoveryProject>, String>,
    },
    Cancel,
    Ignored(bool),
    Confirm,
    Deleted {
        identity: WorktreeSettingsIdentity,
        generation: u64,
        target: (String, String),
        result: Result<DeleteManagedWorktreeResponse, String>,
    },
    Restore {
        project: String,
        snapshot: String,
    },
    Restored {
        identity: WorktreeSettingsIdentity,
        generation: u64,
        target: (String, String),
        result: Result<RestoreWorktreeResponse, String>,
    },
}
#[derive(Debug, Clone)]
pub enum EnvironmentMessage {
    Refresh,
    ProjectsLoaded {
        identity: WorktreeSettingsIdentity,
        generation: u64,
        result: Result<Vec<ProjectDto>, String>,
    },
    Add,
    Picked {
        identity: WorktreeSettingsIdentity,
        generation: u64,
        path: Option<String>,
    },
    Added {
        identity: WorktreeSettingsIdentity,
        generation: u64,
        result: Result<ProjectDto, String>,
    },
    Edit(ProjectDto),
    Loaded {
        identity: WorktreeSettingsIdentity,
        generation: u64,
        result: Result<Option<String>, String>,
    },
    Action(text_editor::Action),
    Cancel,
    Save,
    Saved {
        identity: WorktreeSettingsIdentity,
        generation: u64,
        result: Result<(), String>,
    },
}
