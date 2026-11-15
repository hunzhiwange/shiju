//! 命令使用的会话与任务原始操作；权限与状态转换由运行库负责。
use std::{future::Future, pin::Pin};
use anyhow::Result;
use channel_support::{scope::ChannelMessageSnapshot, routing::ChannelRouteSelection};
use crate::host::ScopedFuture;
pub use session_port::SessionStatus as SessionActivity;

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
pub trait CommandHost: Send + Sync {
    fn command_tenant(&self) -> vw_api_transport::id::TenantScope;
    fn prepare_catalog(&self);
    fn command_now_ms(&self) -> u64;
    fn session_activity(&self, id: &str) -> SessionActivity;
    async fn in_project_scope(&self, snapshot: &ChannelMessageSnapshot, operation: ScopedFuture<'_>);
    async fn in_session_scope(&self, snapshot: &ChannelMessageSnapshot, operation: ScopedFuture<'_>);
    fn get_session<'a>(&'a self, id: &'a str) -> Pin<Box<dyn Future<Output = Result<session_info::Info, session_info::Error>> + Send + 'a>>;
    async fn get_scoped_session(&self, id: &str) -> Result<session_info::Info, session_info::Error>;
    #[cfg(not(target_arch = "wasm32"))]
    async fn reserve_session(&self, snapshot: &ChannelMessageSnapshot) -> Result<session_info::Info>;
    #[cfg(not(target_arch = "wasm32"))]
    async fn discard_session(&self, snapshot: &ChannelMessageSnapshot, info: session_info::Info) -> Result<()>;
    async fn set_session_title(&self, id: &str, title: &str) -> Result<()>;
    async fn set_session_archived(&self, id: &str, archived: Option<u64>) -> Result<()>;
    async fn fork_session(&self, id: &str, anchor: Option<&str>) -> Result<session_info::Info>;
    async fn remove_session(&self, id: &str) -> Result<()>;
    async fn revert_session(&self, id: &str, message: &str, part: Option<&str>) -> Result<session_info::Info>;
    async fn cleanup_revert(&self, info: &session_info::Info) -> Result<()>;
    async fn compact_session(&self, id: &str, anchor: &str, auto: bool) -> Result<session_engine::compaction::CompactionOutcome>;
    async fn load_ui_session(&self, snapshot: &ChannelMessageSnapshot, id: &str) -> Result<Option<session_port::ui_types::ChatSession>>;
    #[cfg(not(target_arch = "wasm32"))]
    async fn load_session_preview(&self, snapshot: &ChannelMessageSnapshot, id: &str) -> Result<Option<String>>;
    fn create_task(&self, project: &str, priority: u32, model: Option<&str>, prompt: &str) -> std::io::Result<String>;
}

struct Sessions<'a>(&'a dyn CommandHost);
impl channel_support::session::ChannelSessions for Sessions<'_> {
    fn get<'a>(&'a self, id: &'a str) -> Pin<Box<dyn Future<Output = Result<session_info::Info, session_info::Error>> + Send + 'a>> {
        Box::pin(self.0.get_session(id))
    }
    #[cfg(not(target_arch = "wasm32"))]
    fn reserve<'a>(&'a self, snapshot: &'a ChannelMessageSnapshot) -> Pin<Box<dyn Future<Output = Result<session_info::Info>> + Send + 'a>> {
        Box::pin(self.0.reserve_session(snapshot))
    }
    #[cfg(not(target_arch = "wasm32"))]
    fn discard<'a>(&'a self, snapshot: &'a ChannelMessageSnapshot, info: session_info::Info) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(self.0.discard_session(snapshot, info))
    }
}
pub async fn create_channel_session(host: &dyn CommandHost, snapshot: &ChannelMessageSnapshot, route: Option<ChannelRouteSelection>) -> Result<String> {
    channel_support::session::create_channel_session(&Sessions(host), snapshot, route).await
}
pub async fn owned_channel_sessions(host: &dyn CommandHost, snapshot: &ChannelMessageSnapshot) -> Result<Vec<session_info::Info>> {
    channel_support::session::owned_channel_sessions(&Sessions(host), snapshot).await
}
