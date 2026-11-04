use super::test_support;
use crate::*;
use serde_json::json;

#[tokio::test]
async fn knowledge_client_preserves_page_query_and_credentials() {
    let server = test_support::server(vec![
        (
            200,
            json!({"items":[{"kb_zid":"kb-1","name":"资料","status":"enabled"}],"page":3,"pageSize":7,"total":30}),
        ),
        (
            200,
            json!({"items":[{"doc_zid":"doc_2","kb_zid":"kb-1","title":"文档","lifecycle_status":"ready"}],"page":3,"pageSize":7,"total":18}),
        ),
    ]);
    let credential = ShijuCredential::new("private-knowledge-key").unwrap();
    let query = ShijuKnowledgePageQuery { page: 3, page_size: 7, q: "资料 &+?".into() };
    let bases = server.client().shiju_knowledge_bases(&credential, &query).await.unwrap();
    assert_eq!((bases.page, bases.page_size, bases.total), (3, 7, 30));
    assert_eq!(bases.items[0].kb_zid, "kb-1");
    let docs =
        server.client().shiju_knowledge_documents(&credential, "kb-1", &query).await.unwrap();
    assert_eq!(docs.items[0].doc_zid, "doc_2");
    assert_eq!((docs.page, docs.page_size, docs.total), (3, 7, 18));
    for route in ["bases", "documents"] {
        let recorded = server.take_request();
        assert_eq!(recorded.method, "GET");
        let url = reqwest::Url::parse(&format!("http://localhost{}", recorded.path)).unwrap();
        assert_eq!(url.path(), format!("/v1/shiju/knowledge/{route}"));
        let pairs = url.query_pairs().collect::<std::collections::BTreeMap<_, _>>();
        assert_eq!(pairs["page"], "3");
        assert_eq!(pairs["pageSize"], "7");
        assert_eq!(pairs["q"], query.q);
        assert_eq!(pairs.len(), if route == "documents" { 4 } else { 3 });
        if route == "documents" {
            assert_eq!(pairs["kb_zid"], "kb-1");
        }
        assert_eq!(recorded.header(SHIJU_CREDENTIAL_HEADER), Some(credential.expose()));
        assert!(!recorded.path.contains(credential.expose()));
        assert!(recorded.body.is_null());
    }
    server.join();
}

#[tokio::test]
async fn knowledge_client_preserves_search_and_read_requests() {
    let server = test_support::server(vec![
        (
            200,
            json!({"records":[{"kb_zid":"kb-1","doc_zid":"doc_2","title":"私有标题","content":"私有正文","score":0.9}],"total":1}),
        ),
        (
            200,
            json!({"kb_zid":"kb-1","doc_zid":"doc_2","title":"私有标题","content":"私有正文","total_chars":50000,"next_offset":32123}),
        ),
    ]);
    let credential = ShijuCredential::new("private-knowledge-key").unwrap();
    let search = ShijuKnowledgeSearchRequest {
        query: "私有检索 &?".into(),
        kb_zids: vec!["kb-1".into()],
        doc_zids: vec!["doc_2".into()],
        top_k: 9,
    };
    let read = ShijuKnowledgeReadRequest {
        doc_zid: "doc_2".into(),
        kb_zid: Some("kb-1".into()),
        offset: 123,
        limit_chars: 32000,
    };
    let results = server.client().shiju_knowledge_search(&credential, &search).await.unwrap();
    assert_eq!(results.total, 1);
    assert_eq!(results.records[0].score, 0.9);
    assert_eq!(results.records[0].content, "私有正文");
    let content = server.client().shiju_knowledge_read(&credential, &read).await.unwrap();
    assert_eq!(content.next_offset, Some(32123));
    assert_eq!(content.total_chars, 50000);
    assert_eq!(content.doc_zid, "doc_2");
    assert_eq!(content.kb_zid, "kb-1");
    assert_eq!(content.content, "私有正文");
    for (route, body) in [
        ("search", serde_json::to_value(&search).unwrap()),
        ("read", serde_json::to_value(&read).unwrap()),
    ] {
        let recorded = server.take_request();
        assert_eq!(recorded.method, "POST");
        assert_eq!(recorded.path, format!("/v1/shiju/knowledge/{route}"));
        assert_eq!(recorded.body, body);
        assert_eq!(recorded.header(SHIJU_CREDENTIAL_HEADER), Some(credential.expose()));
    }
    let debug = format!("{search:?} {read:?} {results:?} {content:?}");
    for private in ["私有检索", "私有正文", "私有标题", "doc_2", "kb-1"] {
        assert!(!debug.contains(private));
    }
    server.join();
}

#[test]
fn knowledge_page_rejects_invalid_bounds() {
    let default: ShijuKnowledgePageQuery = serde_json::from_value(json!({})).unwrap();
    assert_eq!((default.page, default.page_size), (1, 20));
    assert_eq!(serde_json::to_value(&default).unwrap()["pageSize"], 20);
    for (page, page_size) in [(0, 20), (1, 0), (1, 101)] {
        assert!(ShijuKnowledgePageQuery { page, page_size, q: String::new() }.validate().is_err());
    }
    for (page, page_size) in [(1, 1), (u32::MAX, 100)] {
        assert!(ShijuKnowledgePageQuery { page, page_size, q: String::new() }.validate().is_ok());
    }
    assert!(serde_json::from_value::<ShijuKnowledgePageQuery>(json!({"page":-1})).is_err());
    assert!(serde_json::from_value::<ShijuKnowledgePageQuery>(json!({"pageSize":1.5})).is_err());
}

#[test]
fn knowledge_requests_reject_invalid_search_read_and_identifiers() {
    let mut search: ShijuKnowledgeSearchRequest =
        serde_json::from_value(json!({"query":"内容"})).unwrap();
    assert_eq!(search.top_k, 5);
    assert!(search.validate().is_ok());
    for query in ["", " \n\t"] {
        search.query = query.into();
        assert!(search.validate().is_err());
    }
    search.query = "内容".into();
    for k in [0, 21] {
        search.top_k = k;
        assert!(search.validate().is_err());
    }
    search.top_k = 20;
    let mut read: ShijuKnowledgeReadRequest =
        serde_json::from_value(json!({"doc_zid":"doc_2"})).unwrap();
    assert_eq!((read.offset, read.limit_chars), (0, 16000));
    assert!(read.validate().is_ok());
    for id in ["", "a/b", "a?b", "中文", "a\n", "a b", "a.b"] {
        search.kb_zids = vec![id.into()];
        assert!(search.validate().is_err());
        search.kb_zids.clear();
        search.doc_zids = vec![id.into()];
        assert!(search.validate().is_err());
        search.doc_zids.clear();
        read.doc_zid = id.into();
        assert!(read.validate().is_err());
        read.doc_zid = "doc_2".into();
        read.kb_zid = Some(id.into());
        assert!(read.validate().is_err());
        read.kb_zid = None;
    }
    for limit in [0, 32001] {
        read.limit_chars = limit;
        assert!(read.validate().is_err());
    }
    read.limit_chars = 32000;
    read.offset = u64::MAX;
    assert!(read.validate().is_ok());
    assert!(
        serde_json::from_value::<ShijuKnowledgeReadRequest>(json!({"doc_zid":"doc_2","offset":-1}))
            .is_err()
    );
    assert!(
        serde_json::from_value::<ShijuKnowledgeReadRequest>(
            json!({"doc_zid":"doc_2","cursor":"1"})
        )
        .is_err()
    );
}

#[tokio::test]
async fn knowledge_client_rejects_invalid_requests_before_transport() {
    let server = test_support::server(vec![]);
    let credential = ShijuCredential::new("private-knowledge-key").unwrap();
    let page = ShijuKnowledgePageQuery { page: 0, ..Default::default() };
    assert_eq!(
        server.client().shiju_knowledge_bases(&credential, &page).await.unwrap_err(),
        "invalid_knowledge_page"
    );
    assert_eq!(
        server.client().shiju_knowledge_documents(&credential, "kb-1", &page).await.unwrap_err(),
        "invalid_knowledge_page"
    );
    for id in ["", "kb/1", "kb\n"] {
        assert_eq!(
            server
                .client()
                .shiju_knowledge_documents(&credential, id, &ShijuKnowledgePageQuery::default())
                .await
                .unwrap_err(),
            "invalid_knowledge_id"
        );
    }
    let search = serde_json::from_value(json!({"query":" "})).unwrap();
    assert_eq!(
        server.client().shiju_knowledge_search(&credential, &search).await.unwrap_err(),
        "empty_knowledge_query"
    );
    let read = serde_json::from_value(json!({"doc_zid":"doc_2", "limit_chars":0})).unwrap();
    assert_eq!(
        server.client().shiju_knowledge_read(&credential, &read).await.unwrap_err(),
        "invalid_knowledge_limit_chars"
    );
    server.join();
}
