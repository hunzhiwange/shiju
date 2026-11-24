//! Loopback gateway access bound to the launching session's project and user.

use std::sync::Arc;

use axum::{
    Router,
    extract::{Path, Request, State},
    http::StatusCode,
    middleware::{Next, from_fn_with_state},
    response::Response,
};
use gateway_pairing::pairing::{constant_time_eq, hash_skey};
use vw_api_transport::ProjectScope;
use vw_api_types::cli_workbench::CliWorkbenchChildContext;

use super::{ErrorCode, WORKBENCH, WorkbenchError, collaboration_routes, error, not_found};

pub(super) struct ChildAccess {
    context: String,
    server: tokio::task::JoinHandle<()>,
}

struct Binding {
    scope: ProjectScope,
    session_id: String,
    token_hash: String,
}

impl ChildAccess {
    pub(super) async fn start(scope: &ProjectScope, session_id: &str) -> anyhow::Result<Self> {
        let listener = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).await?;
        let token = uuid::Uuid::new_v4().to_string();
        let binding = Arc::new(Binding {
            scope: scope.clone(),
            session_id: session_id.to_owned(),
            token_hash: hash_skey(&token),
        });
        let context = serde_json::to_string(&CliWorkbenchChildContext {
            port: listener.local_addr()?.port(),
            project_id: scope.project_id().as_ref().to_owned(),
            token,
        })?;
        let routes =
            collaboration_routes::<()>().route_layer(from_fn_with_state(binding, authorize));
        let app = Router::new().nest("/v1", routes);
        let server = tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });
        Ok(Self { context, server })
    }

    pub(super) fn context(&self) -> &str {
        &self.context
    }
}

impl Drop for ChildAccess {
    fn drop(&mut self) {
        self.server.abort();
    }
}

async fn authorize(
    State(binding): State<Arc<Binding>>,
    Path(params): Path<std::collections::HashMap<String, String>>,
    mut request: Request,
    next: Next,
) -> Result<Response, WorkbenchError> {
    let token = gateway_types::auth::extract_auth_skey(request.headers());
    if !token.is_some_and(|token| constant_time_eq(&binding.token_hash, &hash_skey(token))) {
        return Err(error(
            StatusCode::UNAUTHORIZED,
            ErrorCode::Unauthorized,
            "invalid workbench child credential",
        ));
    }
    if params.get("project").map(String::as_str) != Some(binding.scope.project_id().as_ref()) {
        return Err(not_found());
    }

    {
        let mut workbench = WORKBENCH.lock();
        let source = workbench.session_mut(&binding.scope, &binding.session_id)?;
        source.refresh_process_state();
        if source.child_access.is_none() {
            return Err(not_found());
        }
    }
    request.extensions_mut().insert(binding.scope.tenant_scope().clone());
    Ok(next.run(request).await)
}

#[cfg(test)]
#[path = "child_access_tests.rs"]
mod tests;
