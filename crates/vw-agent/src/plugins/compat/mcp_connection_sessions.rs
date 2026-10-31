//! Native chat ownership is tenant/workspace/session scoped. No cross-chat connection sharing.
use super::mcp_connections::McpSessionConnections;
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    path::Path,
    sync::{Arc, Mutex, OnceLock},
};

fn sessions() -> &'static Mutex<HashMap<String, Arc<McpSessionConnections>>> {
    static SESSIONS: OnceLock<Mutex<HashMap<String, Arc<McpSessionConnections>>>> = OnceLock::new();
    SESSIONS.get_or_init(Default::default)
}
pub(super) fn key(session: &str, root: &Path) -> String {
    let tenant = crate::session::ui_store::current_tenant_scope();
    let root = root.canonicalize().unwrap_or_else(|_| root.to_owned());
    hex::encode(Sha256::digest(
        serde_json::to_vec(&(tenant, root, session)).expect("session identity"),
    ))
}
pub(crate) fn connections(session: &str, root: &Path) -> Arc<McpSessionConnections> {
    sessions()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .entry(key(session, root))
        .or_default()
        .clone()
}
pub(crate) fn close_session(session: &str, root: &Path) {
    if let Some(pool) = sessions()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .remove(&key(session, root))
    {
        pool.shutdown();
    }
}
pub(crate) fn close_plugin(root: &Path) {
    let root = root.canonicalize().unwrap_or_else(|_| root.to_owned());
    for pool in sessions().lock().unwrap_or_else(std::sync::PoisonError::into_inner).values() {
        pool.close_plugin(&root);
    }
}
