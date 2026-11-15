//! 桌面知识消息编排边界测试。
use super::LoadState;
use vw_gateway_client::{ShijuKnowledgePageDto, ShijuKnowledgeReadDto};
use crate::app::account::SecretSkey;
#[test]
fn shiju_knowledge_state_messages_reject_active_credential_change_and_redact() {
    use crate::app::{
        App,
        account::{AccountIdentity, AccountOrigin, AccountRepository, VerifiedAccount},
        message::skill_hub::knowledge::{KnowledgeMessage, update},
    };
    let (mut app, _) = App::new();
    let mut repository = AccountRepository::default();
    repository.upsert_verified(
        VerifiedAccount::from_parts(
            AccountIdentity::new("company-1", "示例公司", "founder", "active").unwrap(),
            SecretSkey::new("credential").unwrap(),
        ),
        AccountOrigin::BrowserLogin,
    );
    app.account.replace_repository(repository);
    let account = app.account.active_account().unwrap();
    let id = account.id().to_string();
    app.expert_hub
        .knowledge
        .sync_account(id.clone(), SecretSkey::new("obsolete-credential").unwrap());
    let request = app.expert_hub.knowledge.begin_bases().unwrap();
    let _ = update(&mut app, KnowledgeMessage::BasesLoaded(request, Err("stale".into())));
    assert!(matches!(app.expert_hub.knowledge.bases.status, LoadState::Loading { .. }));
    app.expert_hub.knowledge.sync_account(id, SecretSkey::new("credential").unwrap());
    let request = app.expert_hub.knowledge.begin_bases().unwrap();
    let _ = update(
        &mut app,
        KnowledgeMessage::BasesLoaded(
            request,
            Ok(ShijuKnowledgePageDto { items: vec![], page: 1, page_size: 20, total: 0 }),
        ),
    );
    assert!(matches!(app.expert_hub.knowledge.bases.status, LoadState::Loaded(())));
    let request = app.expert_hub.knowledge.begin_bases().unwrap();
    let msg = KnowledgeMessage::ReadLoaded(request, Ok(chunk("private-content", None)));
    assert!(!format!("{msg:?}").contains("private-content"));
    app.expert_hub.clear_for_account(None);
    assert!(app.expert_hub.knowledge.bases.items.is_empty());
    assert!(app.expert_hub.knowledge.reader.is_none());
    assert!(app.expert_hub.knowledge.begin_bases().is_none());
}


fn chunk(content: &str, next_offset: Option<u64>) -> ShijuKnowledgeReadDto {
    ShijuKnowledgeReadDto {
        kb_zid: "kb-a".into(),
        doc_zid: "doc-a".into(),
        title: "文档".into(),
        content: content.into(),
        total_chars: 10,
        next_offset,
    }
}
