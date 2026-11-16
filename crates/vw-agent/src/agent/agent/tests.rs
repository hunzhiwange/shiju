//! Real host loop integration; assertions and fixtures belong to agent-loop.
#[tokio::test]
async fn turn_without_tools_returns_text() {
    agent_loop::agent::test_support::tests::tests::turn_without_tools_returns_text::<crate::agent::loop_::core::skills::NativeCoreSkillHost>().await;
}
#[tokio::test]
async fn turn_with_native_dispatcher_handles_tool_results_variant() {
    agent_loop::agent::test_support::tests::tests::turn_with_native_dispatcher_handles_tool_results_variant::<crate::agent::loop_::core::skills::NativeCoreSkillHost>().await;
}
#[tokio::test]
async fn turn_routes_with_hint_when_query_classification_matches() {
    agent_loop::agent::test_support::tests::tests::turn_routes_with_hint_when_query_classification_matches::<crate::agent::loop_::core::skills::NativeCoreSkillHost>().await;
}
