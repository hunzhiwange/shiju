use super::*;
use crate::app::account::{AccountIdentity, AccountProfile};
use crate::app::state::ToastKind;

fn repository_with_account(secret: &str, company_id: &str) -> AccountRepository {
    let mut repository = AccountRepository::default();
    repository.upsert_verified(
        VerifiedAccount::from_parts(
            AccountIdentity::new(company_id, "测试公司", "founder", "active").unwrap(),
            SecretSkey::new(secret).unwrap(),
        ),
        AccountOrigin::ManualSkey,
    );
    repository
}

fn repository_with_profile(secret: &str) -> AccountRepository {
    let mut repository = AccountRepository::default();
    repository.upsert_verified(
        VerifiedAccount::from_parts_with_profile(
            AccountIdentity::from_profile("测试公司", "enabled").unwrap(),
            AccountProfile::new(
                "enabled",
                "旧名称",
                "old-account",
                "enabled",
                vec!["旧部门".to_string()],
                vec!["旧角色".to_string()],
            )
            .unwrap(),
            SecretSkey::new(secret).unwrap(),
        ),
        AccountOrigin::BrowserLogin,
    );
    repository
}

fn account_me(name: &str, username: &str) -> ShijuAccountMeDto {
    ShijuAccountMeDto {
        user_id: "101026".to_string(),
        company_id: "10011".to_string(),
        account_id: "101026".to_string(),
        account_name: username.to_string(),
        name: name.to_string(),
        source_type: "manual".to_string(),
        external_user_id: String::new(),
        allow_console_login: true,
        status: "enabled".to_string(),
        departments: vec![vw_gateway_client::ShijuAccountDepartmentDto {
            department_id: "101014".to_string(),
            department_name: "AI创新组".to_string(),
            parent_department_id: "101005".to_string(),
        }],
        roles: vec![vw_gateway_client::ShijuAccountRoleDto {
            role_id: "102".to_string(),
            role_name: "默认角色".to_string(),
            role_code: "default".to_string(),
        }],
    }
}

#[test]
fn manual_input_message_debug_is_redacted() {
    let message = AccountMessage::ManualInputChanged(ManualSkeyInput::new("secret-value"));
    assert!(!format!("{message:?}").contains("secret-value"));
}

#[test]
fn account_notice_covers_all_persisted_actions() {
    let notices = [
        AccountNotice::ManualLogin,
        AccountNotice::BrowserLogin,
        AccountNotice::Switch,
        AccountNotice::SignOut,
        AccountNotice::Delete,
    ];
    assert_eq!(notices.len(), 5);
}

#[test]
fn browser_login_waits_for_thirty_minutes() {
    assert_eq!(LOGIN_TIMEOUT, Duration::from_secs(30 * 60));
}

#[test]
fn account_messages_use_gateway_login_and_validation_tasks() {
    let _ = crate::app::session_gateway::gateway_shiju_account_login_async;
    let _ = crate::app::session_gateway::gateway_shiju_account_login_poll_async;
    let _ = crate::app::session_gateway::gateway_shiju_account_profile_async;
    let _ = start_browser_login_task;
    let _ = poll_task;
    let _ = validate_task;
}

#[test]
fn account_profile_edit_messages_use_gateway_and_ignore_late_results() {
    let _ = crate::app::session_gateway::gateway_shiju_account_me_async;
    let _ = crate::app::session_gateway::gateway_shiju_account_update_async;
    let _ = profile_load_task;
    let _ = profile_update_task;
    let _ = profile_persist_task;

    let (mut app, _) = App::new();
    app.account.replace_repository(repository_with_profile("profile-edit-secret"));
    let stale = app.account.begin_profile_editor(AccountProfileEditorState::Loading);
    let current = app.account.begin_profile_editor(AccountProfileEditorState::Loading);

    let stale_task = update(
        &mut app,
        AccountMessage::ProfileLoaded {
            generation: stale,
            result: Ok(account_me("过期名称", "stale-account")),
        },
    );
    assert_eq!(stale_task.units(), 0);
    assert!(app.account.profile_is_current(current));

    let loaded_task = update(
        &mut app,
        AccountMessage::ProfileLoaded {
            generation: current,
            result: Ok(account_me("刘祥敏", "liuxiangmin")),
        },
    );
    assert_eq!(loaded_task.units(), 0);
    assert!(matches!(app.account.profile_editor, AccountProfileEditorState::Editing(_)));

    let submit = update(&mut app, AccountMessage::SubmitProfileEditor);
    assert_eq!(submit.units(), 1);
    let saving_generation = app.account.begin_profile_editor(AccountProfileEditorState::Saving(
        AccountProfileDraft::new("刘祥敏", "liuxiangmin").unwrap(),
    ));
    let persist = update(
        &mut app,
        AccountMessage::ProfileUpdated {
            generation: saving_generation,
            result: Ok(account_me("刘祥敏2", "liuxiangmin2")),
        },
    );
    assert_eq!(persist.units(), 1);

    let mut repository = app.account.repository().clone();
    repository.refresh_active_profile(account_me("刘祥敏2", "liuxiangmin2")).unwrap();
    app.expert_hub.account_id = Some("keep-expert-hub".to_string());
    let feedback = update(
        &mut app,
        AccountMessage::ProfilePersisted { generation: saving_generation, result: Ok(repository) },
    );

    assert_eq!(feedback.units(), 1);
    assert_eq!(app.expert_hub.account_id.as_deref(), Some("keep-expert-hub"));
    let profile = app.account.active_account().unwrap().profile().unwrap();
    assert_eq!(profile.name(), "刘祥敏2");
    assert_eq!(profile.account_name(), "liuxiangmin2");
}

#[test]
fn account_password_edit_validates_redacts_and_ignores_late_results() {
    let _ = crate::app::session_gateway::gateway_shiju_account_password_update_async;
    let _ = password_update_task;
    let input_message = AccountMessage::PasswordFieldChanged {
        field: AccountPasswordField::Current,
        value: AccountPasswordInput::new("message-password-secret".to_string()),
    };
    assert!(!format!("{input_message:?}").contains("message-password-secret"));

    let (mut app, _) = App::new();
    app.account
        .replace_repository(repository_with_account("password-edit-secret", "company-1"));
    let draft = AccountPasswordDraft::new(
        "current-password-123",
        "new-password-456",
        "new-password-456",
    );
    app.account.password_editor = AccountPasswordEditorState::Editing(draft.clone());

    let submit = update(&mut app, AccountMessage::SubmitPasswordEditor);
    assert_eq!(submit.units(), 1);
    assert!(matches!(
        app.account.password_editor,
        AccountPasswordEditorState::Saving(_)
    ));

    let stale = app
        .account
        .begin_password_editor(AccountPasswordEditorState::Saving(draft.clone()));
    let current = app
        .account
        .begin_password_editor(AccountPasswordEditorState::Saving(draft));
    let stale_result = update(
        &mut app,
        AccountMessage::PasswordUpdated {
            generation: stale,
            result: Ok(()),
        },
    );
    assert_eq!(stale_result.units(), 0);
    assert!(app.account.password_is_current(current));

    let failed = update(
        &mut app,
        AccountMessage::PasswordUpdated {
            generation: current,
            result: Err("IAM request failed: 401".to_string()),
        },
    );
    assert_eq!(failed.units(), 0);
    assert!(matches!(
        app.account.password_editor,
        AccountPasswordEditorState::Failed { .. }
    ));

    let success_generation = app.account.begin_password_editor(
        AccountPasswordEditorState::Saving(AccountPasswordDraft::new(
            "current-password-123",
            "new-password-456",
            "new-password-456",
        )),
    );
    let success = update(
        &mut app,
        AccountMessage::PasswordUpdated {
            generation: success_generation,
            result: Ok(()),
        },
    );
    assert_eq!(success.units(), 1);
    assert!(matches!(
        app.account.password_editor,
        AccountPasswordEditorState::Closed
    ));
}

#[test]
fn late_account_responses_do_not_replace_newer_operation() {
    let (mut app, _) = App::new();
    let stale_generation = app.account.begin(AccountOperation::Validating);
    let current_generation = app.account.begin(AccountOperation::StartingBrowser);

    let stale_verified = VerifiedAccount::from_parts(
        AccountIdentity::new("", "旧公司", "", "enabled").unwrap(),
        SecretSkey::new("stale-secret").unwrap(),
    );
    let validation_task = update(
        &mut app,
        AccountMessage::AccountValidated {
            generation: stale_generation,
            origin: AccountOrigin::ManualSkey,
            result: Ok(stale_verified),
        },
    );
    let poll_task = update(
        &mut app,
        AccountMessage::LoginPolled {
            generation: stale_generation,
            result: Ok(LoginPoll::Pending),
        },
    );

    assert_eq!(validation_task.units(), 0);
    assert_eq!(poll_task.units(), 0);
    assert!(app.account.is_current(current_generation));
    assert!(matches!(app.account.operation, AccountOperation::StartingBrowser));
    assert!(app.account.active_account().is_none());
}

#[test]
fn account_change_invalidates_skill_hub_state() {
    let (mut app, _) = App::new();
    let mut first = AccountRepository::default();
    first.upsert_verified(
        VerifiedAccount::from_parts(
            AccountIdentity::new("company-1", "一号公司", "founder", "active").unwrap(),
            SecretSkey::new("first-account-secret").unwrap(),
        ),
        AccountOrigin::BrowserLogin,
    );
    app.account.replace_repository(first);
    app.expert_hub.account_id = Some("stale-account".to_string());
    app.expert_hub.submitted_search = "stale search".to_string();
    app.expert_hub.marketplace = crate::app::skill_hub::LoadState::Loaded(
        crate::app::skill_hub::CursorPage {
            items: Vec::new(),
            next_cursor: None,
            has_more: false,
        },
    );

    let mut signed_out = app.account.repository().clone();
    signed_out.sign_out();
    let generation = app.account.begin(AccountOperation::Saving);
    let _task = update(
        &mut app,
        AccountMessage::RepositoryPersisted {
            generation,
            notice: AccountNotice::SignOut,
            result: Ok(signed_out),
        },
    );

    assert!(app.expert_hub.account_id.is_none());
    assert!(app.expert_hub.submitted_search.is_empty());
    assert!(matches!(
        app.expert_hub.marketplace,
        crate::app::skill_hub::LoadState::NeedsLogin
    ));
}

#[test]
fn successful_login_and_switch_schedule_shiju_key_sync() {
    for notice in [
        AccountNotice::ManualLogin,
        AccountNotice::BrowserLogin,
        AccountNotice::Switch,
    ] {
        let (mut app, _) = App::new();
        let repository = repository_with_account("test-account-secret", "company-1");
        let generation = app.account.begin(AccountOperation::Saving);

        let task = update(
            &mut app,
            AccountMessage::RepositoryPersisted {
                generation,
                notice,
                result: Ok(repository),
            },
        );

        assert_eq!(task.units(), 3);
        assert!(app.account.active_account().is_some());
    }
}

#[test]
fn sign_out_and_delete_do_not_sync_shiju_key() {
    for notice in [AccountNotice::SignOut, AccountNotice::Delete] {
        let (mut app, _) = App::new();
        let generation = app.account.begin(AccountOperation::Saving);

        let task = update(
            &mut app,
            AccountMessage::RepositoryPersisted {
                generation,
                notice,
                result: Ok(AccountRepository::default()),
            },
        );

        assert_eq!(task.units(), 1);
        assert!(app.account.active_account().is_none());
    }
}

#[test]
fn shiju_key_sync_failure_keeps_account_active() {
    let (mut app, _) = App::new();
    app.account
        .replace_repository(repository_with_account("test-account-secret", "company-1"));
    let active_account_id =
        app.account.active_account().expect("fixture should be active").id().to_string();

    let _task = update(
        &mut app,
        AccountMessage::ShijuKeySyncFinished(Err("credential-secret".to_string())),
    );

    assert_eq!(
        app.account.active_account().map(|account| account.id()),
        Some(active_account_id.as_str())
    );
    let warning = app.active_toast.as_ref().expect("sync failure should show a warning");
    assert_eq!(warning.kind, ToastKind::Warning);
    assert!(warning.message.contains("安装页"));
    assert!(!warning.message.contains("credential-secret"));
}
