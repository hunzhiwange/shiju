use super::routing;

#[test]
fn model_protocol_014_shared_selector_advertises_only_executable_messages_capabilities() {
    use provider_protocol_routes::RouteCapability as C;
    use model_protocols::ModelProtocol as P;
    let conversions = routing::relay_conversions(false);
    let conversion =
        conversions.iter().find(|c| c.source == P::Messages && c.target == P::Chat).unwrap();
    assert!(conversion.capabilities.contains(&C::Image));
    assert!(conversion.capabilities.contains(&C::StructuredOutput));
    assert!(conversion.capabilities.contains(&C::ThinkingReplay));
    assert!(conversion.capabilities.contains(&C::Thinking));
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
