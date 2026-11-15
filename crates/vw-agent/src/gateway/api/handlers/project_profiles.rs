//! Profile route composition with administrator-owned authorization.
pub(super) fn router() -> axum::Router {
    gateway_project::project_profiles::router(gateway_project::project_profiles::ProfileAuthorization {
        read_or_create: super::agent_apps::authorize_project_profiles,
        maintenance: super::agent_apps::authorize_project_profiles_maintenance,
    })
}
