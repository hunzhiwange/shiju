//! Mount IO binding for history-anchored skill persistence.
use crate::skills::InvokedSkillRecord;
use serde_json::Value;
use super::compaction_checkpoint::scope_for_directory;
struct NativeMountRevision;
impl session_engine::skill_checkpoint::MountRevision for NativeMountRevision {
    async fn checkpoint_revision(session: &str) -> Result<Option<u64>, String> {
        crate::acp::mounts::checkpoint_revision(session).await
    }
}
pub(crate) async fn save(session: &str, agent: &str, history: &[Value], records: &[InvokedSkillRecord]) -> Result<(), String> {
    session_engine::skill_checkpoint::save::<NativeMountRevision>(session, agent, history, records).await
}
pub(crate) async fn restore(session: &str, agent: &str, history: &[Value]) -> Result<Vec<InvokedSkillRecord>, String> {
    session_engine::skill_checkpoint::restore::<NativeMountRevision>(session, agent, history).await
}
pub(super) async fn copy_scoped(source_scope: &str, target_scope: &str, source: &str, target: &str, hashes: Option<&[String]>) -> Result<(), String> {
    session_engine::skill_checkpoint::copy_scoped(source_scope, target_scope, source, target, hashes).await
}
pub(crate) async fn remove(session: &str, directory: &str) -> Result<(), String> {
    session_engine::skill_checkpoint::remove(&scope_for_directory(directory), session).await
}

#[cfg(test)]
#[path = "skill_checkpoint_tests.rs"]
mod tests;
