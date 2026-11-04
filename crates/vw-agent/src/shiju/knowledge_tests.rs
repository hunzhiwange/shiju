//! 本地上游边界验证分页、归属、错误脱敏及禁止跳转。
use super::super::account_tests::{fixture, fixture_with_status};
use super::*;
use serde_json::{Value, json};

fn envelope(data: Value) -> String {
    json!({"code": 0, "data": data}).to_string()
}
fn page() -> Value {
    json!({"items": [{"kb_zid":"kb_1", "name":"资料", "status":"enabled"}],
        "page":2, "pageSize":3, "total":8})
}
fn document_page() -> Value {
    json!({"items": [{"kb_zid":"kb_1", "doc_zid":"doc-1", "title":"资料", "lifecycle_status":"ready"}],
        "page":2, "pageSize":3, "total":8})
}
fn query() -> ShijuKnowledgePageQuery {
    ShijuKnowledgePageQuery { page: 2, page_size: 3, q: "中文 &+/?=#".into() }
}
fn search() -> ShijuKnowledgeSearchRequest {
    ShijuKnowledgeSearchRequest {
        query: "中文 &+/?=#".into(),
        kb_zids: vec!["kb_1".into()],
        doc_zids: vec!["doc-1".into()],
        top_k: 5,
    }
}
fn results() -> Value {
    json!({"records":[{"kb_zid":"kb_1", "doc_zid":"doc-1", "content":"片段", "score":0.75}],"total":1})
}
fn client(base: &str) -> ShijuAccountClient {
    ShijuAccountClient::with_base_urls(base, &format!("{base}/sj/")).unwrap()
}

#[tokio::test]
async fn shiju_knowledge_lists_preserve_requested_page() {
    let (base, requests) = fixture(vec![envelope(page()), envelope(document_page())]);
    let client = client(&base);
    let bases = client.knowledge_bases("fixture-key", &query()).await.unwrap();
    let docs = client.knowledge_documents("fixture-key", "kb_1", &query()).await.unwrap();
    assert_eq!((bases.page, bases.page_size, bases.total), (2, 3, 8));
    assert_eq!(docs.items[0].doc_zid, "doc-1");
    let requests = requests.join().unwrap();
    assert_eq!(requests.len(), 2);
    for (request, path) in requests.iter().zip(["knowledge-bases", "documents"]) {
        let target = request.split_whitespace().nth(1).unwrap();
        let url = reqwest::Url::parse(&format!("{base}{target}")).unwrap();
        assert_eq!(url.path(), format!("/sj/kb/api/v1/{path}"));
        let pairs: std::collections::HashMap<_, _> = url.query_pairs().into_owned().collect();
        assert_eq!(pairs["page"], "2");
        assert_eq!(pairs["pageSize"], "3");
        assert_eq!(pairs["q"], query().q);
        if path == "documents" {
            assert_eq!(pairs["kb_zid"], "kb_1");
        }
        assert!(request.starts_with("GET "));
        assert!(request.to_lowercase().contains("x-api-key: fixture-key"));
        assert!(!target.contains("fixture-key"));
    }
}

#[tokio::test]
async fn shiju_knowledge_rejects_page_and_base_mismatch() {
    let mut invalid = Vec::new();
    for (key, value) in [
        ("page", json!(1)),
        ("pageSize", json!(4)),
        ("total", json!(-1)),
        ("page", json!(0)),
        ("pageSize", json!(101)),
    ] {
        let mut data = page();
        data[key] = value;
        invalid.push(envelope(data));
    }
    let mut data = page();
    data["items"][0]["kb_zid"] = json!("invalid/id");
    invalid.push(envelope(data));
    let mut wrong_document_page = document_page();
    wrong_document_page["page"] = json!(1);
    let (document_base, document_requests) = fixture(vec![envelope(wrong_document_page)]);
    assert!(
        client(&document_base).knowledge_documents("fixture-key", "kb_1", &query()).await.is_err()
    );
    document_requests.join().unwrap();
    let count = invalid.len();
    let (base, requests) = fixture(invalid);
    let account = client(&base);
    for _ in 0..count {
        assert!(account.knowledge_bases("fixture-key", &query()).await.is_err());
    }
    assert_eq!(requests.join().unwrap().len(), count);
    for (field, value) in [("kb_zid", "other"), ("doc_zid", "bad\n"), ("kb_zid", "")] {
        let mut data = document_page();
        data["items"][0][field] = json!(value);
        let (base, requests) = fixture(vec![envelope(data)]);
        assert!(client(&base).knowledge_documents("fixture-key", "kb_1", &query()).await.is_err());
        requests.join().unwrap();
    }
}

#[tokio::test]
async fn shiju_knowledge_search_validates_scope_and_errors() {
    let (base, requests) = fixture(vec![envelope(results())]);
    let result = client(&base).knowledge_search("fixture-key", &search()).await.unwrap();
    assert_eq!(result.records[0].score, 0.75);
    let requests = requests.join().unwrap();
    assert!(requests[0].starts_with("POST /sj/kb/api/v1/retrieval/query "));
    let body: Value = serde_json::from_str(requests[0].split("\r\n\r\n").nth(1).unwrap()).unwrap();
    assert_eq!(body, serde_json::to_value(search()).unwrap());
    assert!(requests[0].to_lowercase().contains("x-api-key: fixture-key"));
    for (field, value) in [("kb_zid", "other"), ("doc_zid", "other"), ("doc_zid", "bad/id")] {
        let mut data = results();
        data["records"][0][field] = json!(value);
        let (base, requests) = fixture(vec![envelope(data)]);
        assert!(client(&base).knowledge_search("fixture-key", &search()).await.is_err());
        requests.join().unwrap();
    }
}

#[tokio::test]
async fn shiju_knowledge_invalid_inputs_fail_before_http() {
    // 绑定但不接受请求：验证错误必须来自本地校验，且监听队列为空。
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let client = client(&format!("http://{}", listener.local_addr().unwrap()));
    for (page, page_size) in [(0, 20), (1, 0), (1, 101)] {
        let query = ShijuKnowledgePageQuery { page, page_size, q: String::new() };
        assert!(client.knowledge_bases("key", &query).await.is_err());
        assert!(client.knowledge_documents("key", "kb_1", &query).await.is_err());
    }
    for id in ["", "../escape", "中文", "kb\n"] {
        assert!(client.knowledge_documents("key", id, &query()).await.is_err());
    }
    for data in [
        json!({"query":" "}),
        json!({"query":"x","top_k":0}),
        json!({"query":"x","top_k":21}),
        json!({"query":"x","kb_zids":["bad/id"]}),
        json!({"query":"x","doc_zids":[""]}),
    ] {
        let request = serde_json::from_value(data).unwrap();
        assert!(client.knowledge_search("key", &request).await.is_err());
    }
    assert!(client.knowledge_bases(" ", &query()).await.is_err());
    assert!(client.knowledge_documents("", "kb_1", &query()).await.is_err());
    assert!(client.knowledge_search("", &search()).await.is_err());
    assert_eq!(listener.accept().unwrap_err().kind(), std::io::ErrorKind::WouldBlock);
}

#[tokio::test]
async fn shiju_knowledge_errors_are_safe_and_preserve_http_status() {
    for status in ["401 Unauthorized", "403 Forbidden", "500 Internal Server Error"] {
        let (base, requests) =
            fixture_with_status(vec![(status.into(), "upstream-secret fixture-key".into())]);
        let error = client(&base).knowledge_bases("fixture-key", &query()).await.unwrap_err();
        assert_eq!(error.status().unwrap().as_u16().to_string(), &status[..3]);
        assert!(!format!("{error:?}").contains("upstream-secret"));
        assert!(!error.to_string().contains("fixture-key"));
        requests.join().unwrap();
    }
    for body in [json!({"code":200,"data":results()}).to_string(),json!({"code":403,"message":"upstream-secret fixture-key"}).to_string(),"not json".into(),envelope(json!(null)),envelope(json!({"records":[],"total":-1})),envelope(json!({"records":[{"kb_zid":"kb_1","doc_zid":"doc-1","score":null}],"total":1})),"{\"code\":0,\"data\":{\"records\":[{\"kb_zid\":\"kb_1\",\"doc_zid\":\"doc-1\",\"score\":1e999}],\"total\":1}}".into()] {
        let (base, requests) = fixture(vec![body]);
        let error = client(&base).knowledge_search("fixture-key", &search()).await.unwrap_err();
        assert!(!format!("{error:?}").contains("upstream-secret"));
        assert!(!error.to_string().contains("fixture-key"));
        requests.join().unwrap();
    }
}

#[tokio::test]
async fn shiju_knowledge_redirect_is_not_followed() {
    let destination = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    destination.set_nonblocking(true).unwrap();
    let location = format!("http://{}/stolen", destination.local_addr().unwrap());
    let (base, requests) =
        fixture_with_status(vec![(format!("302 Found\r\nLocation: {location}"), String::new())]);
    assert!(client(&base).knowledge_bases("fixture-key", &query()).await.is_err());
    requests.join().unwrap();
    assert_eq!(destination.accept().unwrap_err().kind(), std::io::ErrorKind::WouldBlock);
}
