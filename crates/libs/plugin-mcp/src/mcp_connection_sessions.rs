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
fn key(tenant: &vw_api_transport::TenantScope, session: &str, root: &Path) -> String {
    let root = root.canonicalize().unwrap_or_else(|_| root.to_owned());
    hex::encode(Sha256::digest(
        serde_json::to_vec(&(tenant, root, session)).expect("session identity"),
    ))
}
pub fn connections(host: Arc<dyn super::connection_host::ConnectionHost>, tenant: vw_api_transport::TenantScope, session: &str, root: &Path) -> Arc<McpSessionConnections> {
    sessions()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .entry(key(&tenant, session, root))
        .or_insert_with(|| {
            Arc::new(McpSessionConnections::for_form_owner(host, super::mcp_elicitation::FormOwner {
                session: session.to_owned(),
                tenant,
            }))
        })
        .clone()
}
pub fn close_session(host: &dyn super::connection_host::ConnectionHost, tenant: vw_api_transport::TenantScope, session: &str, root: &Path) {
    host.revoke_app(&tenant, session, root);
    if let Some(pool) = sessions()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .remove(&key(&tenant, session, root))
    {
        pool.shutdown();
    }
}
pub fn close_plugin(root: &Path) {
    let root = root.canonicalize().unwrap_or_else(|_| root.to_owned());
    for pool in sessions().lock().unwrap_or_else(std::sync::PoisonError::into_inner).values() {
        pool.close_plugin(&root);
    }
}
