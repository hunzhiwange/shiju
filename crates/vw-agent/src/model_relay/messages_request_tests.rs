//! The complete lib suite also checks shared routing capability registration.
use super::{chat, messages, routing};
#[path = "messages_request_core_tests.rs"]
mod request_core_tests;

#[test]
fn model_protocol_014_shared_selector_advertises_only_executable_messages_capabilities() {
    use vw_provider_resolver::protocol_routes::RouteCapability as C;
    use vw_shared::provider::protocols::ModelProtocol as P;
    let conversions = routing::relay_conversions(false);
    let conversion =
        conversions.iter().find(|c| c.source == P::Messages && c.target == P::Chat).unwrap();
    assert!(conversion.capabilities.contains(&C::Image));
    assert!(conversion.capabilities.contains(&C::StructuredOutput));
    assert!(!conversion.capabilities.contains(&C::ThinkingReplay));
    assert!(!conversion.capabilities.contains(&C::Thinking));
}

#[tokio::test]
async fn model_protocol_014_http_request_reaches_bound_chat_and_finishes_tool_round() {
    let _lock = crate::test_support::global_async_env_lock().lock().await;
    super::messages_request_http_tests::request_reaches_bound_chat_and_finishes_tool_round().await;
}
