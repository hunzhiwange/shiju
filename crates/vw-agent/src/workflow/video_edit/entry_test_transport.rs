//! Session-scoped external transport registration; receipts belong to the domain.
use super::entry::VideoEntryCloud;
use crate::tools::ToolRuntimeContext;
use model_types::Provider;
use std::{collections::{BTreeMap, HashSet}, sync::{Arc, LazyLock, Mutex}};
type External = (Arc<dyn VideoEntryCloud>, Arc<dyn Provider>);
type ExternalFactory = Arc<dyn Fn(&ToolRuntimeContext) -> External + Send + Sync>;
static EXTERNALS: LazyLock<Mutex<BTreeMap<String, ExternalFactory>>> =
    LazyLock::new(Default::default);
pub(super) fn install_external(session: &str, factory: ExternalFactory) -> TransportGuard {
    let guard = install(session);
    assert!(EXTERNALS.lock().unwrap().insert(session.into(), factory).is_none());
    guard
}
static SESSIONS: LazyLock<Mutex<HashSet<String>>> = LazyLock::new(Default::default);
pub(crate) struct TransportGuard(String);
impl Drop for TransportGuard {
    fn drop(&mut self) {
        SESSIONS.lock().unwrap().remove(&self.0);
        EXTERNALS.lock().unwrap().remove(&self.0);
    }
}
pub(crate) fn install(session: &str) -> TransportGuard {
    assert!(SESSIONS.lock().unwrap().insert(session.into()));
    TransportGuard(session.into())
}
pub(super) fn get(
    session: &str,
    context: &ToolRuntimeContext,
) -> Option<(Arc<dyn VideoEntryCloud>, Arc<dyn Provider>)> {
    if let Some(factory) = EXTERNALS.lock().unwrap().get(session) {
        return Some(factory(context));
    }
    SESSIONS.lock().unwrap().contains(session).then(|| {
        workflow_video_edit::test_support::receipts::external(Arc::new(context.clone()))
    })
}
