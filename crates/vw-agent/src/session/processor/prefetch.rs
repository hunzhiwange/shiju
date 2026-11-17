//! Typed prefetch tool and agent lookup composition.
use std::{collections::HashSet, path::{Path, PathBuf}};
use crate::tools::ToolRuntimeContext;
#[cfg(all(not(test), not(target_arch = "wasm32")))]
pub(crate) use session_engine::prefetch::app_session_scope_from_root_async;
#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) use session_engine::prefetch::{block_on, app_session_scope_from_root, app_session_scope_from_root_async};
pub(crate) struct AgentPrefetchHost;
impl session_engine::prefetch::PrefetchHost for AgentPrefetchHost {
    type Context = ToolRuntimeContext;
    fn root(ctx: &Self::Context) -> Option<&str> { ctx.root.as_deref() }
    type State = super::ToolSessionState;
    async fn default_agent() -> Result<String, String> { crate::agent::default_agent().await }
    async fn permission_rules(name: &str) -> Option<permission_runtime::next::Ruleset> { crate::agent::permission_rules(name).await }
    fn is_binary(path: &Path) -> bool { crate::tools::is_binary(path) }
    fn resolve_full_path(ctx: &ToolRuntimeContext, path: &str) -> PathBuf { super::utils::resolve_full_path(ctx, path) }
    fn run_read(session: &mut super::Session, input: &str, ctx: &ToolRuntimeContext, state: &mut Self::State) {
        let mut noop = |_event: super::StreamEvent| true;
        let _ = super::tools_exec::run_tool_and_record(session, "file_read", input, ctx, false, &mut noop, state);
    }
}
#[cfg(test)]
fn prefetch_allowed(permission: &str, target: &str) -> bool {
    session_engine::prefetch::prefetch_allowed::<AgentPrefetchHost>(permission, target)
}
#[cfg(test)]
fn try_prefetch_read(session: &mut super::Session, path: &str, ctx: &ToolRuntimeContext, state: &mut super::ToolSessionState) -> bool {
    session_engine::prefetch::try_prefetch_read::<AgentPrefetchHost>(session, path, ctx, state)
}
pub(crate) fn prefetch_seed_context(session: &mut super::Session, query: &str, ctx: &ToolRuntimeContext, allowed: &HashSet<String>, state: &mut super::ToolSessionState) {
    session_engine::prefetch::prefetch_seed_context::<AgentPrefetchHost>(session, query, ctx, allowed, state)
}
#[cfg(test)]
#[path = "../../../../libs/session-engine/tests/host_suites/prefetch_tests.rs"]
mod prefetch_tests;
