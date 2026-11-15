//! Host scope and storage construction for the ACP lifecycle owner.
use crate::{gateway::settings_store::GatewaySettingsStore, global_database::GlobalDatabase};
pub(super) use llm_runtime::acp::lifecycle::Selection;

pub(super) fn session_identity(session: &str, directory: &str) -> String {
    llm_runtime::acp::lifecycle::session_identity(&crate::session::ui_store::current_tenant_scope(), session, directory)
}
pub(super) fn try_lock(session: &str, directory: &str) -> Result<tokio::sync::OwnedMutexGuard<()>, String> {
    llm_runtime::acp::lifecycle::try_lock(&crate::session::ui_store::current_tenant_scope(), session, directory)
}
pub(super) struct Store;
impl Store {
    pub fn open(session: &str, directory: &str) -> anyhow::Result<llm_runtime::acp::lifecycle::Store> {
        let store = GatewaySettingsStore::new(GlobalDatabase::for_home(&crate::global::paths().home)?)?;
        Ok(llm_runtime::acp::lifecycle::Store::open(store, crate::session::ui_store::current_tenant_scope(), session, directory))
    }
}
pub(crate) fn locked_model(session: &str, directory: &str) -> Result<Option<vw_api_acp_model::acp_model_source::AcpSessionModelSelection>, String> {
    Ok(Store::open(session, directory).and_then(|store| store.read()).map_err(|error| error.to_string())?.filter(Selection::has_started).map(|selection| selection.model))
}
