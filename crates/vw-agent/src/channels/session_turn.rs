//! Session I/O adapter for the channel turn transaction.
use super::*;
pub(super) use channel_support::session_turn::mark_channel_reply_retryable;

pub(super) async fn take_committed_channel_reply(
    snapshot: &ChannelMessageSnapshot,
    msg: &traits::ChannelMessage,
) -> Result<Option<(String, Option<String>)>> {
    channel_support::session_turn::take_committed_channel_reply::<ChannelTurnHost>(snapshot, msg).await
}

pub(super) async fn persist_channel_turn(
    ctx: &ChannelRuntimeContext,
    msg: &traits::ChannelMessage,
    route: &ChannelRouteSelection,
    response: &str,
    processed: ChannelProcessorResponse,
) -> Result<Option<String>> {
    channel_support::session_turn::persist_channel_turn::<ChannelTurnHost>(ctx, msg, route, response, processed).await
}

struct ChannelTurnHost;
#[cfg(target_arch = "wasm32")]
impl channel_support::session_turn::ChannelTurnHost for ChannelTurnHost {}

#[cfg(not(target_arch = "wasm32"))]
impl channel_support::session_turn::ChannelTurnHost for ChannelTurnHost {
    async fn lock_history(id: &str) -> tokio::sync::OwnedMutexGuard<()> {
        crate::session::message::lock_history(id).await
    }
    async fn get(id: &str) -> Result<crate::session::session::Info> {
        Ok(crate::session::session::get_any(id).await?)
    }
    fn now_ms() -> u64 { crate::session::session::now_ms() }
    fn first_response_source(content: &str) -> String {
        crate::session::title::first_response_source(content)
    }
    fn load(tenant: &vw_api_transport::id::TenantScope, id: &str, scope: Option<&str>) -> Result<Option<crate::session::ui_types::ChatSession>> {
        crate::session::ui_store::load_session_scoped_for_tenant_fallible(tenant, id, scope)
    }
    fn commit_first(tenant: &vw_api_transport::id::TenantScope, project: &str, commit: crate::session::store::FirstResponseCommit<'_>) -> Result<bool> {
        crate::session::ui_store::commit_first_response(tenant, project, commit)
    }
    fn commit_snapshot(tenant: &vw_api_transport::id::TenantScope, chat: &crate::session::ui_types::ChatSession, scope: Option<&str>, raw: &[crate::session::llm_raw::LlmRawArtifact], steps: &[crate::session::store::LlmRawStepKey]) -> Result<PathBuf> {
        crate::session::ui_store::commit_session_ui_snapshot_scoped_for_tenant(tenant, chat, scope, raw, steps)
    }
    async fn refresh(id: &str) -> Result<()> {
        crate::session::session::update_any(id, |_| {}).await?;
        Ok(())
    }
    fn spawn_title(snapshot: &ChannelMessageSnapshot, tenant: vw_api_transport::id::TenantScope, id: String, preview: String, model: String) {
        crate::session::title::spawn_first_response_title(
            &tokio::runtime::Handle::current(), tenant,
            crate::project::instance::Context {
                directory: snapshot.directory.to_string_lossy().into(),
                worktree: snapshot.project.worktree.clone(),
                project: snapshot.project.clone(),
            }, id, preview, Some(model),
        );
    }
}
