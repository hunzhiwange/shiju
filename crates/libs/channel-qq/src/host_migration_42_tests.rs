use super::*;

#[tokio::test]
async fn channel_project_binding_qq_webhook_uses_default_instance() {
    let channel = QQChannel::new("app".into(), "secret".into(), vec!["*".into()]);
    let messages = channel.parse_webhook_payload(&serde_json::json!({"op": 0, "t": "C2C_MESSAGE_CREATE", "d": {"id": "msg", "content": "hello", "author": {"user_openid": "user"}}})).await;
    let message = &messages[0];
    assert_eq!(message.instance_id, "default");
    assert_eq!(message.channel, "qq");
}
