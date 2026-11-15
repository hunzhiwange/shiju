use super::{GatewayClient, test_support};
use serde_json::json;
use vw_api_app_aggregation::*;
fn scope() -> AggregationQuery {
    AggregationQuery { target_host_id: "host & one".into(), project_id: "project/one".into() }
}
fn settings() -> serde_json::Value {
    json!({"revision":"settings-r1","requested":{"subagentModelId":"saved-model","subagentEffort":"high","autoReviewModelId":null,"classicTools":true},"effective":{"subagentModelId":null,"subagentEffort":null,"autoReviewModelId":null,"classicTools":false},"choices":[],"notices":[]})
}
#[tokio::test]
async fn aggregation_t52_aggregation_codex_api_preserves_contract() {
    let server = test_support::server(vec![(200, settings())]);
    let result = server.client().aggregation_codex_settings(&scope()).await;
    assert!(result.is_ok(), "settings must reach the scoped gateway: {result:?}");
    assert_eq!(serde_json::to_value(result.unwrap()).unwrap(), settings());
    let request = server.take_request();
    assert_eq!(request.method, "GET");
    let url = reqwest::Url::parse(&format!("{}{}", server.base_url(), request.path)).unwrap();
    assert_eq!(url.path(), "/v1/agent-apps/codex/aggregation/codex/settings");
    let query = url.query_pairs().collect::<std::collections::BTreeMap<_, _>>();
    assert_eq!(query["targetHostId"], scope().target_host_id);
    assert_eq!(query["projectId"], scope().project_id.as_ref());
    server.join();
}
