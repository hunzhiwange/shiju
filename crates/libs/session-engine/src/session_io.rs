//! Primitive metadata operations supplied by the current session storage owner.
use session_info::{Info, Error};
use std::future::Future;
use permission_runtime::next as permission_next;
#[derive(Debug, Clone, Default)]
pub struct CreateInput {
    /// 父会话 ID（用于创建子会话）
    pub parent_id: Option<String>,
    /// 会话标题（可选，未提供则使用默认标题）
    pub title: Option<String>,
    /// 工作目录路径
    pub directory: String,
    /// 权限规则集（可选）
    pub permission: Option<permission_next::Ruleset>,
}


pub trait SessionIo: Send + Sync + 'static {
    fn home() -> std::path::PathBuf;
    fn get(id: &str) -> impl Future<Output=Result<Info,Error>> + Send;
    fn get_any(id: &str) -> impl Future<Output=Result<Info,Error>> + Send;
    fn update(id: &str, f: impl FnOnce(&mut Info) + Send) -> impl Future<Output=Result<Info,Error>> + Send;
    fn create_next(input: CreateInput) -> impl Future<Output=Result<Info,Error>> + Send;
    #[cfg(not(target_arch="wasm32"))]
    fn reserve_next(input: CreateInput) -> impl Future<Output=Result<Info,Error>> + Send;
    #[cfg(not(target_arch="wasm32"))]
    fn reserve_visible_task(input: CreateInput) -> impl Future<Output=Result<Info,Error>> + Send;
    fn children_including_pending(id: &str) -> impl Future<Output=Result<Vec<Info>,Error>> + Send;
}
