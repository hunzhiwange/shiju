//! 使用真实宿主的命令目录集成测试。
use super::*;

use channel_runtime::command_test_support::CatalogChannel;

pub(super) async fn catalog_context()
-> (tempfile::TempDir, ChannelRuntimeContext, traits::ChannelMessage) {
    let (temp, mut ctx, _) = super::session::session_tests::registered_context().await;
    crate::config::update_project_for_tenant(&crate::session::ui_store::current_tenant_scope(), &std::fs::canonicalize(temp.path()).unwrap(), serde_json::json!({
        "providers": {"catalog": {"api": "https://example.test/v1", "adapter": "openai-compatible", "models": {
            "known": {"id": "gpt-5", "status": "active", "reasoning": true, "reasoning_options": [{"type": "effort", "values": ["low", "high"]}]},
            "other": {"id": "gpt-4.1", "status": "active", "reasoning": false}
        }}}
    })).await.unwrap();
    ctx.default_provider = Arc::new("catalog".into());
    ctx.model = Arc::new("other".into());
    let mut msg = super::session::session_tests::channel_message(Some("topic"));
    msg.channel = "lark".into();
    (temp, ctx, msg)
}

async fn catalog_dispatch(
    ctx: &ChannelRuntimeContext,
    msg: &traits::ChannelMessage,
    channel: &Arc<CatalogChannel>,
) -> String {
    let mut scoped = ctx.clone();
    scoped.message_snapshot = Some(resolve_channel_message_snapshot(ctx, msg).await.unwrap());
    let target: Arc<dyn Channel> = channel.clone();
    assert!(handle_runtime_command_if_needed(&scoped, msg, Some(&target)).await);
    channel.0.lock().await.last().unwrap().content.clone()
}

#[tokio::test]
async fn channel_command_catalog_real_provider_model_and_effort_success() {
    let (_temp, ctx, msg) = catalog_context().await;
    channel_runtime::command_catalog_contracts::channel_command_catalog_real_provider_model_and_effort_success(ctx, msg, dispatch_contract).await;
}

#[tokio::test]
async fn channel_command_catalog_invalid_inputs_preserve_route_and_history() {
    let (_temp, ctx, msg) = catalog_context().await;
    channel_runtime::command_catalog_contracts::channel_command_catalog_invalid_inputs_preserve_route_and_history(ctx, msg, dispatch_contract).await;
}

#[tokio::test]
async fn channel_command_catalog_model_selection_isolated_across_domains() {
    let (_temp, ctx, msg) = catalog_context().await;
    channel_runtime::command_catalog_contracts::channel_command_catalog_model_selection_isolated_across_domains(ctx, msg, dispatch_contract).await;
}

#[tokio::test]
async fn channel_command_catalog_status_is_read_only_and_scoped() {
    let (_temp, ctx, msg) = catalog_context().await;
    channel_runtime::command_catalog_contracts::channel_command_catalog_status_is_read_only_and_scoped(ctx, msg, dispatch_contract).await;
}

#[tokio::test]
async fn channel_command_catalog_provider_catalog_isolated_by_tenant() {
    let (temp, ctx, mut msg) = catalog_context().await;
    let channel = Arc::new(CatalogChannel::default());
    msg.content = "/model known".into();
    assert!(catalog_dispatch(&ctx, &msg, &channel).await.contains("Model switched"));
    let tenant =
        vw_api_transport::TenantScope::new("command-catalog", uuid::Uuid::new_v4().to_string())
            .unwrap();
    crate::session::ui_store::with_tenant_scope(tenant.clone(), async {
        let (project, _) = crate::project::from_directory(&tenant, temp.path()).await.unwrap();
        crate::config::update_project_for_tenant(&tenant, &std::fs::canonicalize(temp.path()).unwrap(), serde_json::json!({"providers": {"catalog": {"api": "https://example.test/v1", "models": {"tenant-model": {"status": "active"}}}}})).await.unwrap();
        let context = crate::project::instance::Context { directory: std::fs::canonicalize(temp.path()).unwrap().to_string_lossy().into(), worktree: project.worktree.clone(), project };
        crate::project::instance::provide_context(context, async {
            assert!(provider::get_model("catalog", "known").await.is_err());
            let loaded = crate::config::get_project_for_tenant(&tenant, &std::fs::canonicalize(temp.path()).unwrap()).await.unwrap();
            assert!(loaded.providers.get("catalog").and_then(|p| p.get("models")).and_then(|m| m.get("tenant-model")).is_some(), "isolated fixture configuration missing model");
            let result = provider::get_model("catalog", "tenant-model").await;
            assert!(result.is_ok(), "{result:?}");
        }).await;
    }).await;
}

fn dispatch_contract<'a>(ctx: &'a ChannelRuntimeContext, msg: &'a traits::ChannelMessage) -> std::pin::Pin<Box<dyn std::future::Future<Output = String> + Send + 'a>> {
    Box::pin(async move { catalog_dispatch(ctx, msg, &Arc::new(CatalogChannel::default())).await })
}
