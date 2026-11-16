//! Real native host entry bindings; shared scenarios keep the full assertions.
use agent_loop::host_contract_29::runner;

#[tokio::test]
async fn process_message_returns_error_for_unknown_provider() {
    runner::process_message_returns_error_for_unknown_provider(super::process_message).await;
}

#[tokio::test]
async fn process_message_uses_default_provider_fallback_before_submit_error() {
    runner::process_message_uses_default_provider_fallback_before_submit_error(super::process_message).await;
}
