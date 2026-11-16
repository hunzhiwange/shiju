//! Session UI persistence compatibility exports and trusted binding projection.
pub use session_store::ui_store::*;
use vw_api_transport::id::TenantScope;

/// Load only the display projection; UI writes never reach the trusted binding record.
pub(crate) fn load_session_agent_identity_scoped_for_tenant(
    tenant: &TenantScope,
    session: &str,
    project: Option<&str>,
) -> std::io::Result<Option<crate::session::ui_types::SessionAgentIdentity>> {
    let Some(project) = project else {
        return Ok(None);
    };
    crate::session::prompt::block_on(with_tenant_scope(tenant.clone(), async {
        crate::session::agent_binding::load_scoped(project, session)
            .await
            .map(|binding| binding.map(|binding| binding.identity()))
            .map_err(std::io::Error::other)
    }))
}
