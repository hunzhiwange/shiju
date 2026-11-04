//! 从真实检索按钮打开原文，验证中文正文续读及过期响应。
use super::shiju_knowledge_view_tests::inspect;
use crate::app::message::{SkillHubMessage, skill_hub::knowledge::KnowledgeMessage};
use crate::app::{
    App, Message,
    account::SecretSkey,
    skill_hub::{ExpertHubSection, LoadState},
};
use iced::Theme;
use vw_gateway_client::{ShijuKnowledgeReadDto, ShijuKnowledgeRecordDto, ShijuKnowledgeSearchDto};

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
fn chunk(content: &str, next_offset: Option<u64>) -> ShijuKnowledgeReadDto {
    ShijuKnowledgeReadDto {
        kb_zid: "kb-a".into(),
        doc_zid: "doc-b".into(),
        title: "相同的中文标题".repeat(8),
        content: content.into(),
        total_chars: 100,
        next_offset,
    }
}
fn action(messages: &[Message]) -> &KnowledgeMessage {
    match &messages[0] {
        Message::SkillHub(SkillHubMessage::Knowledge(action)) => action,
        _ => panic!("knowledge action"),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn shiju_knowledge_search_modes_preserve_results_and_reader() {
    use crate::app::skill_hub::knowledge::KnowledgeSearchMode;
    let mut app = app();
    for theme in [Theme::Light, Theme::Dark] {
        app.app_theme = theme.clone();
        app.expert_hub.knowledge.search_mode = KnowledgeSearchMode::Catalog;
        let (texts, _) = inspect(super::view(&app), theme.clone(), 1100.0, None).await;
        assert!(texts.contains(&"文档".into()));
        assert!(!texts.contains(&"正文".into()));
        app.expert_hub.knowledge.search_mode = KnowledgeSearchMode::Content;
        let (texts, _) = inspect(super::view(&app), theme.clone(), 1100.0, None).await;
        assert!(texts.contains(&"检索摘要".into()));
        assert!(app.expert_hub.knowledge.open_reader("kb-a".into(), "doc-b".into()));
        let request = app.expert_hub.knowledge.begin_read().unwrap();
        app.expert_hub
            .knowledge
            .finish_read(&request, Ok(chunk("# 中文正文\n\n```text\n中文代码\n```", None)));
        let (texts, messages) = inspect(super::view(&app), theme, 1100.0, Some(("关闭", 0))).await;
        assert!(texts.contains(&"已读完".into()));
        assert!(!texts.contains(&"正文".into()));
        assert!(matches!(action(&messages), KnowledgeMessage::CloseReader));
        app.expert_hub.knowledge.close_reader();
        assert_eq!(app.expert_hub.knowledge.search_mode, KnowledgeSearchMode::Content);
        assert!(matches!(app.expert_hub.knowledge.search, LoadState::Loaded(_)));
    }
}
#[tokio::test(flavor = "multi_thread")]
async fn shiju_knowledge_reader_opens_search_result_identity() {
    let mut app = app();
    let (texts, messages) =
        inspect(super::view(&app), Theme::Light, 1100.0, Some(("打开原文", 1))).await;
    assert!(texts.contains(&"检索摘要".into()));
    assert!(texts.iter().any(|s| s.contains("0.750")));
    let KnowledgeMessage::OpenReader { kb, doc } = action(&messages) else { panic!("open reader") };
    assert_eq!((kb.as_str(), doc.as_str()), ("kb-a", "doc-b"));
    assert!(app.expert_hub.knowledge.open_reader(kb.clone(), doc.clone()));
}
#[tokio::test(flavor = "multi_thread")]
async fn shiju_knowledge_reader_retries_same_offset_without_duplicates() {
    let mut app = app();
    let state = &mut app.expert_hub.knowledge;
    assert!(state.open_reader("kb-a".into(), "doc-b".into()));
    let first = state.begin_read().unwrap();
    state.finish_read(
        &first,
        Ok(chunk("# 中文标题\n\n第一段正文。\n\n```text\n中文代码\n```\n", Some(50))),
    );
    for theme in [Theme::Light, Theme::Dark] {
        app.app_theme = theme.clone();
        let (_, messages) = inspect(super::view(&app), theme, 720.0, Some(("继续读取", 0))).await;
        assert!(matches!(action(&messages), KnowledgeMessage::ContinueRead));
    }
    let state = &mut app.expert_hub.knowledge;
    let next = state.begin_read().unwrap();
    assert!(state.begin_read().is_none());
    state.finish_read(&next, Err("fixture offline".into()));
    let before = state.reader.as_ref().unwrap().content.clone();
    let (_, messages) = inspect(super::view(&app), Theme::Dark, 720.0, Some(("重试", 0))).await;
    assert!(matches!(action(&messages), KnowledgeMessage::ContinueRead));
    let state = &mut app.expert_hub.knowledge;
    let retry = state.begin_read().unwrap();
    assert_eq!(retry.work, next.work);
    state.finish_read(&retry, Ok(chunk("\n第二段正文。", None)));
    state.finish_read(&retry, Ok(chunk("\n第二段正文。", None)));
    assert_eq!(state.reader.as_ref().unwrap().content, before + "\n第二段正文。");
    assert!(state.begin_read().is_none());
    for theme in [Theme::Light, Theme::Dark] {
        app.app_theme = theme.clone();
        let (texts, _) = inspect(super::view(&app), theme, 720.0, None).await;
        assert!(texts.contains(&"已读完".into()));
        assert!(!texts.contains(&"继续读取".into()));
    }
}
#[tokio::test(flavor = "multi_thread")]
async fn shiju_knowledge_reader_ignores_closed_document_response() {
    let mut app = app();
    app.expert_hub.knowledge.open_reader("kb-a".into(), "doc-b".into());
    let request = app.expert_hub.knowledge.begin_read().unwrap();
    let (_, messages) = inspect(super::view(&app), Theme::Dark, 1100.0, Some(("关闭", 0))).await;
    assert!(matches!(action(&messages), KnowledgeMessage::CloseReader));
    let state = &mut app.expert_hub.knowledge;
    state.close_reader();
    state.finish_read(&request, Ok(chunk("过期内容", None)));
    assert!(state.reader.is_none());
    state.open_reader("kb-a".into(), "doc-a".into());
    state.finish_read(&request, Ok(chunk("过期内容", None)));
    assert!(state.reader.as_ref().unwrap().content.is_empty());
    state.clear(None);
    state.finish_read(&request, Ok(chunk("过期内容", None)));
    assert!(state.reader.is_none());
}

/// 复用桌面客户端测试的 loopback HTTP 模式，真实协议响应覆盖 21×21 数据集。
#[tokio::test(flavor = "multi_thread")]
async fn shiju_knowledge_reader_walks_local_http_pages_and_chinese_chunks() {
    use serde_json::{Value, json};
    use std::{
        io::{Read, Write},
        net::TcpListener,
        sync::{
            Arc, Mutex,
            atomic::{AtomicBool, Ordering},
        },
        thread,
        time::Duration,
    };
    use vw_gateway_client::{
        GatewayClient, GatewayEndpoint, ShijuCredential, ShijuKnowledgeReadRequest,
        ShijuKnowledgeSearchRequest,
    };
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let port = listener.local_addr().unwrap().port();
    let requests = Arc::new(Mutex::new(Vec::new()));
    let stop = Arc::new(AtomicBool::new(false));
    let recorded = requests.clone();
    let stopped = stop.clone();
    let server = thread::spawn(move || {
        let mut read_count = 0;
        while !stopped.load(Ordering::Relaxed) {
            let (mut stream, _) = match listener.accept() {
                Ok(pair) => pair,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(2));
                    continue;
                }
                Err(error) => panic!("accept: {error}"),
            };
            stream.set_nonblocking(false).unwrap();
            stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
            let mut bytes = vec![];
            let (header_end, length) = loop {
                let mut buffer = [0; 4096];
                let n = stream.read(&mut buffer).unwrap();
                assert!(n > 0);
                bytes.extend_from_slice(&buffer[..n]);
                if let Some(end) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                    let headers = String::from_utf8_lossy(&bytes[..end]).to_lowercase();
                    let length = headers
                        .lines()
                        .find_map(|line| line.strip_prefix("content-length:"))
                        .map(|v| v.trim().parse::<usize>().unwrap())
                        .unwrap_or(0);
                    break (end + 4, length);
                }
            };
            while bytes.len() < header_end + length {
                let mut buffer = [0; 4096];
                let n = stream.read(&mut buffer).unwrap();
                assert!(n > 0);
                bytes.extend_from_slice(&buffer[..n]);
            }
            let request = String::from_utf8_lossy(&bytes[..header_end]);
            assert!(request.to_lowercase().contains("x-vibewindow-shiju-credential:"));
            let path = request.split_whitespace().nth(1).unwrap();
            let url = reqwest::Url::parse(&format!("http://localhost{path}")).unwrap();
            let query =
                url.query_pairs().into_owned().collect::<std::collections::BTreeMap<_, _>>();
            let body: Value = if length == 0 {
                Value::Null
            } else {
                serde_json::from_slice(&bytes[header_end..header_end + length]).unwrap()
            };
            recorded.lock().unwrap().push((url.path().to_string(), query.clone(), body.clone()));
            let (status, response) = match url.path() {
                "/v1/shiju/knowledge/bases" | "/v1/shiju/knowledge/documents" => {
                    let page: u32 = query["page"].parse().unwrap();
                    let documents = url.path().ends_with("documents");
                    let total = if query.get("q").is_some_and(|q| q == "收缩") { 1 } else { 21 };
                    let items = ((page-1)*20..(page*20).min(total)).map(|i| {
                        if documents { json!({"kb_zid":query["kb_zid"], "doc_zid":format!("doc-{i}"),"title":format!("中文文档 {i}"),"lifecycle_status":"ready"}) }
                        else { json!({"kb_zid":format!("kb-{i}"), "name":format!("中文资料库 {i}"), "status":"active"}) }
                    }).collect::<Vec<_>>();
                    (200, json!({"items":items,"page":page,"pageSize":20,"total":total}))
                }
                "/v1/shiju/knowledge/search" => {
                    assert_eq!(body["top_k"], 5);
                    assert_eq!(body["kb_zids"], json!(["kb-20"]));
                    (
                        200,
                        json!({"records":[{"kb_zid":"kb-20","doc_zid":"doc-20","title":"原文","content":"摘要","score":0.9}],"total":1}),
                    )
                }
                "/v1/shiju/knowledge/read" => {
                    read_count += 1;
                    if read_count == 2 {
                        (503, json!({"error":"fixture_offline"}))
                    } else {
                        let first = body["offset"] == 0;
                        (
                            200,
                            json!({"kb_zid":"kb-20","doc_zid":"doc-20","title":"原文","content":if first {"第一段中文。"} else {"第二段中文。"},"total_chars":12,"next_offset":if first {Some(6)} else {None}}),
                        )
                    }
                }
                _ => panic!("unexpected route"),
            };
            let response = response.to_string();
            write!(stream,"HTTP/1.1 {status} Fixture\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{response}",response.len()).unwrap();
        }
    });
    // 正常结束时停止并回收 fixture，避免后台线程泄漏。
    struct Fixture {
        stop: Arc<AtomicBool>,
        handle: Option<thread::JoinHandle<()>>,
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            self.stop.store(true, Ordering::Relaxed);
            self.handle.take().unwrap().join().unwrap();
        }
    }
    let _fixture = Fixture { stop, handle: Some(server) };
    let client = GatewayClient::new(GatewayEndpoint::new("127.0.0.1", port)).unwrap();
    let credential = ShijuCredential::new("fixture-only").unwrap();
    let mut app = app();
    let state = &mut app.expert_hub.knowledge;
    state.clear(None);
    state.sync_account("fixture".into(), SecretSkey::new("fixture-only").unwrap());
    for page in [1, 2] {
        if page == 2 {
            assert!(state.set_base_page(2));
        }
        let request = state.begin_bases().unwrap();
        let result = client.shiju_knowledge_bases(&credential, &state.bases.query).await;
        assert_eq!(state.finish_bases(&request, result), Some(false));
        assert_eq!(state.bases.items.len(), if page == 1 { 20 } else { 1 });
        let bases = state.bases.items.clone();
        for base in bases {
            assert!(state.select_base(base.kb_zid.clone()));
            for doc_page in [1, 2] {
                if doc_page == 2 {
                    assert!(state.documents.set_page(2));
                }
                let request = state.begin_documents().unwrap();
                let result = client
                    .shiju_knowledge_documents(&credential, &base.kb_zid, &state.documents.query)
                    .await;
                assert_eq!(state.finish_documents(&request, result), Some(false));
                assert_eq!(state.documents.items.len(), if doc_page == 1 { 20 } else { 1 });
            }
        }
    }
    // 上游总数收缩：保留第 2 页查询，收到响应后纠正并补取第 1 页。
    state.documents.query.q = "收缩".into();
    for correction in [true, false] {
        let request = state.begin_documents().unwrap();
        let result =
            client.shiju_knowledge_documents(&credential, "kb-20", &state.documents.query).await;
        assert_eq!(state.finish_documents(&request, result), Some(correction));
    }
    assert_eq!(state.documents.query.page, 1);
    state.search_documents("中文".into());
    assert_eq!(state.documents.query.page, 1);
    let request = state.begin_search("正文".into()).unwrap();
    state.finish_search(
        &request,
        client
            .shiju_knowledge_search(
                &credential,
                &ShijuKnowledgeSearchRequest {
                    query: "正文".into(),
                    kb_zids: vec!["kb-20".into()],
                    doc_zids: vec![],
                    top_k: 5,
                },
            )
            .await,
    );
    assert!(state.open_reader("kb-20".into(), "doc-20".into()));
    for offset in [0, 6, 6] {
        let request = state.begin_read().unwrap();
        let result = client
            .shiju_knowledge_read(
                &credential,
                &ShijuKnowledgeReadRequest {
                    kb_zid: Some("kb-20".into()),
                    doc_zid: "doc-20".into(),
                    offset,
                    limit_chars: 16000,
                },
            )
            .await;
        state.finish_read(&request, result);
        if offset == 0 || matches!(state.reader.as_ref().unwrap().status, LoadState::Error(_)) {
            assert_eq!(state.reader.as_ref().unwrap().content, "第一段中文。");
            assert_eq!(state.reader.as_ref().unwrap().next_offset, Some(6));
        }
    }
    assert_eq!(state.reader.as_ref().unwrap().content, "第一段中文。第二段中文。");
    let requests = requests.lock().unwrap();
    assert_eq!(requests.iter().filter(|(path, _, _)| path.ends_with("documents")).count(), 44);
    let offsets = requests
        .iter()
        .filter(|(path, _, _)| path.ends_with("read"))
        .map(|(_, _, body)| body["offset"].as_u64().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(offsets, [0, 6, 6]);
}

#[tokio::test(flavor = "multi_thread")]
async fn shiju_knowledge_reader_search_submits_draft_and_ignores_edited_query_response() {
    let mut app = app();
    let state = &mut app.expert_hub.knowledge;
    let old = state.begin_search("旧查询".into()).unwrap();
    state.edit_search_query("中文查询".into());
    state.finish_search(&old, Err("stale error".into()));
    assert!(matches!(state.search, LoadState::Idle));
    let (_, messages) = inspect(super::view(&app), Theme::Light, 1100.0, Some(("检索", 0))).await;
    assert!(matches!(action(&messages),KnowledgeMessage::Search(query) if query == "中文查询"));
    let state = &mut app.expert_hub.knowledge;
    let request = state.begin_search("中文查询".into()).unwrap();
    let (_, messages) = inspect(super::view(&app), Theme::Dark, 1100.0, Some(("检索", 0))).await;
    assert!(messages.is_empty());
    app.expert_hub.knowledge.finish_search(&request, Err("offline".into()));
    let (texts, messages) =
        inspect(super::view(&app), Theme::Dark, 1100.0, Some(("重试", 0))).await;
    assert!(texts.contains(&"检索失败，请重试".into()));
    assert!(matches!(action(&messages),KnowledgeMessage::Search(query) if query == "中文查询"));
    let request = app.expert_hub.knowledge.begin_search("中文查询".into()).unwrap();
    app.expert_hub
        .knowledge
        .finish_search(&request, Ok(ShijuKnowledgeSearchDto { records: vec![], total: 0 }));
    let (texts, _) = inspect(super::view(&app), Theme::Light, 1100.0, None).await;
    assert!(texts.contains(&"暂无检索结果".into()));
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
    assert_eq!(app.active_toast.as_ref().unwrap().message, "检索失败，请重试");
    app.active_toast = None;
    let _ = update(&mut app, KnowledgeMessage::SearchLoaded(search, Err("duplicate".into())));
    assert!(app.active_toast.is_none());
    let _ = update(&mut app, KnowledgeMessage::ReadLoaded(read.clone(), Err("offline".into())));
    assert_eq!(app.active_toast.as_ref().unwrap().message, "原文加载失败，请重试");
    app.active_toast = None;
    let _ = update(&mut app, KnowledgeMessage::ReadLoaded(read, Err("duplicate".into())));
    assert!(app.active_toast.is_none());
}
