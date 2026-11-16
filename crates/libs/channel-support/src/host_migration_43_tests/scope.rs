//! 真实项目存储上的通道作用域策略。
use super::routing::context;
use super::session::channel_message;
use crate::{runtime_state::ChannelRuntimeContext, scope::*, routing::get_route_selection};
use crate::routing_context::set_route_selection;
use model_types::ChatMessage;
use std::sync::Arc;
use channel_types as traits;
use anyhow::Result;
use session_store::ui_store::{current_tenant_scope, with_tenant_scope};

async fn registered_context() -> (tempfile::TempDir, ChannelRuntimeContext, agent_project::Info) {
    let temp = tempfile::tempdir().unwrap();
    let ctx = context(temp.path());
    let (project, _) = agent_project::from_directory(&current_tenant_scope(), temp.path()).await.unwrap();
    (temp, ctx, project)
}
async fn resolve_channel_message_snapshot(ctx: &ChannelRuntimeContext, msg: &traits::ChannelMessage) -> Result<ChannelMessageSnapshot> {
    crate::scope::resolve_channel_message_snapshot(&ctx.project_state, ctx.message_snapshot.as_ref(), &ctx.workspace_dir, msg).await
}
fn append_sender_turn(ctx: &ChannelRuntimeContext, key: &str, turn: ChatMessage) {
    crate::history::append_sender_turn(&ctx.conversation_histories, key, turn);
}
fn clear_sender_history(ctx: &ChannelRuntimeContext, key: &str) {
    crate::history::clear_sender_history(&ctx.conversation_histories, key);
}

#[tokio::test]
pub(crate) async fn channel_project_scope_binding_priority_and_immutable_snapshot() {
    let (_a, ctx, a) = registered_context().await;
    let (_b, _, b) = registered_context().await;
    let msg = channel_message(None);
    let fallback = resolve_channel_message_snapshot(&ctx, &msg).await.unwrap();
    assert_eq!(fallback.scope.project, a.id);
    assert_eq!(fallback.source, ChannelProjectSource::Workspace);
    ctx.project_state
        .lock()
        .unwrap()
        .config
        .instance_default_projects
        .insert("telegram:default".into(), b.id.clone());
    let default = resolve_channel_message_snapshot(&ctx, &msg).await.unwrap();
    assert_eq!(default.scope.project, b.id);
    assert_eq!(default.source, ChannelProjectSource::Instance);
    ctx.project_state
        .lock()
        .unwrap()
        .selections
        .insert(ChannelChatScope::from_message(&msg, current_tenant_scope()), a.id.clone());
    let explicit = resolve_channel_message_snapshot(&ctx, &msg).await.unwrap();
    assert_eq!(explicit.source, ChannelProjectSource::Chat);
    assert_eq!(explicit.scope.project, a.id);
    assert_eq!(default.scope.project, b.id);
    ctx.project_state
        .lock()
        .unwrap()
        .config
        .instance_default_projects
        .insert("telegram:default".into(), "missing".into());
    assert_eq!(resolve_channel_message_snapshot(&ctx, &msg).await.unwrap().scope.project, a.id);
}

#[tokio::test]
pub(crate) async fn channel_project_scope_invalid_binding_preserves_state() {
    let (_a, ctx, _) = registered_context().await;
    let msg = channel_message(None);
    ctx.project_state
        .lock()
        .unwrap()
        .config
        .instance_default_projects
        .insert("telegram:default".into(), "missing-project-for-scope".into());
    append_sender_turn(&ctx, "keep", ChatMessage::user("keep"));
    assert!(resolve_channel_message_snapshot(&ctx, &msg).await.is_err());
    assert_eq!(ctx.conversation_histories.lock().unwrap()["keep"].len(), 1);
    assert!(ctx.project_state.lock().unwrap().selections.is_empty());
    ctx.project_state
        .lock()
        .unwrap()
        .selections
        .insert(ChannelChatScope::from_message(&msg, current_tenant_scope()), "missing-explicit".into());
    ctx.project_state.lock().unwrap().config.instance_default_projects.clear();
    assert!(resolve_channel_message_snapshot(&ctx, &msg).await.is_err());
}

#[tokio::test]
pub(crate) async fn channel_project_scope_project_history_and_routes_are_independent() {
    let (_a, ctx, _) = registered_context().await;
    let (_b, _, b) = registered_context().await;
    let msg = channel_message(None);
    let a = resolve_channel_message_snapshot(&ctx, &msg).await.unwrap();
    ctx.project_state
        .lock()
        .unwrap()
        .config
        .instance_default_projects
        .insert("telegram:default".into(), b.id);
    let b = resolve_channel_message_snapshot(&ctx, &msg).await.unwrap();
    let mut route = get_route_selection(&ctx, &a.scope.key());
    route.model = "chosen-model".into();
    route.task_mode_enabled = true;
    set_route_selection(&ctx, &a.scope.key(), route);
    assert_ne!(get_route_selection(&ctx, &b.scope.key()).model, "chosen-model");
    assert!(!get_route_selection(&ctx, &b.scope.key()).task_mode_enabled);
    append_sender_turn(&ctx, &a.scope.key(), ChatMessage::user("keep a"));
    clear_sender_history(&ctx, &b.scope.key());
    assert_eq!(ctx.conversation_histories.lock().unwrap()[&a.scope.key()][0].content, "keep a");
}

#[tokio::test]
pub(crate) async fn channel_project_scope_tenant_cannot_resolve_other_tenant_binding() {
    let (_a, ctx, a) = registered_context().await;
    let msg = channel_message(None);
    ctx.project_state
        .lock()
        .unwrap()
        .config
        .instance_default_projects
        .insert("telegram:default".into(), a.id);
    let first = ChannelChatScope::from_message(&msg, current_tenant_scope());
    with_tenant_scope(
        vw_api_transport::TenantScope::new("scope-platform", "scope-company").unwrap(),
        async {
            assert_ne!(first, ChannelChatScope::from_message(&msg, current_tenant_scope()));
            ctx.project_state.lock().unwrap().tenant =
                current_tenant_scope();
            assert!(resolve_channel_message_snapshot(&ctx, &msg).await.is_err());
        },
    )
    .await;
}
