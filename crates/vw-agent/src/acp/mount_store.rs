//! 宿主在持久化 ACP 挂载前验证实际技能租约。

use crate::session::runtime::SessionScope;
use acp_server::server::BackendError;
pub(super) use agent_acp::mount_store::load;
#[cfg(test)]
pub(super) use agent_acp::mount_store::key;

pub(super) async fn save(
    scope: &SessionScope,
    session: &str,
    prepared: &super::mounts::PreparedMounts,
) -> Result<(), BackendError> {
    prepared
        .skills
        .validate()
        .map_err(|_| BackendError::InvalidRequest("session skill source changed".into()))?;
    agent_acp::mount_store::save(
        scope,
        session,
        prepared.response.revision,
        prepared.skills.mounts().to_vec(),
        prepared.skills.fingerprint().map(str::to_owned),
        prepared.response.mcp_servers.clone(),
        prepared.mcp_catalog.clone(),
    ).await
}
