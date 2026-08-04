use super::*;
use crate::app::account::{
    AccountIdentity, AccountOrigin, AccountPasswordDraft, AccountProfileDraft, SecretSkey,
    VerifiedAccount,
};

fn repository_with_accounts() -> AccountRepository {
    let mut repository = AccountRepository::default();
    for number in 1..=2 {
        repository.upsert_verified(
            VerifiedAccount::from_parts(
                AccountIdentity::new(
                    format!("company-{number}"),
                    format!("账户 {number}"),
                    "founder",
                    "active",
                )
                .unwrap(),
                SecretSkey::new(format!("key-{number}")).unwrap(),
            ),
            AccountOrigin::ManualSkey,
        );
    }
    repository
}

#[test]
fn startup_restore_uses_exact_active_account() {
    let repository = repository_with_accounts();
    let expected = repository.active_account_id().unwrap().to_string();
    let mut state = AccountState::default();
    state.replace_repository(repository);
    assert_eq!(state.active_account().unwrap().id(), expected);
}

#[test]
fn stale_results_are_ignored_after_new_operation() {
    let mut state = AccountState::default();
    let old = state.begin(AccountOperation::Validating);
    let current = state.begin(AccountOperation::StartingBrowser);
    assert!(!state.is_current(old));
    assert!(state.is_current(current));
}

#[test]
fn cancel_returns_to_idle_and_invalidates_pending_results() {
    let mut state = AccountState::default();
    let cancelled = state.begin(AccountOperation::StartingBrowser);

    state.cancel();

    assert!(!state.operation.is_busy());
    assert!(!state.is_current(cancelled));
}

#[test]
fn manual_input_debug_is_redacted() {
    let mut state = AccountState::default();
    state.manual_input = ManualSkeyInput::new("top-secret");
    let debug = format!("{state:?}");
    assert!(!debug.contains("top-secret"));
    assert!(debug.contains("<redacted>"));
}

#[test]
fn account_profile_edit_generation_rejects_stale_results_and_cancels_cleanly() {
    let mut state = AccountState::default();
    let stale = state.begin_profile_editor(AccountProfileEditorState::Loading);
    let current = state.begin_profile_editor(AccountProfileEditorState::Editing(
        AccountProfileDraft::new("刘祥敏", "liuxiangmin").unwrap(),
    ));

    assert!(!state.profile_is_current(stale));
    assert!(state.profile_is_current(current));
    assert!(!state.profile_editor.is_busy());

    state.cancel_profile_editor();

    assert!(!state.profile_is_current(current));
    assert!(matches!(state.profile_editor, AccountProfileEditorState::Closed));
}

#[test]
fn account_password_edit_validates_redacts_and_ignores_late_results() {
    let draft = AccountPasswordDraft::new(
        "current-password-123",
        "new-password-456",
        "new-password-456",
    );
    assert!(draft.update_request().is_ok());
    assert!(
        AccountPasswordDraft::new("current-password-123", "short1", "short1")
            .update_request()
            .is_err()
    );
    assert!(
        AccountPasswordDraft::new("current-password-123", "onlyletters", "onlyletters")
            .update_request()
            .is_err()
    );
    assert!(
        AccountPasswordDraft::new("current-password-123", "12345678", "12345678")
            .update_request()
            .is_err()
    );
    assert!(
        AccountPasswordDraft::new(
            "current-password-123",
            "new-password-456",
            "different-password-789",
        )
        .update_request()
        .is_err()
    );
    let debug = format!("{draft:?}");
    assert!(!debug.contains("current-password-123"));
    assert!(!debug.contains("new-password-456"));
    assert!(debug.contains("<redacted>"));

    let mut state = AccountState::default();
    let stale =
        state.begin_password_editor(AccountPasswordEditorState::Saving(draft.clone()));
    let current =
        state.begin_password_editor(AccountPasswordEditorState::Editing(draft));
    assert!(!state.password_is_current(stale));
    assert!(state.password_is_current(current));

    state.cancel_password_editor();
    assert!(!state.password_is_current(current));
    assert!(matches!(state.password_editor, AccountPasswordEditorState::Closed));
}
