//! Real CLI acceptance orchestration; callers supply actual host configuration writes.
//! No simulated runtime or permission implementation is provided here.
use super::host::{Host, HostFuture};
use serde_json::Value;
use std::path::Path;
pub trait AcceptanceHost: Host {
    fn update_project<'a>(root: &'a Path, patch: Value) -> HostFuture<'a, anyhow::Result<()>>;
    fn update(patch: Value) -> HostFuture<'static, anyhow::Result<()>>;
}
mod controls {
    fn host<H: super::AcceptanceHost>() -> crate::acp::controls::Host {
        crate::acp::controls::Host {
            tenant: H::tenant, open: H::open_selection,
            session_name: crate::acp::session::acp_session_name,
            verified_release: crate::acp::model_source::drivers::verified_release,
            capabilities: crate::acp::model_source::drivers::capabilities,
        }
    }
    pub(super) async fn read<H: super::AcceptanceHost>(session: &str, cwd: &str) -> Result<vw_api_acp_model::acp_session::AcpSessionOptions, String> {
        crate::acp::controls::read(&host::<H>(), session, cwd).await
    }
    pub(super) async fn select<H: super::AcceptanceHost>(session: &str, cwd: &str, selection: vw_api_acp_model::acp_session::AcpSessionSelection) -> Result<vw_api_acp_model::acp_session::AcpSessionOptions, String> {
        crate::acp::controls::select(&host::<H>(), session, cwd, selection).await
    }
}
pub mod model_source;
pub mod skill_mounts;
