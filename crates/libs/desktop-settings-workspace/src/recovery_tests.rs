use crate::{messages::RecoveryMessage, state::*, recovery};
use vw_api_worktree::*;
fn identity() -> WorktreeSettingsIdentity {
    WorktreeSettingsIdentity {
        gateway: "fixture".into(), tenant: Default::default(), auth_fingerprint: [0; 32], project: None,
    }
}
fn app() -> WorktreeSettingsState {
    let mut settings = WorktreeSettingsState::default();
    let identity = identity();
    let state = &mut settings;
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
    settings
}
fn update(settings: &mut WorktreeSettingsState, event: RecoveryMessage) -> iced::Task<crate::messages::WorktreeSettingsMessage> {
    recovery::update(settings, event, identity(), 0, Err("fixture".into()), &mut Vec::new())
}
#[test]
fn worktree_cleanup_flow_cancel_does_not_delete() {
    let mut app = WorktreeSettingsState::default();
    app.managed.preflight_target = Some(("project".into(), "tree".into()));
    let task = update(&mut app, RecoveryMessage::Cancel);
    assert!(app.managed.preflight_target.is_none());
    assert_eq!(task.units(), 0);
}

#[test]
fn worktree_cleanup_flow_rejects_stale_confirmation() {
    let mut app = app();
    app.managed.preflight.as_mut().unwrap().expires_at_ms = 1;
    let _ = update(&mut app, RecoveryMessage::Confirm);
    assert!(app.recovery.pending.is_none());
    assert!(app.recovery.error.is_some());
}

#[test]
fn worktree_cleanup_flow_ignored_files_require_separate_acknowledgement() {
    let mut app = app();
    app.saved = Some(config_worktree::WorktreeSettings {
        ask_before_delete: false,
        ..Default::default()
    });
    app.managed.preflight.as_mut().unwrap().has_ignored_files = true;
    let _ = update(&mut app, RecoveryMessage::Confirm);
    assert!(app.recovery.pending.is_none());
    let _ = update(&mut app, RecoveryMessage::Ignored(true));
    let task = update(&mut app, RecoveryMessage::Confirm);
    assert_eq!(task.units(), 1);
    assert!(app.recovery.pending.is_some());
    assert_eq!(update(&mut app, RecoveryMessage::Confirm).units(), 0);
}

#[test]
fn worktree_cleanup_flow_protected_confirmation_never_submits() {
    let mut app = app();
    app
        .managed
        .preflight
        .as_mut()
        .unwrap()
        .protection_reasons
        .push(WorktreeProtectionReason::Busy);
    let _ = update(&mut app, RecoveryMessage::Confirm);
    assert!(app.recovery.pending.is_none());
}
