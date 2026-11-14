use super::search_api::MarketSearchError;
use crate::client::test_support;
use serde_json::json;
use vw_api_types::agent_apps_skills::search_api::*;

fn query() -> MarketSearchQuery {
    MarketSearchQuery { project_id: "project-a".into(), target_host_id: "host-a".into(),
        query: "中文 & rust".into(), limit: 20, offset: 3 }
}

#[tokio::test]
async fn cp12_search_api_success() {
    let body = json!({"query":"中文 & rust","totalCount":123,"skills":[{
        "key":"owner/repo/tool","name":"工具","directory":"tool",
        "repoOwner":"owner","repoName":"repo","repoBranch":"main",
        "installs":9876543210u64,"readmeUrl":"https://github.com/owner/repo"}]});
    let server = test_support::server(vec![(200, body.clone())]);
    let result = server.client().agent_apps_market_search(&query(), "test-token").await.unwrap();
    assert_eq!(serde_json::to_value(result).unwrap(), body);
    let request = server.take_request();
    assert_eq!(request.method, "GET");
    assert_eq!(request.header("x-agent-apps-token"), Some("test-token"));
    server.join();
}

#[tokio::test]
async fn cp12_search_api_preserves_structured_failures() {
    let failures = [MarketSearchFailure::Forbidden, MarketSearchFailure::NotFound,
        MarketSearchFailure::RateLimited, MarketSearchFailure::Timeout,
        MarketSearchFailure::Transport, MarketSearchFailure::InvalidResponse,
        MarketSearchFailure::HttpStatus { status: 503 }];
    let server = test_support::server(failures.iter()
        .map(|failure| (502, serde_json::to_value(failure).unwrap())).collect());
    for failure in failures {
        assert_eq!(server.client().agent_apps_market_search(&query(), "test-token").await,
            Err(MarketSearchError::Http { status: 502, failure }));
        server.take_request();
    }
    server.join();
}
