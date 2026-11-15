use super::test_support;
use serde_json::json;

#[tokio::test]
async fn cc_mcp_t41_existing_mcp_inspection_and_bound_call_keep_their_routes() {
    let server = test_support::server(vec![
        (
            200,
            json!({"servers":[{"name":"docs","transport":"http","enabled":true,"required":false,"connection_status":"connected","tool_count":0,"resource_count":0,"prompt_count":0,"tools":[],"resources":[],"prompts":[]}]}),
        ),
        (200, json!({"content":[]})),
    ]);
    assert_eq!(server.client().mcp_inspect().await.unwrap().servers[0].name, "docs");
    let inspect = server.take_request();
    assert_eq!(inspect.method, "GET");
    assert_eq!(inspect.path, "/v1/mcp/inspect");
    assert!(inspect.body.is_null());
    assert_eq!(
        server
            .client()
            .mcp_app_call("session", "docs", "binding", "search", &json!({"query":"fixture"}))
            .await
            .unwrap(),
        json!({"content":[]})
    );
    let call = server.take_request();
    assert_eq!(call.method, "POST");
    assert_eq!(call.path, "/v1/mcp/apps/call");
    assert_eq!(
        call.body,
        json!({"session":"session","server":"docs","binding":"binding","name":"search","arguments":{"query":"fixture"}})
    );
    server.join();
}
