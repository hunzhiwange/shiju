use std::time::Duration;

use serde_json::{Value, json};
use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::TcpListener};

use super::market_search::{MarketSearchError, search_at};

const TIMEOUT: Duration = Duration::from_secs(2);

fn skill(id: &str, source: &str, installs: u64) -> Value {
    json!({"id": id, "skillId": id, "name": id, "source": source, "installs": installs})
}

async fn serve_once(
    status: u16,
    body: String,
) -> (String, tokio::task::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let task = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut request = Vec::new();
        loop {
            let mut buffer = [0; 1024];
            let size = stream.read(&mut buffer).await.unwrap();
            if size == 0 {
                break;
            }
            request.extend_from_slice(&buffer[..size]);
            if request.windows(4).any(|part| part == b"\r\n\r\n") {
                break;
            }
        }
        let response = format!(
            "HTTP/1.1 {status} Fixture\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len(),
        );
        // A rejected oversized response may close before the fixture finishes.
        let _ = stream.write_all(response.as_bytes()).await;
        String::from_utf8(request).unwrap()
    });
    (format!("http://{address}/api/search"), task)
}

#[tokio::test]
async fn cp12_market_search_success() {
    let body = json!({
        "query": "rust & 中文", "count": 97, "searchType": "search", "duration_ms": 5,
        "skills": [skill("first", "owner/repo.tools", 2), skill("second", "org/repo", 900)]
    }).to_string();
    let (endpoint, server) = serve_once(200, body).await;
    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    let result = search_at(&client, &endpoint, "rust & 中文", 12, 24, TIMEOUT).await.unwrap();
    let request = server.await.unwrap();
    let target = request.lines().next().unwrap().split_whitespace().nth(1).unwrap();
    let url = reqwest::Url::parse(&format!("http://localhost{target}")).unwrap();
    let pairs: std::collections::HashMap<_, _> = url.query_pairs().into_owned().collect();
    assert_eq!(pairs.get("q").unwrap(), "rust & 中文");
    assert_eq!(pairs.get("limit").unwrap(), "12");
    assert_eq!(pairs.get("offset").unwrap(), "24");
    assert_eq!(result.query, "rust & 中文");
    assert_eq!(result.total_count, 97);
    assert_eq!(result.skills.len(), 2);
    assert_eq!(result.skills[0].key, "first");
    assert_eq!(result.skills[0].directory, "first");
    assert_eq!(result.skills[0].repo_owner, "owner");
    assert_eq!(result.skills[0].repo_name, "repo.tools");
    assert_eq!(result.skills[0].repo_branch, "main");
    assert_eq!(result.skills[0].installs, 2);
    assert_eq!(result.skills[1].installs, 900);
    assert_eq!(result.skills[0].readme_url.as_deref(), Some("https://github.com/owner/repo.tools"));
    let serialized = serde_json::to_value(result).unwrap();
    assert_eq!(serialized["totalCount"], 97);
    assert_eq!(serialized["skills"][0]["repoOwner"], "owner");
}

#[tokio::test]
async fn cp12_market_search_rejects_without_side_effects() {
    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    // Search accepts no store or installation path and only issues one GET.
    // The fixture contains a deliberately sensitive upstream error body.
    for (status, expected) in [
        (403, MarketSearchError::Forbidden),
        (404, MarketSearchError::NotFound),
        (429, MarketSearchError::RateLimited),
        (500, MarketSearchError::HttpStatus { status: 500 }),
        (200, MarketSearchError::InvalidResponse),
    ] {
        let (endpoint, server) = serve_once(status, "secret-token invalid-json".to_owned()).await;
        let error = search_at(&client, &endpoint, "private-query", 10, 0, TIMEOUT).await.unwrap_err();
        assert_eq!(error, expected);
        assert!(!error.to_string().contains("secret-token"));
        let serialized = serde_json::to_string(&error).unwrap();
        assert!(!serialized.contains("private-query"));
        assert!(!serialized.contains("secret-token"));
        let request = server.await.unwrap();
        assert!(request.starts_with("GET /api/search?"));
    }
}

#[tokio::test]
async fn cp12_market_search_filters_unsafe_sources_and_preserves_total() {
    let mut skills = vec![skill("safe", "org/repo.with.dots", u64::MAX)];
    for source in ["host.example/repo", "org/a/b", "org/..", "org/a%2fb", "org/", "/repo"] {
        skills.push(skill("ignored", source, 1));
    }
    for directory in ["../escape", "..", "/absolute", "a\\b", " bad", "bad\n"] {
        skills.push(skill(directory, "org/repo", 1));
    }
    let body = json!({"query": "q", "count": 30, "skills": skills}).to_string();
    let (endpoint, server) = serve_once(200, body).await;
    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    let result = search_at(&client, &endpoint, "q", 20, 0, TIMEOUT).await.unwrap();
    server.await.unwrap();
    assert_eq!(result.total_count, 30);
    assert_eq!(result.skills.len(), 1);
    assert_eq!(result.skills[0].installs, u64::MAX);
}

#[tokio::test]
async fn cp12_market_search_rejects_invalid_schema_and_large_body() {
    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    let mut invalid_installs = skill("bad", "org/repo", 1);
    invalid_installs["installs"] = json!(-1);
    for body in [
        "{}".to_owned(),
        json!({"query": "q", "count": 1, "skills": [invalid_installs]}).to_string(),
        "x".repeat(2 * 1024 * 1024 + 1),
    ] {
        let (endpoint, server) = serve_once(200, body).await;
        let result = search_at(&client, &endpoint, "q", 10, 0, TIMEOUT).await;
        assert_eq!(result, Err(MarketSearchError::InvalidResponse));
        // The client may close early when rejecting Content-Length.
        server.abort();
        let _ = server.await;
    }
}

#[tokio::test]
async fn cp12_market_search_distinguishes_timeout_and_transport() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}/api/search", listener.local_addr().unwrap());
    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    // A bound listener that never responds times out without any external host.
    let result = search_at(&client, &endpoint, "q", 10, 0, Duration::from_millis(30)).await;
    assert_eq!(result, Err(MarketSearchError::Timeout));
    drop(listener);
    assert_eq!(
        search_at(&client, "http://[", "q", 10, 0, TIMEOUT).await,
        Err(MarketSearchError::Transport),
    );
}
