use crate::app::message::settings::worktrees::recovery::{RecoveryMessage, update};
#[test]
fn worktree_cleanup_flow_cancel_does_not_delete() {
    let mut app = crate::app::App::new().0;
    app.worktree_settings.managed.preflight_target = Some(("project".into(), "tree".into()));
    let task = update(&mut app, RecoveryMessage::Cancel);
    assert!(app.worktree_settings.managed.preflight_target.is_none());
    assert_eq!(task.units(), 0);
}

use crate::app::state::{WorktreeRecoveryProject, WorktreeSettingsIdentity};
use vw_gateway_client::vw_api_types::worktree::*;
fn app() -> crate::app::App {
    let mut app = crate::app::App::new().0;
    let identity = WorktreeSettingsIdentity::current(&app);
    let state = &mut app.worktree_settings;
    state.managed.identity = Some(identity.clone());
    state.recovery.identity = Some(identity);
    state.managed.preflight_target = Some(("project".into(), "tree".into()));
    state.managed.preflight = Some(WorktreeDeletionPreflight {
        fingerprint: "current-fingerprint".into(),
        expires_at_ms: u64::MAX,
        confirmation_required: true,
        has_changes: true,
        has_ignored_files: false,
        protection_reasons: vec![],
    });
    app
}
#[test]
fn worktree_cleanup_flow_rejects_stale_confirmation() {
    let mut app = app();
    app.worktree_settings.managed.preflight.as_mut().unwrap().expires_at_ms = 1;
    let _ = update(&mut app, RecoveryMessage::Confirm);
    assert!(app.worktree_settings.recovery.pending.is_none());
    assert!(app.worktree_settings.recovery.error.is_some());
}
#[test]
fn worktree_cleanup_flow_ignored_files_require_separate_acknowledgement() {
    let mut app = app();
    app.worktree_settings.saved = Some(vw_config_types::worktree::WorktreeSettings {
        ask_before_delete: false,
        ..Default::default()
    });
    app.worktree_settings.managed.preflight.as_mut().unwrap().has_ignored_files = true;
    let _ = update(&mut app, RecoveryMessage::Confirm);
    assert!(app.worktree_settings.recovery.pending.is_none());
    let _ = update(&mut app, RecoveryMessage::Ignored(true));
    let task = update(&mut app, RecoveryMessage::Confirm);
    assert_eq!(task.units(), 1);
    assert!(app.worktree_settings.recovery.pending.is_some());
    assert_eq!(update(&mut app, RecoveryMessage::Confirm).units(), 0);
}
#[test]
fn worktree_cleanup_flow_protected_confirmation_never_submits() {
    let mut app = app();
    app.worktree_settings
        .managed
        .preflight
        .as_mut()
        .unwrap()
        .protection_reasons
        .push(WorktreeProtectionReason::Busy);
    let _ = update(&mut app, RecoveryMessage::Confirm);
    assert!(app.worktree_settings.recovery.pending.is_none());
}
#[test]
fn worktree_cleanup_flow_cross_project_response_is_ignored() {
    let mut app = app();
    let identity = WorktreeSettingsIdentity::current(&app);
    app.worktree_settings.recovery.loading = true;
    app.project_path = Some("/different-project".into());
    let _ = update(
        &mut app,
        RecoveryMessage::Loaded { identity, generation: 0, result: Err("old failure".into()) },
    );
    assert!(app.worktree_settings.recovery.error.is_none());
    assert!(!app.worktree_settings.recovery.loaded);
}
#[test]
fn worktree_cleanup_flow_old_epoch_response_is_ignored() {
    let mut app = app();
    let identity = WorktreeSettingsIdentity::current(&app);
    crate::app::message::settings::worktrees::recovery::invalidate(&mut app);
    app.worktree_settings.recovery.identity = Some(identity.clone());
    app.worktree_settings.recovery.loading = true;
    let _ =
        update(&mut app, RecoveryMessage::Loaded { identity, generation: 0, result: Ok(vec![]) });
    assert!(!app.worktree_settings.recovery.loaded);
}
#[test]
fn worktree_cleanup_flow_item_failure_requires_new_preflight() {
    let mut app = app();
    let identity = WorktreeSettingsIdentity::current(&app);
    let _ = update(&mut app, RecoveryMessage::Confirm);
    let _ = update(
        &mut app,
        RecoveryMessage::Deleted {
            identity,
            generation: 1,
            target: ("project".into(), "tree".into()),
            result: Err("protection changed".into()),
        },
    );
    assert!(app.worktree_settings.managed.preflight.is_none());
    assert!(app.worktree_settings.recovery.pending.is_none());
    assert_eq!(app.worktree_settings.recovery.outcomes.len(), 1);
    assert!(app.worktree_settings.recovery.outcomes[0].2.is_err());
    assert_eq!(update(&mut app, RecoveryMessage::Confirm).units(), 0);
}
fn restored() -> RestoreWorktreeResponse {
    serde_json::from_value(serde_json::json!({
        "worktree": {"worktree": {"id":"restored", "project_id":"project", "name":"恢复工作树", "branch":"recovered", "directory":"/恢复/新路径", "status":"ready", "created_at_ms":1, "updated_at_ms":2}, "title":"恢复", "sessions":[], "protection_reasons":[], "recovered_from":"snapshot"},
        "original_worktree_id":"tree", "branch_changed":true
    })).unwrap()
}
#[test]
fn worktree_cleanup_flow_restores_and_refreshes() {
    let mut app = app();
    let identity = WorktreeSettingsIdentity::current(&app);
    app.worktree_settings.recovery.pending = Some(("project".into(), "snapshot".into()));
    let _ = update(
        &mut app,
        RecoveryMessage::Restored {
            identity,
            generation: 0,
            target: ("project".into(), "snapshot".into()),
            result: Ok(restored()),
        },
    );
    assert!(app.worktree_settings.managed.loading);
    assert!(app.worktree_settings.recovery.loading);
    assert_eq!(app.worktree_settings.recovery.restored.as_ref().unwrap().2, "/恢复/新路径");
    assert!(app.current_session_runtime().draft.is_none());
    assert!(!app.new_session_creating);
}
#[test]
fn worktree_cleanup_flow_delete_success_refreshes_and_records_snapshot() {
    let mut app = app();
    let identity = WorktreeSettingsIdentity::current(&app);
    let _ = update(&mut app, RecoveryMessage::Confirm);
    let _ = update(
        &mut app,
        RecoveryMessage::Deleted {
            identity,
            generation: 1,
            target: ("project".into(), "tree".into()),
            result: Ok(DeleteManagedWorktreeResponse { snapshot_id: "persisted".into() }),
        },
    );
    assert!(app.worktree_settings.managed.loading);
    assert!(app.worktree_settings.recovery.loading);
    assert_eq!(app.worktree_settings.recovery.outcomes[0].2.as_ref().unwrap(), "persisted");
}
#[test]
fn worktree_cleanup_flow_load_failure_does_not_report_empty_success() {
    let mut app = app();
    let identity = WorktreeSettingsIdentity::current(&app);
    app.worktree_settings.recovery.loading = true;
    let _ = update(
        &mut app,
        RecoveryMessage::Loaded { identity, generation: 0, result: Err("offline".into()) },
    );
    assert!(!app.worktree_settings.recovery.loaded);
    assert_eq!(app.worktree_settings.recovery.error.as_deref(), Some("offline"));
}
#[test]
fn worktree_cleanup_flow_restore_failure_can_retry_persisted_snapshot() {
    let mut app = app();
    let identity = WorktreeSettingsIdentity::current(&app);
    let project = serde_json::from_value(serde_json::json!({"id":"project","name":"项目","directory":"/project","display_path":"/project","status":"ready","created_at_ms":1,"updated_at_ms":2,"git":{"is_repo":true,"has_uncommitted_changes":false}})).unwrap();
    app.worktree_settings.recovery.projects = vec![WorktreeRecoveryProject {
        project,
        snapshots: Ok(vec![WorktreeSnapshotSummary {
            id: "snapshot".into(),
            original_worktree_id: "tree".into(),
            directory: "/old".into(),
            branch: None,
            has_ignored_files: false,
            restored: false,
        }]),
        proposal: Ok(WorktreeCleanupProposal {
            candidates: vec![],
            protected: vec![],
            remaining_excess: 0,
        }),
    }];
    assert_eq!(
        update(
            &mut app,
            RecoveryMessage::Restore { project: "project".into(), snapshot: "snapshot".into() }
        )
        .units(),
        1
    );
    let _ = update(
        &mut app,
        RecoveryMessage::Restored {
            identity,
            generation: 1,
            target: ("project".into(), "snapshot".into()),
            result: Err("disk unavailable".into()),
        },
    );
    assert!(app.worktree_settings.recovery.error.is_some());
    assert_eq!(
        update(
            &mut app,
            RecoveryMessage::Restore { project: "project".into(), snapshot: "snapshot".into() }
        )
        .units(),
        1
    );
}

#[test]
fn worktree_cleanup_flow_cancel_rejects_late_preflight() {
    let mut app = app();
    let identity = WorktreeSettingsIdentity::current(&app);
    let preflight = app.worktree_settings.managed.preflight.clone().unwrap();
    let _ = update(&mut app, RecoveryMessage::Cancel);
    let _ = crate::app::message::settings::worktrees::update(
        &mut app,
        crate::app::message::settings::worktrees::WorktreeSettingsMessage::Managed(
            crate::app::message::settings::worktrees::ManagedMessage::PreflightLoaded {
                identity,
                generation: 0,
                target: ("project".into(), "tree".into()),
                result: Ok(preflight),
            },
        ),
    );
    assert!(app.worktree_settings.managed.preflight.is_none());
    assert!(app.worktree_settings.managed.preflight_target.is_none());
    assert_eq!(update(&mut app, RecoveryMessage::Confirm).units(), 0);
}
#[test]
fn worktree_cleanup_flow_preflight_cannot_cross_project() {
    let mut app = app();
    let identity = WorktreeSettingsIdentity::current(&app);
    let preflight = app.worktree_settings.managed.preflight.take().unwrap();
    app.project_path = Some("/other".into());
    let _ = crate::app::message::settings::worktrees::update(
        &mut app,
        crate::app::message::settings::worktrees::WorktreeSettingsMessage::Managed(
            crate::app::message::settings::worktrees::ManagedMessage::PreflightLoaded {
                identity,
                generation: 0,
                target: ("project".into(), "tree".into()),
                result: Ok(preflight),
            },
        ),
    );
    assert!(app.worktree_settings.managed.preflight.is_none());
    assert_eq!(update(&mut app, RecoveryMessage::Confirm).units(), 0);
}
#[test]
fn worktree_cleanup_flow_delete_revokes_inflight_snapshot_list() {
    let mut app = app();
    let identity = WorktreeSettingsIdentity::current(&app);
    app.worktree_settings.recovery.loading = true;
    let _ = update(&mut app, RecoveryMessage::Confirm);
    assert!(!app.worktree_settings.recovery.loading);
    let _ =
        update(&mut app, RecoveryMessage::Loaded { identity, generation: 0, result: Ok(vec![]) });
    assert!(!app.worktree_settings.recovery.loaded);
    assert!(app.worktree_settings.recovery.pending.is_some());
}
