//! 推理门面的公开类型与错误转发契约。
use super::{GatewayAgentRequest, GatewayClient};

#[tokio::test]
async fn inference_facade_preserves_validation_errors_and_domain_types() {
    let client = GatewayClient::new(crate::endpoint::GatewayEndpoint::new("127.0.0.1", 1))
        .expect("client");
    let request: gateway_inference::GatewayAgentRequest =
        GatewayAgentRequest { message: "  ".into() };
    assert_eq!(client.inference_agent(&request).await.unwrap_err(), "agent message must not be empty");
    assert_eq!(
        client.inference_openai_chat(&serde_json::Value::Null).await.unwrap_err(),
        "OpenAI chat request must be a JSON object"
    );
}
