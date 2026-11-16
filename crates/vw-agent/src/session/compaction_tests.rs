//! Real compaction entrance drives the domain-owned boundary contract.
use session_engine::host_migration_23_compaction_contract as contract;
struct Host;
impl contract::CompactionBoundaryHost for Host {
    async fn process(input: session_engine::compaction::ProcessInput,model:&crate::provider::provider::Model)->Result<session_engine::compaction::CompactionOutcome,crate::session::session::Error> { super::process(input,model).await }
}
#[tokio::test]
async fn process_returns_stop_when_parent_user_message_is_missing() { contract::process_returns_stop_when_parent_user_message_is_missing::<Host>().await; }
#[tokio::test]
async fn process_does_not_compact_when_the_boundary_has_no_prior_history() { contract::process_does_not_compact_when_the_boundary_has_no_prior_history::<Host>().await; }
#[tokio::test]
async fn process_does_not_synthesize_a_summary_for_blank_history() { contract::process_does_not_synthesize_a_summary_for_blank_history::<Host>().await; }
