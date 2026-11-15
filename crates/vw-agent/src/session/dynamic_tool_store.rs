//! Scoped host adapter; persistence and catalog validation live in session-store.
use anyhow::Result;
pub use session_store::dynamic_tool_store::{PersistentToolPolicy, PersistedDynamicTools, rebind, interrupt_part};

/// Load only after the lifecycle boundary has checked session ownership.
pub(crate) fn load_scoped(
    project_id: &str,
    session_id: &str,
) -> Result<Option<PersistedDynamicTools>> {
    super::ui_store::with_dynamic_tool_store(project_id, |store, scope| {
        store.load_dynamic_tools(scope, session_id)
    })
}

pub(crate) fn copy_scoped(
    project_id: &str,
    source: &str,
    target: &super::session::Info,
) -> Result<()> {
    super::ui_store::with_dynamic_tool_store(project_id, |store, scope| {
        store.copy_dynamic_tools(scope, source, target)
    })
}
