// 验证 Webhook 与代理请求体能够反序列化消息字段。
use super::*;

#[test]
fn webhook_and_agent_bodies_deserialize_messages() {
    let webhook: WebhookBody =
        serde_json::from_value(serde_json::json!({"message": "ping"})).expect("webhook body");
    let agent: AgentBody =
        serde_json::from_value(serde_json::json!({"message": "run"})).expect("agent body");

    assert_eq!(webhook.message, "ping");
    assert_eq!(agent.message, "run");
}
