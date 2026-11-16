//! Host injection for the native scanner during ACP recovery.
pub(super) use llm_runtime::acp::recovery::{Launch, RecoveredSession};

pub(super) async fn recover_session(
    session: &str,
    identity: &str,
    cwd: &str,
    generation: u64,
    launch: &Launch<'_>,
) -> anyhow::Result<Option<RecoveredSession>> {
    llm_runtime::acp::recovery::recover_session(
        session, identity, cwd, generation, launch,
        |agent, configured| {
            super::skill_mounts::configure_native_scanner(agent, configured, None)
                .map_err(|error| anyhow::anyhow!(error.to_string()))
        },
    ).await
}
