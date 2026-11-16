//! Read committed ACP catalog references after checking the exact session scope.

pub use agent_acp::catalog_mounts::{CatalogMountSelection, CatalogMountSnapshot};

pub async fn read_catalog_mounts(
    project: &vw_api_transport::ProjectScope,
    session_id: &str,
) -> Result<CatalogMountSnapshot, acp_server::server::BackendError> {
    agent_acp::catalog_mounts::read_catalog_mounts(
        project, session_id, &crate::global::paths().home,
        |scope, id| async move {
            crate::session::runtime::validate_session_scope(scope, &id).await
                .map_err(|_| acp_server::server::BackendError::OwnershipMismatch(id))
        },
    ).await
}
