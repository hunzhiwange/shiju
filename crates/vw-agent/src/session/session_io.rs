//! Primitive metadata storage and installation scope binding.
pub(crate) struct NativeSessionIo;
impl session_engine::session_lifecycle::LifecycleHost for NativeSessionIo {
    fn home() -> std::path::PathBuf { crate::global::paths().home.clone() }
    fn version() -> String { agent_installation::version() }
    async fn close_hooks(id: &str, directory: &std::path::Path) { crate::plugins::hook_session::close(id, directory).await; }
    fn close_mcp(id: &str, directory: &std::path::Path) { crate::plugins::close_session_mcp_connections(id, directory); }
}
impl session_engine::session_io::SessionIo for NativeSessionIo {
    fn home() -> std::path::PathBuf { crate::global::paths().home.clone() }
    async fn get(id: &str) -> Result<session_info::Info, session_info::Error> { session_engine::session_lifecycle::get::<Self>(id).await }
    async fn get_any(id: &str) -> Result<session_info::Info, session_info::Error> { session_engine::session_lifecycle::get_any::<Self>(id).await }
    async fn create_next(input: session_engine::session_io::CreateInput) -> Result<session_info::Info, session_info::Error> { session_engine::session_lifecycle::create_next::<Self>(input).await }
    #[cfg(not(target_arch="wasm32"))]
    async fn reserve_next(input: session_engine::session_io::CreateInput) -> Result<session_info::Info, session_info::Error> { session_engine::session_lifecycle::reserve_next::<Self>(input).await }
    #[cfg(not(target_arch="wasm32"))]
    async fn reserve_visible_task(input: session_engine::session_io::CreateInput) -> Result<session_info::Info, session_info::Error> { session_engine::session_lifecycle::reserve_visible_task::<Self>(input).await }
    async fn children_including_pending(id: &str) -> Result<Vec<session_info::Info>, session_info::Error> { session_engine::session_lifecycle::children_including_pending(id).await }
    async fn update(id: &str, f: impl FnOnce(&mut session_info::Info) + Send) -> Result<session_info::Info, session_info::Error> { session_engine::session_lifecycle::update::<Self>(id, f).await }
}
