//! Append-only context checkpoints. Original chat messages are never rewritten.

use serde_json::Value;
pub(super) use session_engine::compaction_checkpoint::{validate_session, message_hash, matches_prefix};
pub(super) fn scope_for_directory(directory: &str) -> String {
    session_engine::compaction_checkpoint::scope_for_directory(&super::ui_store::current_tenant_scope(), directory)
}
fn scope() -> String { scope_for_directory(&crate::project::instance::directory()) }
pub(crate) async fn save(session: &str, source: &[Value], summary: &str, retained_user: Option<Value>) -> Result<(), String> {
    session_engine::compaction_checkpoint::save(&scope(), session, source, summary, retained_user).await
}
pub(crate) async fn restore(session: &str, history: &[Value]) -> Result<Vec<Value>, String> {
    session_engine::compaction_checkpoint::restore(&scope(), session, history).await
}
pub(crate) async fn copy(source: &str, target: &str) -> Result<(), String> {
    let scope = scope();
    session_engine::compaction_checkpoint::copy_scoped(&scope, &scope, source, target, None).await
}
pub(crate) async fn copy_fork(source_directory: &str, target_directory: &str, source: &str, target: &str, history: &[Value]) -> Result<(), String> {
    session_engine::compaction_checkpoint::copy_fork(&scope_for_directory(source_directory), &scope_for_directory(target_directory), source, target, history).await
}
pub(crate) async fn remove_strict(session: &str, directory: &str) -> Result<(), String> {
    session_engine::compaction_checkpoint::remove_strict(&scope_for_directory(directory), session).await
}
pub(crate) async fn remove(session: &str, directory: &str) -> Result<(), String> {
    session_engine::compaction_checkpoint::remove(&scope_for_directory(directory), session).await
}
