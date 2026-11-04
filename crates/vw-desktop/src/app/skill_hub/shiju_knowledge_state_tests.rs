//! 直接验证真实状态转换，覆盖分页范围、账号隔离与续读重试。
use super::{LoadState, knowledge::*};
use crate::app::account::SecretSkey;
use vw_gateway_client::*;

fn state() -> KnowledgeState {
    let mut state = KnowledgeState::default();
    state.sync_account("account-a".into(), SecretSkey::new("key-a").unwrap());
    state
}
fn bases(state: &mut KnowledgeState, ids: &[&str], total: u64) {
    let request = state.begin_bases().unwrap();
    state.finish_bases(
        &request,
        Ok(ShijuKnowledgePageDto {
            page: state.bases.query.page,
            page_size: 20,
            total,
            items: ids
                .iter()
                .map(|id| ShijuKnowledgeBaseDto {
                    kb_zid: (*id).into(),
                    name: (*id).into(),
                    description: String::new(),
                    status: "active".into(),
                })
                .collect(),
        }),
    );
}
fn document(state: &mut KnowledgeState) {
    bases(state, &["kb-a", "kb-b"], 50);
    let request = state.begin_documents().unwrap();
    state.finish_documents(
        &request,
        Ok(ShijuKnowledgePageDto {
            page: 1,
            page_size: 20,
            total: 50,
            items: vec![ShijuKnowledgeDocumentDto {
                doc_zid: "doc-a".into(),
                kb_zid: "kb-a".into(),
                title: "文档".into(),
                original_filename: String::new(),
                lifecycle_status: "active".into(),
            }],
        }),
    );
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
#[test]
fn shiju_knowledge_state_pages_reset_by_scope() {
    let mut state = state();
    document(&mut state);
    assert!(state.documents.set_page(2));
    assert_eq!(state.bases.query.page, 1);
    assert!(state.bases.set_page(2));
    assert_eq!(state.documents.query.page, 2);
    state.search_documents("文档".into());
    assert_eq!(state.documents.query.page, 1);
    assert_eq!(state.bases.query.page, 2);
    state.select_base("kb-b".into());
    assert_eq!(state.documents.query.q, "");
    assert_eq!(state.documents.query.page, 1);
    state.search_bases("资料".into());
    assert_eq!(state.bases.query.page, 1);
    assert!(state.selected_base.is_none());
    assert!(!state.bases.set_page(0));
    state.bases.total = u64::MAX;
    assert_eq!(state.bases.pages(), u32::MAX);
    bases(&mut state, &["kb-c"], 1);
    assert_eq!(state.selected_base.as_deref(), Some("kb-c"));
}
#[test]
fn shiju_knowledge_state_discards_stale_responses() {
    let mut state = state();
    let old = state.begin_bases().unwrap();
    bases(&mut state, &["kb-a"], 1);
    assert_eq!(state.finish_bases(&old, Err("stale".into())), None);
    assert!(matches!(state.bases.status, LoadState::Loaded(())));
    let old = state.begin_bases().unwrap();
    state.sync_account("account-a".into(), SecretSkey::new("key-b").unwrap());
    assert_eq!(state.finish_bases(&old, Err("stale credential".into())), None);
    assert!(state.bases.items.is_empty());
    let old = state.begin_bases().unwrap();
    state.clear(None);
    assert_eq!(state.finish_bases(&old, Err("stale account".into())), None);
    assert!(state.begin_bases().is_none());
    assert!(state.reader.is_none());
}
#[test]
fn shiju_knowledge_state_reader_retry_is_idempotent() {
    let mut state = state();
    document(&mut state);
    assert!(state.open_reader("kb-a".into(), "doc-a".into()));
    let first = state.begin_read().unwrap();
    assert!(state.begin_read().is_none());
    state.finish_read(&first, Ok(chunk("first", Some(5))));
    let second = state.begin_read().unwrap();
    state.finish_read(&second, Err("retry".into()));
    let reader = state.reader.as_ref().unwrap();
    assert_eq!(reader.content, "first");
    assert_eq!(reader.next_offset, Some(5));
    let retry = state.begin_read().unwrap();
    assert_eq!(retry.work, second.work);
    state.finish_read(&retry, Ok(chunk("last", None)));
    state.finish_read(&retry, Ok(chunk("last", None)));
    assert_eq!(state.reader.as_ref().unwrap().content, "firstlast");
    assert!(state.begin_read().is_none());
    state.close_reader();
    state.finish_read(&first, Ok(chunk("stale", None)));
    assert!(state.reader.is_none());
}
#[test]
fn shiju_knowledge_state_shrinking_total_corrects_only_once() {
    let mut state = state();
    bases(&mut state, &["kb-a"], 100);
    state.bases.set_page(5);
    let request = state.begin_bases().unwrap();
    assert_eq!(
        state.finish_bases(
            &request,
            Ok(ShijuKnowledgePageDto { items: vec![], page: 5, page_size: 20, total: 21 })
        ),
        Some(true)
    );
    assert_eq!(state.bases.query.page, 2);
    let request = state.begin_bases().unwrap();
    assert_eq!(
        state.finish_bases(
            &request,
            Ok(ShijuKnowledgePageDto { items: vec![], page: 2, page_size: 20, total: 0 })
        ),
        Some(false)
    );
}
#[test]
fn shiju_knowledge_state_refresh_restarts_single_page_correction_budget() {
    let mut state = state();
    bases(&mut state, &["kb-a"], 100);
    state.set_base_page(5);
    let request = state.begin_bases().unwrap();
    assert_eq!(
        state.finish_bases(
            &request,
            Ok(ShijuKnowledgePageDto { items: vec![], page: 5, page_size: 20, total: 21 })
        ),
        Some(true)
    );
    let request = state.begin_bases().unwrap();
    assert_eq!(
        state.finish_bases(
            &request,
            Ok(ShijuKnowledgePageDto { items: vec![], page: 2, page_size: 20, total: 0 })
        ),
        Some(false)
    );
    assert!(matches!(state.bases.status, LoadState::Error(_)));
    state.bases.refresh();
    let request = state.begin_bases().unwrap();
    assert_eq!(
        state.finish_bases(
            &request,
            Ok(ShijuKnowledgePageDto { items: vec![], page: 2, page_size: 20, total: 0 })
        ),
        Some(true)
    );
    assert_eq!(state.bases.query.page, 1);
}

#[test]
fn shiju_knowledge_state_selection_invalidates_documents_search_and_reader() {
    let mut state = state();
    document(&mut state);
    let docs = state.begin_documents().unwrap();
    let search = state.begin_search("query".into()).unwrap();
    state.open_reader("kb-a".into(), "doc-a".into());
    let read = state.begin_read().unwrap();
    state.select_base("kb-b".into());
    assert_eq!(state.finish_documents(&docs, Err("stale".into())), None);
    state.finish_search(&search, Err("stale".into()));
    state.finish_read(&read, Ok(chunk("stale", None)));
    assert!(matches!(state.search, LoadState::Idle));
    assert!(state.reader.is_none());
}

#[test]
fn shiju_knowledge_state_rejects_mismatched_pages_and_reader_identity() {
    let mut state = state();
    document(&mut state);
    state.documents.set_page(2);
    let request = state.begin_documents().unwrap();
    state.finish_documents(&request, Err("offline".into()));
    assert_eq!(state.documents.query.page, 2);
    let retry = state.begin_documents().unwrap();
    assert_eq!(retry.work, request.work);
    state.finish_documents(
        &retry,
        Ok(ShijuKnowledgePageDto { items: vec![], page: 1, page_size: 20, total: 50 }),
    );
    assert!(matches!(state.documents.status, LoadState::Error(_)));
    state.open_reader("kb-a".into(), "doc-a".into());
    let request = state.begin_read().unwrap();
    let mut wrong = chunk("secret from wrong document", None);
    wrong.doc_zid = "doc-b".into();
    state.finish_read(&request, Ok(wrong));
    assert!(state.reader.as_ref().unwrap().content.is_empty());
    assert!(matches!(state.reader.as_ref().unwrap().status, LoadState::Error(_)));
    let retry = state.begin_read().unwrap();
    assert_eq!(retry.work, request.work);
    state.close_reader();
    state.open_reader("kb-a".into(), "doc-a".into());
    state.finish_read(&retry, Ok(chunk("old reader", None)));
    assert!(state.reader.as_ref().unwrap().content.is_empty());
}

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

#[test]
fn shiju_knowledge_state_base_page_failure_clears_previous_selection() {
    let mut state = state();
    document(&mut state);
    state.open_reader("kb-a".into(), "doc-a".into());
    let old_docs = state.begin_documents().unwrap();
    assert!(state.set_base_page(2));
    assert!(state.selected_base.is_none());
    assert!(state.documents.items.is_empty());
    assert_eq!(state.documents.query.page, 1);
    assert!(state.reader.is_none());
    let request = state.begin_bases().unwrap();
    state.finish_bases(&request, Err("offline".into()));
    assert!(state.selected_base.is_none());
    assert!(state.bases.items.is_empty());
    assert_eq!(state.bases.query.page, 2);
    assert_eq!(state.finish_documents(&old_docs, Err("stale".into())), None);
}
