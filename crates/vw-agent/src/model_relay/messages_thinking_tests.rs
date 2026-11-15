use super::routing;
#[tokio::test]
async fn model_protocol_015_http_tool_round_and_sse_preserve_reasoning() {
    let _lock = crate::test_support::global_async_env_lock().lock().await;
    super::messages_thinking_http_tests::thinking_replays_through_bound_http().await;
}

#[test]
fn model_protocol_015_selector_exposes_thinking_and_replay() {
    use provider_protocol_routes::RouteCapability as C;
    use model_protocols::ModelProtocol as P;
    let routes = routing::relay_conversions(false);
    let route = routes.iter().find(|r| r.source == P::Messages).unwrap();
    assert!(route.capabilities.contains(&C::Thinking));
    assert!(route.capabilities.contains(&C::ThinkingReplay));
}
