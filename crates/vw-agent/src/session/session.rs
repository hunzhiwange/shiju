//! Session lifecycle composition and turn-fork compatibility.
pub use session_engine::session_lifecycle::{event, now_ms, Session, Message, Role, Error, Info, RevertInfo, ShareInfo, Summary, TimeInfo, CreateInput, create_slug, is_default_title, lock_metadata_writes, children, children_including_pending};
use super::session_io::NativeSessionIo;
#[path = "turn_fork.rs"]
mod turn_fork;
pub(crate) use turn_fork::{TurnFork, forked_session, load_fork_history};
pub async fn create_next(input: CreateInput) -> Result<Info, Error> {
    session_engine::session_lifecycle::create_next::<NativeSessionIo>(input).await
}
#[cfg(not(target_arch = "wasm32"))]
pub(crate) async fn reserve_next(input: CreateInput) -> Result<Info, Error> {
    session_engine::session_lifecycle::reserve_next::<NativeSessionIo>(input).await
}
#[cfg(not(target_arch = "wasm32"))]
pub(super) async fn reserve_visible_task(input: CreateInput) -> Result<Info, Error> {
    session_engine::session_lifecycle::reserve_visible_task::<NativeSessionIo>(input).await
}
#[cfg(not(target_arch = "wasm32"))]
pub(super) async fn reserve_child_agent(input: CreateInput) -> Result<Info, Error> {
    session_engine::session_lifecycle::reserve_child_agent::<NativeSessionIo>(input).await
}
pub async fn get(session_id: &str) -> Result<Info, Error> {
    session_engine::session_lifecycle::get::<NativeSessionIo>(session_id).await
}
pub async fn touch(session_id: &str) -> Result<Info, Error> {
    session_engine::session_lifecycle::touch::<NativeSessionIo>(session_id).await
}
pub async fn update(session_id: &str, f: impl FnOnce(&mut Info)) -> Result<Info, Error> {
    session_engine::session_lifecycle::update::<NativeSessionIo>(session_id, f).await
}
pub async fn list() -> Result<Vec<Info>, Error> {
    session_engine::session_lifecycle::list::<NativeSessionIo>().await
}
pub(crate) async fn list_for_project(project_id: String) -> Result<Vec<Info>, Error> {
    session_engine::session_lifecycle::list_for_project::<NativeSessionIo>(project_id).await
}
pub async fn list_all() -> Result<Vec<Info>, Error> {
    session_engine::session_lifecycle::list_all::<NativeSessionIo>().await
}
pub async fn get_any(session_id: &str) -> Result<Info, Error> {
    session_engine::session_lifecycle::get_any::<NativeSessionIo>(session_id).await
}
pub async fn update_any(session_id: &str, f: impl FnOnce(&mut Info)) -> Result<Info, Error> {
    session_engine::session_lifecycle::update_any::<NativeSessionIo>(session_id, f).await
}
pub async fn remove(session_id: &str) -> Result<(), Error> {
    session_engine::session_lifecycle::remove::<NativeSessionIo>(session_id).await
}
pub async fn remove_any(session_id: &str) -> Result<(), Error> {
    session_engine::session_lifecycle::remove_any::<NativeSessionIo>(session_id).await
}
pub(crate) async fn remove_archived_locked(info: Info) -> Result<(), Error> {
    session_engine::session_lifecycle::remove_archived_locked::<NativeSessionIo>(info).await
}
pub async fn fork(session_id: &str, message_id: Option<&str>) -> Result<Info, Error> {
    fork_with_parent(session_id, message_id, None).await
}

pub(crate) async fn fork_with_parent(session_id: &str, message_id: Option<&str>, parent_id: Option<String>) -> Result<Info, Error> {
    session_engine::session_fork::fork_with_parent::<super::session_io::NativeSessionIo>(session_id, message_id, parent_id).await
}

#[cfg(test)]
#[path = "session_tests.rs"]
mod session_tests;
