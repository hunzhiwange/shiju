//! Verify the domain restoration contract with the real host ClientTool specialization.
use anyhow::Result;
use std::sync::Arc;
use agent_database::GlobalDatabase;
use session_engine::dynamic_tool_restore_contract::fixture;

#[test]
fn dynamic_tools_restore_client_specs_do_not_restore_host_capabilities() -> Result<()> {
    let home = tempfile::tempdir()?;
    let (store, scope) = fixture(GlobalDatabase::new(home.path().join("db.sqlite3"))?, "client-restore")?;
    session_engine::dynamic_tool_restore_contract::dynamic_tools_restore_client_specs_do_not_restore_host_capabilities(store, scope, home.path(), |spec, host| {
        Ok(Arc::new(crate::tools::client_tool::ClientTool::new(spec, host)?))
    })
}
