use super::*;
use std::io::{Read, Write};
use vw_gateway_client::ShijuKnowledgeReadRequest;

fn request(offset: u64) -> ShijuKnowledgeReadRequest {
    ShijuKnowledgeReadRequest {
        doc_zid: "doc-1".into(),
        kb_zid: Some("kb_1".into()),
        offset,
        limit_chars: 2,
    }
}
fn detail() -> String {
    serde_json::json!({"code":0,"data":{"doc_zid":"doc-1","kb_zid":"kb_1",
        "title":"资料 /?+#%.md","original_filename":"fallback.md","lifecycle_status":"ready"}})
    .to_string()
}
fn page(meta: &str, body: &str) -> Vec<u8> {
    format!("---\ndoc_zid: doc-1\nkb_zid: kb_1\n{meta}\n---\n# 文档\n\n## 正文\n\n{body}")
        .into_bytes()
}
fn fixture(
    responses: Vec<(&str, Vec<u8>)>,
) -> (ShijuAccountClient, std::thread::JoinHandle<Vec<String>>) {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let responses: Vec<_> =
        responses.into_iter().map(|(mime, body)| (mime.to_string(), body)).collect();
    let handle = std::thread::spawn(move || {
        responses.into_iter().map(|(mime, body)| {
        let (mut stream, _) = listener.accept().unwrap();
        stream.set_write_timeout(Some(std::time::Duration::from_secs(2))).unwrap();
        let mut bytes = Vec::new();
        while !bytes.windows(4).any(|v| v == b"\r\n\r\n") {
            let mut buf = [0; 2048];
            let n = stream.read(&mut buf).unwrap();
            assert!(n > 0);
            bytes.extend_from_slice(&buf[..n]);
        }
        let header = format!("HTTP/1.1 200 OK\r\nContent-Type: {mime}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",body.len());
        let _ = stream.write_all(header.as_bytes()).and_then(|_| stream.write_all(&body));
        String::from_utf8(bytes).unwrap()
    }).collect()
    });
    (ShijuAccountClient::with_base_urls(&base, &format!("{base}/sj/")).unwrap(), handle)
}
#[tokio::test]
async fn shiju_knowledge_content_reads_next_offset() {
    let first = page(
        "total_chars: 5\nhas_more: true\nnext_offset: 3",
        "中文🙂\n## 续读\n\n继续\n## 操作指引\n\n隐藏",
    );
    let last = page(
        "total_chars: 5\nhas_more: false\nnext_offset: null",
        "末段\n\n第二段\n## 操作指引\n\n隐藏",
    );
    let (client, server) = fixture(vec![
        ("application/json", detail().into_bytes()),
        ("text/markdown; charset=utf-8", first),
        ("application/json", detail().into_bytes()),
        ("text/markdown", last),
    ]);
    let a = client.knowledge_read("fixture-key", &request(0)).await.unwrap();
    assert_eq!(a.content, "中文🙂");
    assert_eq!(a.next_offset, Some(3));
    assert_eq!(a.total_chars, 5);
    let b = client.knowledge_read("fixture-key", &request(a.next_offset.unwrap())).await.unwrap();
    assert_eq!(b.content, "末段\n\n第二段");
    assert_eq!(b.next_offset, None);
    let requests = server.join().unwrap();
    assert_eq!(requests.len(), 4);
    assert!(requests[0].starts_with("GET /sj/kb/api/v1/documents/doc-1 "));
    for (index, offset) in [(1, "0"), (3, "3")] {
        let target = requests[index].split_whitespace().nth(1).unwrap();
        let url = reqwest::Url::parse(&format!("http://localhost{target}")).unwrap();
        assert_eq!(
            url.path(),
            "/sj/kb/api/v1/agent/documents/by-filename/%E8%B5%84%E6%96%99%20%2F%3F+%23%25.md/content.md"
        );
        assert_eq!(url.query(), Some(format!("offset={offset}&limit_chars=2").as_str()));
        assert!(requests[index].to_lowercase().contains("x-api-key: fixture-key"));
    }
}
#[tokio::test]
async fn shiju_knowledge_content_rejects_identity_mismatch() {
    for (field, value) in [("doc_zid", "other"), ("kb_zid", "other")] {
        let mut doc: serde_json::Value = serde_json::from_str(&detail()).unwrap();
        doc["data"][field] = value.into();
        let (client, server) = fixture(vec![("application/json", doc.to_string().into_bytes())]);
        assert!(client.knowledge_read("key", &request(0)).await.is_err());
        server.join().unwrap();
        let raw =
            String::from_utf8(page("total_chars: 1\nhas_more: false", "secret")).unwrap().replace(
                &format!("{field}: {}", if field == "doc_zid" { "doc-1" } else { "kb_1" }),
                &format!("{field}: {value}"),
            );
        rejects("text/markdown", raw.into_bytes(), 0).await;
    }
}
async fn rejects(mime: &str, body: Vec<u8>, offset: u64) {
    let (client, server) = fixture(vec![("application/json", detail().into_bytes()), (mime, body)]);
    let error = client.knowledge_read("key", &request(offset)).await.unwrap_err();
    assert!(!error.to_string().contains("secret"));
    server.join().unwrap();
}
#[tokio::test]
async fn shiju_knowledge_content_rejects_invalid_continuation() {
    for meta in [
        "total_chars: 5\nhas_more: true\nnext_offset: 3",
        "total_chars: 5\nhas_more: true\nnext_offset: 6",
        "total_chars: 5\nhas_more: true",
        "total_chars: -1\nhas_more: false",
        "total_chars: 5\nhas_more: true\nnext_offset: -1",
        "total_chars: 2\nhas_more: false",
    ] {
        rejects("text/markdown", page(meta, "secret"), 3).await;
    }
}
#[tokio::test]
async fn shiju_knowledge_content_rejects_malformed_protocol() {
    let meta = "total_chars: 5\nhas_more: false";
    rejects("application/json", page(meta, "secret"), 0).await;
    rejects("text/markdown", vec![0xff], 0).await;
    rejects("text/markdown", page("total_chars: [broken", "secret"), 0).await;
    rejects("text/markdown", "## 正文\n\nsecret".as_bytes().to_vec(), 0).await;
    for body in [
        "secret\n## 操作指引\n\nx\n## 操作指引\n\ny",
        "secret\n## 续读\n\nunexpected",
        "secret\n## 操作指引 \n\nhidden",
        "secret\n## 操作指引\nhidden",
    ] {
        rejects("text/markdown", page(meta, body), 0).await;
    }
    rejects("text/markdown", vec![b'x'; 1024 * 1024 + 1], 0).await;
}

#[tokio::test]
async fn shiju_knowledge_content_preserves_document_body_headings() {
    let content = "# 文档\n\n## 摘要\n\n中文摘要\n\n## 正文\n\n中文正文\n\n```text\n## 正文\n```";
    let (client, server) = fixture(vec![
        ("application/json", detail().into_bytes()),
        (
            "text/markdown",
            page("total_chars: 100\nhas_more: false", &format!("{content}\n\n## 操作指引\n\n隐藏")),
        ),
    ]);
    let result = client.knowledge_read("key", &request(0)).await.unwrap();
    assert_eq!(result.content, content);
    server.join().unwrap();
}
#[tokio::test]
async fn shiju_knowledge_content_accepts_crlf_and_empty_body() {
    for body in ["", "第一段\n\n第二段"] {
        let raw = String::from_utf8(page(
            "total_chars: 5\nhas_more: false\nnext_offset: 4",
            &format!("{body}\n## 操作指引\n\n隐藏"),
        ))
        .unwrap()
        .replace('\n', "\r\n");
        let (client, server) = fixture(vec![
            ("application/json", detail().into_bytes()),
            ("text/markdown", raw.into_bytes()),
        ]);
        let result = client.knowledge_read("key", &request(0)).await.unwrap();
        assert_eq!(result.content, body);
        assert_eq!(result.next_offset, None);
        server.join().unwrap();
    }
}

#[tokio::test]
async fn shiju_knowledge_content_invalid_inputs_fail_before_http() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let client = ShijuAccountClient::with_base_urls(&base, &base).unwrap();
    for (field, value) in [
        ("doc_zid", serde_json::json!("bad/id")),
        ("kb_zid", serde_json::json!("bad/id")),
        ("limit_chars", serde_json::json!(0)),
        ("limit_chars", serde_json::json!(32001)),
    ] {
        let mut data = serde_json::to_value(request(0)).unwrap();
        data[field] = value;
        let invalid = serde_json::from_value(data).unwrap();
        assert!(client.knowledge_read("key", &invalid).await.is_err());
    }
    assert!(client.knowledge_read(" ", &request(0)).await.is_err());
    assert_eq!(listener.accept().unwrap_err().kind(), std::io::ErrorKind::WouldBlock);
}

#[tokio::test]
async fn shiju_knowledge_content_falls_back_to_original_filename() {
    let mut doc: serde_json::Value = serde_json::from_str(&detail()).unwrap();
    doc["data"]["title"] = "".into();
    let (client, server) = fixture(vec![
        ("application/json", doc.to_string().into_bytes()),
        ("text/markdown", page("total_chars: 0\nhas_more: false", "")),
    ]);
    let mut input = request(0);
    input.kb_zid = None;
    let result = client.knowledge_read("key", &input).await.unwrap();
    assert_eq!(result.title, "fallback.md");
    assert_eq!(result.content, "");
    assert!(server.join().unwrap()[1].contains("/by-filename/fallback.md/content.md?"));
}
