//! 真实穿过账户路由、凭据提取和知识库适配的本地 HTTP 验收。
use crate::shiju::account::ShijuAccountClient;
use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use serde_json::{Value, json};
use std::{
    io::{Read, Write},
    net::TcpListener,
    thread,
};
use tower::ServiceExt;
use vw_gateway_client::SHIJU_CREDENTIAL_HEADER;

fn app(base: &str) -> Router {
    super::shiju_account::router_with_client(
        ShijuAccountClient::with_base_urls(base, &format!("{base}/sj/")).unwrap(),
    )
}
fn fixture(responses: Vec<(String, String, String)>) -> (Router, thread::JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let server = thread::spawn(move || {
        responses.into_iter().map(|(status, mime, body)| {
        let (mut stream, _) = listener.accept().unwrap();
        stream.set_read_timeout(Some(std::time::Duration::from_secs(5))).unwrap();
        let mut bytes = Vec::new();
        loop {
            let mut buf = [0; 2048];
            let n = stream.read(&mut buf).unwrap();
            assert!(n > 0);
            bytes.extend_from_slice(&buf[..n]);
            if let Some(end) = bytes.windows(4).position(|v| v == b"\r\n\r\n") {
                let headers = String::from_utf8_lossy(&bytes[..end]);
                let length = headers.lines().find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    name.eq_ignore_ascii_case("content-length").then(|| value.trim().parse::<usize>().unwrap())
                }).unwrap_or(0);
                if bytes.len() >= end + 4 + length { break; }
            }
        }
        write!(stream, "HTTP/1.1 {status}\r\nContent-Type: {mime}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
        String::from_utf8(bytes).unwrap()
    }).collect()
    });
    (app(&base), server)
}
fn ok(data: Value) -> (String, String, String) {
    ("200 OK".into(), "application/json".into(), json!({"code":0,"data":data}).to_string())
}
async fn call(
    app: &Router,
    path: &str,
    body: Option<Value>,
    credential: bool,
) -> (StatusCode, Value) {
    let mut request =
        Request::builder().uri(path).method(if body.is_some() { "POST" } else { "GET" });
    if credential {
        request = request.header(SHIJU_CREDENTIAL_HEADER, "fixture-key");
    }
    let request = if let Some(body) = body {
        request.header("content-type", "application/json").body(Body::from(body.to_string()))
    } else {
        request.body(Body::empty())
    }
    .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
}
#[tokio::test]
async fn shiju_knowledge_gateway_forwards_page_and_scope() {
    let (app, server) = fixture(vec![
        ok(json!({"items":[],"page":1,"pageSize":3,"total":8})),
        ok(json!({"items":[],"page":2,"pageSize":3,"total":8})),
        ok(
            json!({"items":[{"doc_zid":"doc-1","kb_zid":"kb_1","title":"资料","lifecycle_status":"ready"}],"page":2,"pageSize":3,"total":8}),
        ),
        ok(
            json!({"records":[{"doc_zid":"doc-1","kb_zid":"kb_1","content":"片段","score":0.8}],"total":1}),
        ),
    ]);
    for page in [1, 2] {
        let (status, data) = call(
            &app,
            &format!("/shiju/knowledge/bases?page={page}&pageSize=3&q=%E4%B8%AD%E6%96%87%20%26%2B"),
            None,
            true,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(data["page"], page);
    }
    let (status, data) =
        call(&app, "/shiju/knowledge/documents?kb_zid=kb_1&page=2&pageSize=3&q=docs", None, true)
            .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(data["items"][0]["doc_zid"], "doc-1");
    let search = json!({"query":"中文","kb_zids":["kb_1"],"doc_zids":["doc-1"],"top_k":5});
    let (status, data) = call(&app, "/shiju/knowledge/search", Some(search.clone()), true).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(data["total"], 1);
    let requests = server.join().unwrap();
    for (index, raw) in requests.iter().enumerate() {
        assert!(raw.to_lowercase().contains("x-api-key: fixture-key"));
        let target = raw.split_whitespace().nth(1).unwrap();
        assert!(!target.contains("fixture-key"));
        let url = reqwest::Url::parse(&format!("http://localhost{target}")).unwrap();
        if index < 3 {
            let pairs: std::collections::HashMap<_, _> = url.query_pairs().into_owned().collect();
            assert_eq!(pairs["page"], if index == 0 { "1" } else { "2" });
            assert_eq!(pairs["pageSize"], "3");
            assert_eq!(pairs["q"], if index < 2 { "中文 &+" } else { "docs" });
            assert!(raw.starts_with("GET "));
            assert_eq!(
                url.path(),
                if index < 2 { "/sj/kb/api/v1/knowledge-bases" } else { "/sj/kb/api/v1/documents" }
            );
            if index == 2 {
                assert_eq!(pairs["kb_zid"], "kb_1");
            }
        } else {
            assert!(raw.starts_with("POST /sj/kb/api/v1/retrieval/query "));
            assert_eq!(
                serde_json::from_str::<Value>(raw.split("\r\n\r\n").nth(1).unwrap()).unwrap(),
                search
            );
        }
    }
}
#[tokio::test]
async fn shiju_knowledge_gateway_reads_first_and_next_segment() {
    let detail =
        json!({"doc_zid":"doc-1","kb_zid":"kb_1","title":"资料.md","lifecycle_status":"ready"});
    let markdown = |more, next, content| {
        (
            "200 OK".into(),
            "text/markdown".into(),
            format!(
                "---\ndoc_zid: doc-1\nkb_zid: kb_1\ntotal_chars: 4\nhas_more: {more}\nnext_offset: {next}\n---\n## 正文\n\n{content}"
            ),
        )
    };
    let (app, server) = fixture(vec![
        ok(detail.clone()),
        markdown(true, "2", "首段"),
        ok(detail),
        markdown(false, "null", "末段"),
    ]);
    for (offset, expected, next) in [(0, "首段", json!(2)), (2, "末段", Value::Null)] {
        let (status, data) = call(
            &app,
            "/shiju/knowledge/read",
            Some(json!({"doc_zid":"doc-1","kb_zid":"kb_1","offset":offset,"limit_chars":2})),
            true,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(data["content"], expected);
        assert_eq!(data["next_offset"], next);
    }
    let requests = server.join().unwrap();
    for (index, raw) in requests.iter().enumerate() {
        assert!(raw.to_lowercase().contains("x-api-key: fixture-key"));
        if index % 2 == 0 {
            assert!(raw.starts_with("GET /sj/kb/api/v1/documents/doc-1 "));
        } else {
            let url = reqwest::Url::parse(&format!(
                "http://localhost{}",
                raw.split_whitespace().nth(1).unwrap()
            ))
            .unwrap();
            assert_eq!(
                url.path(),
                "/sj/kb/api/v1/agent/documents/by-filename/%E8%B5%84%E6%96%99.md/content.md"
            );
            let pairs: std::collections::HashMap<_, _> = url.query_pairs().into_owned().collect();
            assert_eq!(pairs["offset"], if index == 1 { "0" } else { "2" });
            assert_eq!(pairs["limit_chars"], "2");
        }
    }
}
#[tokio::test]
async fn shiju_knowledge_gateway_rejects_invalid_input() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let app = app(&format!("http://{}", listener.local_addr().unwrap()));
    for (path, body) in [
        ("/shiju/knowledge/bases", None),
        ("/shiju/knowledge/documents?kb_zid=kb_1", None),
        ("/shiju/knowledge/search", Some(json!({"query":"x"}))),
        ("/shiju/knowledge/read", Some(json!({"doc_zid":"doc-1"}))),
    ] {
        assert_eq!(call(&app, path, body, false).await.0, StatusCode::BAD_REQUEST);
    }
    for path in [
        "/shiju/knowledge/bases?page=0",
        "/shiju/knowledge/bases?pageSize=101",
        "/shiju/knowledge/bases?page=-1",
        "/shiju/knowledge/bases?page=4294967296",
        "/shiju/knowledge/bases?page=no",
        "/shiju/knowledge/documents",
        "/shiju/knowledge/documents?kb_zid=../escape",
        "/shiju/knowledge/documents?kb_zid=kb_1&pageSize=0",
    ] {
        assert_eq!(call(&app, path, None, true).await.0, StatusCode::BAD_REQUEST, "{path}");
    }
    for (path, body) in [
        ("search", json!({"query":" "})),
        ("search", json!({"query":"x","top_k":21})),
        ("search", json!({"query":"x","kb_zids":["bad/id"]})),
        ("read", json!({"doc_zid":"../escape"})),
        ("read", json!({"doc_zid":"doc-1","limit_chars":0})),
        ("read", json!({"doc_zid":"doc-1","offset":-1})),
        ("read", json!({"doc_zid":"doc-1","offset":18446744073709551616_f64})),
        ("read", json!({"doc_zid":"doc-1","url":"http://localhost/secret"})),
    ] {
        assert_eq!(
            call(&app, &format!("/shiju/knowledge/{path}"), Some(body), true).await.0,
            StatusCode::BAD_REQUEST
        );
    }
    assert_eq!(listener.accept().unwrap_err().kind(), std::io::ErrorKind::WouldBlock);
}
#[tokio::test]
async fn shiju_knowledge_gateway_preserves_safe_status() {
    for (status, body, expected) in [
        ("401 Unauthorized", "fixture-key upstream-secret", StatusCode::UNAUTHORIZED),
        ("403 Forbidden", "fixture-key upstream-secret", StatusCode::FORBIDDEN),
        (
            "200 OK",
            r#"{"code":403,"message":"fixture-key upstream-secret"}"#,
            StatusCode::INTERNAL_SERVER_ERROR,
        ),
        ("200 OK", "malformed fixture-key upstream-secret", StatusCode::INTERNAL_SERVER_ERROR),
    ] {
        let (app, server) = fixture(vec![(status.into(), "application/json".into(), body.into())]);
        let (actual, body) = call(&app, "/shiju/knowledge/bases", None, true).await;
        assert_eq!(actual, expected);
        assert!(!body.to_string().contains("fixture-key"));
        assert!(!body.to_string().contains("upstream-secret"));
        server.join().unwrap();
    }
}
