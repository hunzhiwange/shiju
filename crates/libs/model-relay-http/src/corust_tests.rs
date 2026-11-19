use super::*;
use serde_json::{Value, json};

#[test]
fn corust_relay_preserves_reviewed_native_and_graph_tool_declarations() {
    fn tool(name: &str) -> Value {
        json!({"type":"function", "function":{"name":name, "parameters":{"type":"object"}}})
    }
    let accepted = [
        "read_file", "write_file", "glob", "grep",
        "rustcodegraph_query", "rustcodegraph_callers", "rustcodegraph_node",
    ];
    let mut request = json!({
        "tools": accepted.into_iter().chain(["command", "rustcodegraph_explore"]).map(tool)
            .collect::<Vec<_>>(),
    });
    retain_corust_tools(&mut request);
    assert_eq!(request["tools"], json!(accepted.map(tool)));

    let mut unmounted = json!({"tools": [tool("read_file"), tool("command")]});
    retain_corust_tools(&mut unmounted);
    assert_eq!(unmounted["tools"], json!([tool("read_file")]));
}

struct NoUpstream;

#[async_trait::async_trait]
impl RelayBackend for NoUpstream {
    async fn execute(
        &self,
        _: &RelayIdentity,
        _: RelayRequest,
        _: CancellationToken,
    ) -> Result<Response, StatusCode> {
        panic!("Corust catalog must stay local")
    }
}

#[async_trait::async_trait]
impl RelayHost for NoUpstream {
    fn usage(
        &self,
        _: &RelayIdentity,
        _: GenerationProtocol,
    ) -> anyhow::Result<agent_usage::relay::RequestUsage> {
        panic!("Corust catalog must not record inference usage")
    }

    fn error(&self, _: GenerationProtocol, status: StatusCode) -> Response {
        status.into_response()
    }

    async fn control(&self, _: &RelayIdentity, _: &str, _: &Value) -> Result<Response, StatusCode> {
        panic!("Corust catalog does not use provider control routes")
    }
}

#[tokio::test]
async fn corust_catalog_is_authenticated_and_scoped_to_its_bound_model() {
    let identity = RelayIdentity {
        tenant: vw_api_transport::TenantScope::new("10001", "100010005").unwrap(),
        project: "corust-test".into(),
        session: "session:corust-agent".into(),
        provider: "alibaba-token-plan-cn".into(),
        model: "alibaba-token-plan-cn/deepseek-v4.1-flash-1017".into(),
        generation: 0,
    };
    let binding = RelayBinding::new(identity, Duration::from_secs(60)).unwrap();
    let server = RelayServer::start(binding.clone(), Arc::new(NoUpstream), Arc::new(NoUpstream))
        .await
        .unwrap();
    let url = format!("http://{}/v1/onerouter/models", server.address());
    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    assert_eq!(client.get(&url).send().await.unwrap().status(), StatusCode::UNAUTHORIZED);
    let response = client.get(&url).bearer_auth(binding.bearer_token()).send().await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let model = &binding.identity().model;
    assert_eq!(
        response.json::<Value>().await.unwrap(),
        json!({
            "provider": "onerouter",
            "models": [{"id": model, "internal_id": model, "display_name": model}],
        })
    );
    drop(server);
}
