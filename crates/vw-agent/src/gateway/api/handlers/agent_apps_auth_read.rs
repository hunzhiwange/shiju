//! Host authentication wiring for the application authorization routes.
use axum::{Router, extract::Request, middleware::Next, response::Response};
use std::path::PathBuf;
#[cfg(test)]
use std::sync::Arc;
#[cfg(test)]
use gateway_agent_apps::agent_apps_auth_start::Runtime;

fn basic_auth(request: Request, next: Next, password: Option<&'static str>, username: &'static str)
    -> futures_util::future::BoxFuture<'static, Response>
{
    Box::pin(crate::gateway::middleware::basic_auth_middleware_with_credentials(request, next, password, username))
}

pub(crate) fn router(password: Option<&'static str>, username: &'static str, policy: Option<PathBuf>) -> Router {
    gateway_agent_apps::agent_apps_auth_read::router(password, username, policy, basic_auth)
}

#[cfg(test)]
pub(super) fn router_with_runtime(password: Option<&'static str>, username: &'static str, policy: Option<PathBuf>, runtime: Arc<Runtime>) -> Router {
    gateway_agent_apps::agent_apps_auth_read::router_with_runtime(password, username, policy, basic_auth, runtime)
}
