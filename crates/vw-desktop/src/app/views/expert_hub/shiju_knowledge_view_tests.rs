use crate::app::{App, account::SecretSkey};
use crate::app::message::skill_hub::knowledge::KnowledgeMessage;
#[test]
fn shiju_knowledge_view_notifies_accepted_errors_only() {
    use crate::app::account::{AccountIdentity, AccountOrigin, AccountRepository, VerifiedAccount};
    use crate::app::message::skill_hub::knowledge::update;
    let (mut app, _) = App::new();
    let mut repository = AccountRepository::default();
    repository.upsert_verified(
        VerifiedAccount::from_parts(
            AccountIdentity::new("company-1", "示例公司", "founder", "active").unwrap(),
            SecretSkey::new("fixture-only").unwrap(),
        ),
        AccountOrigin::BrowserLogin,
    );
    app.account.replace_repository(repository);
    let account = app.account.active_account().unwrap();
    app.expert_hub.knowledge.sync_account(account.id().to_string(), account.skill_hub_credential());
    let stale = app.expert_hub.knowledge.begin_bases().unwrap();
    let current = app.expert_hub.knowledge.begin_bases().unwrap();
    let _ = update(&mut app, KnowledgeMessage::BasesLoaded(stale, Err("obsolete".into())));
    assert!(app.toast_state.active_toast.is_none());
    let _ = update(
        &mut app,
        KnowledgeMessage::BasesLoaded(current.clone(), Err("fixture-only-error".into())),
    );
    assert_eq!(app.toast_state.active_toast.as_ref().unwrap().message, "资料加载失败，请重试");
    app.toast_state.active_toast = None;
    let _ = update(&mut app, KnowledgeMessage::BasesLoaded(current, Err("duplicate".into())));
    assert!(app.toast_state.active_toast.is_none());
}
