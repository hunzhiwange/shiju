//! 跨域账号与 toast 映射的桌面组合测试。
use crate::app::{App, account::SecretSkey, skill_hub::{ExpertHubSection, LoadState}};
use crate::app::message::skill_hub::knowledge::KnowledgeMessage;
use gateway_shiju_knowledge::{ShijuKnowledgeRecordDto, ShijuKnowledgeSearchDto};
fn app() -> App {
    let (mut app, _) = App::new();
    app.expert_hub.section = ExpertHubSection::Knowledge;
    let state = &mut app.expert_hub.knowledge;
    state.sync_account("fixture".into(), SecretSkey::new("fixture-only").unwrap());
    state.selected_base = Some("kb-a".into());
    state.search_mode = crate::app::skill_hub::knowledge::KnowledgeSearchMode::Content;
    state.bases.status = LoadState::Loaded(());
    state.documents.status = LoadState::Loaded(());
    state.search = LoadState::Loaded(ShijuKnowledgeSearchDto {
        records: ["doc-a", "doc-b"]
            .into_iter()
            .map(|id| ShijuKnowledgeRecordDto {
                kb_zid: "kb-a".into(),
                doc_zid: id.into(),
                title: "相同的中文标题".repeat(8),
                content: "检索摘要".into(),
                score: 0.75,
            })
            .collect(),
        total: 2,
    });
    app
}
#[test]
fn shiju_knowledge_reader_notifies_accepted_errors_only() {
    use crate::app::account::{AccountIdentity, AccountOrigin, AccountRepository, VerifiedAccount};
    use crate::app::message::skill_hub::knowledge::update;
    let mut app = app();
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
    let results = app.expert_hub.knowledge.search.clone();
    app.expert_hub.knowledge.sync_account(account.id().to_string(), account.skill_hub_credential());
    let state = &mut app.expert_hub.knowledge;
    state.selected_base = Some("kb-a".into());
    state.search_mode = crate::app::skill_hub::knowledge::KnowledgeSearchMode::Content;
    state.search = results;
    assert!(state.open_reader("kb-a".into(), "doc-b".into()));
    let read = state.begin_read().unwrap();
    let search = state.begin_search("中文".into()).unwrap();
    let _ = update(&mut app, KnowledgeMessage::SearchLoaded(search.clone(), Err("offline".into())));
    assert_eq!(app.toast_state.active_toast.as_ref().unwrap().message, "检索失败，请重试");
    app.toast_state.active_toast = None;
    let _ = update(&mut app, KnowledgeMessage::SearchLoaded(search, Err("duplicate".into())));
    assert!(app.toast_state.active_toast.is_none());
    let _ = update(&mut app, KnowledgeMessage::ReadLoaded(read.clone(), Err("offline".into())));
    assert_eq!(app.toast_state.active_toast.as_ref().unwrap().message, "原文加载失败，请重试");
    app.toast_state.active_toast = None;
    let _ = update(&mut app, KnowledgeMessage::ReadLoaded(read, Err("duplicate".into())));
    assert!(app.toast_state.active_toast.is_none());
}
