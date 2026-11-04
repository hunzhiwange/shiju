//! 所有云端资料请求复用敏感 GatewayClient，并在状态接收前核对当前凭据。
use super::SkillHubMessage;
use crate::app::{
    App, Message, config,
    skill_hub::{
        LoadState,
        knowledge::{KnowledgeRequest, KnowledgeWork},
    },
};
use iced::Task;
use vw_gateway_client::{
    ShijuKnowledgeBaseDto, ShijuKnowledgeDocumentDto, ShijuKnowledgePageDto, ShijuKnowledgeReadDto,
    ShijuKnowledgeReadRequest, ShijuKnowledgeSearchDto, ShijuKnowledgeSearchRequest,
};

#[derive(Clone)]
#[allow(dead_code)] // 后续资料界面直接使用这些显式动作。
pub(crate) enum KnowledgeMessage {
    RefreshBases,
    BasesPage(u32),
    SearchBases(String),
    SelectBase(String),
    RefreshDocuments,
    DocumentsPage(u32),
    SearchDocuments(String),
    Search(String),
    OpenReader {
        kb: String,
        doc: String,
    },
    ContinueRead,
    CloseReader,
    BasesLoaded(KnowledgeRequest, Result<ShijuKnowledgePageDto<ShijuKnowledgeBaseDto>, String>),
    DocumentsLoaded(
        KnowledgeRequest,
        Result<ShijuKnowledgePageDto<ShijuKnowledgeDocumentDto>, String>,
    ),
    SearchLoaded(KnowledgeRequest, Result<ShijuKnowledgeSearchDto, String>),
    ReadLoaded(KnowledgeRequest, Result<ShijuKnowledgeReadDto, String>),
}
impl std::fmt::Debug for KnowledgeMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("KnowledgeMessage(<redacted>)")
    }
}
pub(crate) fn message(message: KnowledgeMessage) -> Message {
    Message::SkillHub(SkillHubMessage::Knowledge(message))
}
fn sync_account(app: &mut App) -> bool {
    let Some(account) = app.account.active_account() else {
        app.expert_hub.knowledge.clear(None);
        app.expert_hub.knowledge.bases.status = LoadState::NeedsLogin;
        return false;
    };
    app.expert_hub.knowledge.sync_account(account.id().to_string(), account.skill_hub_credential());
    true
}
fn current(app: &App, request: &KnowledgeRequest) -> bool {
    app.account.active_account().is_some_and(|account| {
        account.id() == request.account_id
            && app
                .expert_hub
                .knowledge
                .account_matches(account.id(), &account.skill_hub_credential())
    })
}
pub(crate) fn ensure_loaded(app: &mut App) -> Task<Message> {
    if !sync_account(app) {
        return Task::none();
    }
    if matches!(app.expert_hub.knowledge.bases.status, LoadState::Idle | LoadState::Error(_)) {
        return load(app);
    }
    Task::none()
}
pub(crate) fn load(app: &mut App) -> Task<Message> {
    if !sync_account(app) {
        return Task::none();
    }
    app.expert_hub.knowledge.bases.refresh();
    let request = app.expert_hub.knowledge.begin_bases();
    dispatch(app, request)
}
fn load_documents(app: &mut App) -> Task<Message> {
    let request = app.expert_hub.knowledge.begin_documents();
    dispatch(app, request)
}
fn read(app: &mut App) -> Task<Message> {
    let request = app.expert_hub.knowledge.begin_read();
    dispatch(app, request)
}
fn dispatch(app: &App, request: Option<KnowledgeRequest>) -> Task<Message> {
    let Some(request) = request else {
        return Task::none();
    };
    let Some(account) = app.account.active_account() else {
        return Task::none();
    };
    let credential = account.skill_hub_credential();
    Task::perform(
        async move {
            let result = async {
                let credential =
                    credential.gateway_credential().map_err(|error| error.to_string())?;
                Ok::<_, String>((config::gateway_client()?, credential))
            }
            .await;
            match request.work.clone() {
                KnowledgeWork::Bases(query) => {
                    let result = match result {
                        Ok((client, credential)) => {
                            client.shiju_knowledge_bases(&credential, &query).await
                        }
                        Err(error) => Err(error),
                    };
                    KnowledgeMessage::BasesLoaded(request, result)
                }
                KnowledgeWork::Documents(kb, query) => {
                    let result = match result {
                        Ok((client, credential)) => {
                            client.shiju_knowledge_documents(&credential, &kb, &query).await
                        }
                        Err(error) => Err(error),
                    };
                    KnowledgeMessage::DocumentsLoaded(request, result)
                }
                KnowledgeWork::Search(kb, query) => {
                    let result = match result {
                        Ok((client, credential)) => {
                            client
                                .shiju_knowledge_search(
                                    &credential,
                                    &ShijuKnowledgeSearchRequest {
                                        query,
                                        kb_zids: vec![kb],
                                        doc_zids: vec![],
                                        top_k: 5,
                                    },
                                )
                                .await
                        }
                        Err(error) => Err(error),
                    };
                    KnowledgeMessage::SearchLoaded(request, result)
                }
                KnowledgeWork::Read { kb, doc, offset } => {
                    let result = match result {
                        Ok((client, credential)) => {
                            client
                                .shiju_knowledge_read(
                                    &credential,
                                    &ShijuKnowledgeReadRequest {
                                        kb_zid: Some(kb),
                                        doc_zid: doc,
                                        offset,
                                        limit_chars: 16000,
                                    },
                                )
                                .await
                        }
                        Err(error) => Err(error),
                    };
                    KnowledgeMessage::ReadLoaded(request, result)
                }
            }
        },
        message,
    )
}
pub(crate) fn update(app: &mut App, msg: KnowledgeMessage) -> Task<Message> {
    // 响应不得触发账号同步或新请求；旧账号与旧凭据直接丢弃。
    match msg {
        KnowledgeMessage::BasesLoaded(request, result) => {
            if !current(app, &request) {
                return Task::none();
            }
            let previous = app.expert_hub.knowledge.selected_base.clone();
            match app.expert_hub.knowledge.finish_bases(&request, result) {
                Some(true) => {
                    let request = app.expert_hub.knowledge.begin_bases();
                    return dispatch(app, request);
                }
                Some(false)
                    if app.expert_hub.knowledge.selected_base.is_some()
                        && (previous != app.expert_hub.knowledge.selected_base
                            || matches!(
                                app.expert_hub.knowledge.documents.status,
                                LoadState::Idle
                            )) =>
                {
                    return load_documents(app);
                }
                _ => {}
            }
        }
        KnowledgeMessage::DocumentsLoaded(request, result) => {
            if current(app, &request)
                && app.expert_hub.knowledge.finish_documents(&request, result) == Some(true)
            {
                return load_documents(app);
            }
        }
        KnowledgeMessage::SearchLoaded(request, result) => {
            if current(app, &request) {
                app.expert_hub.knowledge.finish_search(&request, result);
            }
        }
        KnowledgeMessage::ReadLoaded(request, result) => {
            if current(app, &request) {
                app.expert_hub.knowledge.finish_read(&request, result);
            }
        }
        action => {
            if !sync_account(app) {
                return Task::none();
            }
            match action {
                KnowledgeMessage::RefreshBases => return load(app),
                KnowledgeMessage::BasesPage(page) => {
                    if app.expert_hub.knowledge.set_base_page(page) {
                        return load(app);
                    }
                }
                KnowledgeMessage::SearchBases(q) => {
                    app.expert_hub.knowledge.search_bases(q);
                    return load(app);
                }
                KnowledgeMessage::SelectBase(kb) => {
                    if app.expert_hub.knowledge.select_base(kb) {
                        return load_documents(app);
                    }
                }
                KnowledgeMessage::RefreshDocuments => {
                    app.expert_hub.knowledge.documents.refresh();
                    return load_documents(app);
                }
                KnowledgeMessage::DocumentsPage(page) => {
                    if app.expert_hub.knowledge.documents.set_page(page) {
                        return load_documents(app);
                    }
                }
                KnowledgeMessage::SearchDocuments(q) => {
                    app.expert_hub.knowledge.search_documents(q);
                    return load_documents(app);
                }
                KnowledgeMessage::Search(query) => {
                    let request = app.expert_hub.knowledge.begin_search(query);
                    return dispatch(app, request);
                }
                KnowledgeMessage::OpenReader { kb, doc } => {
                    if app.expert_hub.knowledge.open_reader(kb, doc) {
                        return read(app);
                    }
                }
                KnowledgeMessage::ContinueRead => return read(app),
                KnowledgeMessage::CloseReader => app.expert_hub.knowledge.close_reader(),
                _ => unreachable!("responses handled above"),
            }
        }
    }
    Task::none()
}
