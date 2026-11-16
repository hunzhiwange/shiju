//! Authorization route behavior owned by this crate; real host assembly stays in vw-agent.
use crate::agent_apps_auth_start;

pub(crate) fn build_router_with_auth(cors: Vec<String>, auth: axum::Router) -> axum::Router {
    agent_gateway::router::build_router_with_auth(
        cors, auth, axum::Router::new(), axum::Router::new(), axum::Router::new(),
    )
}

pub(crate) mod auth_router {
    use axum::{Router, extract::Request, middleware::Next, response::Response};
    use std::{path::PathBuf, sync::Arc};
    use crate::agent_apps_auth_start::Runtime;

    fn basic_auth(request: Request, next: Next, password: Option<&'static str>, username: &'static str)
        -> futures_util::future::BoxFuture<'static, Response>
    {
        Box::pin(agent_gateway::middleware::basic_auth_middleware_with_credentials(request, next, password, username))
    }

    pub(crate) fn router(password: Option<&'static str>, username: &'static str, policy: Option<PathBuf>) -> Router {
        crate::agent_apps_auth_read::router(password, username, policy, basic_auth)
    }

    pub(crate) fn router_with_runtime(password: Option<&'static str>, username: &'static str, policy: Option<PathBuf>, runtime: Arc<Runtime>) -> Router {
        crate::agent_apps_auth_read::router_with_runtime(password, username, policy, basic_auth, runtime)
    }
}

mod agent_apps_auth_binding_tests;
mod agent_apps_auth_default_tests;
mod agent_apps_auth_integration_tests;
mod agent_apps_auth_logout_tests;
mod agent_apps_auth_read_tests;
mod agent_apps_auth_security_tests;
mod agent_apps_auth_start_tests;
mod agent_apps_auth_cancel_tests;
mod agent_apps_auth_poll_tests;
mod agent_apps_auth_redaction_tests;
