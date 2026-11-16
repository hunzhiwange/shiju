//! ACP 挂载存储由领域负责，保留宿主兼容入口。
pub(super) use agent_acp::mount_store::load;
#[cfg(test)]
pub(super) use agent_acp::mount_store::key;
pub(super) use agent_acp::mounts::save;
