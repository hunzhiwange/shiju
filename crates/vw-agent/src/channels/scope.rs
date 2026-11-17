//! 消息身份、项目解析和不可变执行快照。绑定失败时拒绝执行，不发现或创建项目。
use super::*;

#[cfg(all(not(test), not(target_arch = "wasm32")))]
pub(crate) use channel_support::scope::ChannelMessageSnapshot;
#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) use channel_support::scope::{ChannelChatScope, ChannelMessageScope, ChannelProjectSource, ChannelMessageSnapshot, ChannelProjectState};

pub(crate) async fn resolve_channel_message_snapshot(ctx: &ChannelRuntimeContext, msg: &traits::ChannelMessage) -> Result<ChannelMessageSnapshot> {
    channel_support::scope::resolve_channel_message_snapshot(&ctx.project_state, ctx.message_snapshot.as_ref(), &ctx.workspace_dir, msg).await
}

pub(crate) fn scoped_history_key(ctx: &ChannelRuntimeContext, msg: &traits::ChannelMessage) -> String {
    channel_support::scope::scoped_history_key(ctx, msg, crate::session::ui_store::current_tenant_scope())
}

#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) use channel_support::scope::channel_project_directory;

/// 以结构化编码索引出站实例，避免同平台多机器人互相覆盖。
#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) use channel_support::scope::channel_instance_key;

#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) use channel_support::scope::channel_for_message;

/// 使用运行时固定租户，避免依赖调用者线程的租户上下文。
#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) use channel_support::scope::channel_chat_scope;

/// 模型执行和项目切换共用锁；弱引用不保留已退出的聊天。
pub(crate) fn channel_chat_gate(ctx: &ChannelRuntimeContext, msg: &traits::ChannelMessage) -> Arc<tokio::sync::RwLock<()>> {
    channel_support::scope::channel_chat_gate(&ctx.project_state, msg)
}
