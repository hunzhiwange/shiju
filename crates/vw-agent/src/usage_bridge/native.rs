//! Native request host authorization and runtime context adapter.
#[cfg(all(not(test), not(target_arch = "wasm32")))]
pub(crate) use agent_usage::native::with_session;
#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) use agent_usage::native::{with_session, ui_tokens};

pub(crate) struct NativeRequest;

impl NativeRequest {
    pub(crate) fn begin(provider: &str, model: &str, session: Option<&str>) -> Option<agent_usage::native::NativeRequest> {
        match Self::try_begin(provider, model, session) {
            Ok(request) => Some(request),
            Err(_) => {
                tracing::debug!("native usage observation denied: trusted local scope unavailable");
                None
            }
        }
    }

    fn try_begin(provider: &str, model: &str, session: Option<&str>) -> anyhow::Result<agent_usage::native::NativeRequest> {
        let tenant = crate::session::ui_store::explicit_tenant_scope()
            .ok_or_else(|| anyhow::anyhow!("usage requires explicit tenant"))?;
        let project = crate::project::instance::project()
            .ok_or_else(|| anyhow::anyhow!("usage requires registered project"))?;
        let scope = vw_api_transport::ProjectScope::new(tenant, project.id.into())?;
        let database = super::context::database()?;
        anyhow::ensure!(crate::project::store::ProjectStore::new(database)?.get(&scope)?.is_some(),
            "usage project outside tenant");
        // Only existing private desktop authorization may establish this local host.
        anyhow::ensure!(local_app_auth::restore(&crate::global::paths().home,
            scope.platform_id().as_ref(), scope.company_id().as_ref())?.is_some(),
            "local usage host is not provisioned");
        Ok(agent_usage::native::NativeRequest::begin(scope, super::context::local_host()?,
            super::context::database, provider, model, session))
    }
}
