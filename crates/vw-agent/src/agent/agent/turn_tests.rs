//! Real host loop integration; assertions and fixtures belong to agent-loop.
#[tokio::test]
async fn turn_uses_ephemeral_system_reminder_and_enriches_user_message() {
    agent_loop::agent::test_support::turn_tests::turn_uses_ephemeral_system_reminder_and_enriches_user_message::<crate::agent::loop_::core::skills::NativeCoreSkillHost>().await;
}
#[tokio::test]
async fn agent_and_query_engine_use_ephemeral_agent_context_each_turn() {
    agent_loop::agent::test_support::turn_tests::agent_and_query_engine_use_ephemeral_agent_context_each_turn::<crate::agent::loop_::core::skills::NativeCoreSkillHost>().await;
}
#[tokio::test]
async fn agent_and_query_engine_use_ephemeral_stream_context_each_turn() {
    agent_loop::agent::test_support::turn_tests::agent_and_query_engine_use_ephemeral_stream_context_each_turn::<crate::agent::loop_::core::skills::NativeCoreSkillHost>().await;
}
#[tokio::test]
async fn turn_timestamp_uses_configured_timezone() {
    agent_loop::agent::test_support::turn_tests::turn_timestamp_uses_configured_timezone::<crate::agent::loop_::core::skills::NativeCoreSkillHost>().await;
}
#[tokio::test]
async fn turn_skips_memory_context_when_loader_fails() {
    agent_loop::agent::test_support::turn_tests::turn_skips_memory_context_when_loader_fails::<crate::agent::loop_::core::skills::NativeCoreSkillHost>().await;
}
#[tokio::test]
async fn turn_auto_save_records_user_message_without_blocking_response() {
    agent_loop::agent::test_support::turn_tests::turn_auto_save_records_user_message_without_blocking_response::<crate::agent::loop_::core::skills::NativeCoreSkillHost>().await;
}
#[tokio::test]
async fn turn_executes_tool_calls_and_returns_final_response() {
    agent_loop::agent::test_support::turn_tests::turn_executes_tool_calls_and_returns_final_response::<crate::agent::loop_::core::skills::NativeCoreSkillHost>().await;
}
#[tokio::test]
async fn turn_routes_classified_message_to_hint_model() {
    agent_loop::agent::test_support::turn_tests::turn_routes_classified_message_to_hint_model::<crate::agent::loop_::core::skills::NativeCoreSkillHost>().await;
}
#[tokio::test]
async fn turn_propagates_provider_error() {
    agent_loop::agent::test_support::turn_tests::turn_propagates_provider_error::<crate::agent::loop_::core::skills::NativeCoreSkillHost>().await;
}
#[tokio::test]
async fn turn_trims_history_after_success() {
    agent_loop::agent::test_support::turn_tests::turn_trims_history_after_success::<crate::agent::loop_::core::skills::NativeCoreSkillHost>().await;
}
#[tokio::test]
async fn turn_with_stream_sends_progress_and_final_response() {
    agent_loop::agent::test_support::turn_tests::turn_with_stream_sends_progress_and_final_response::<crate::agent::loop_::core::skills::NativeCoreSkillHost>().await;
}
#[tokio::test]
async fn turn_with_stream_timestamp_uses_configured_timezone() {
    agent_loop::agent::test_support::turn_tests::turn_with_stream_timestamp_uses_configured_timezone::<crate::agent::loop_::core::skills::NativeCoreSkillHost>().await;
}
#[tokio::test]
async fn turn_with_stream_auto_saves_and_uses_memory_context() {
    agent_loop::agent::test_support::turn_tests::turn_with_stream_auto_saves_and_uses_memory_context::<crate::agent::loop_::core::skills::NativeCoreSkillHost>().await;
}

use super::Agent;
use std::sync::Arc;
use model_types::{Provider, ChatMessage, ChatResponse, ToolCall};
use agent_memory::NoneMemory;
use agent_observability::NoopObserver;
use config_agent::AgentDefinitionConfig;
use agent_loop::agent::test_support::turn_tests::{ProviderHandle, text_response, collect_chunks};
#[path = "turn_skills_tests.rs"]
mod skills_tests;
