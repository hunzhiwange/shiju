//! Primitive metadata storage and installation scope binding.
pub(crate) struct NativeSessionIo;
impl session_engine::session_io::SessionIo for NativeSessionIo {
    fn home() -> std::path::PathBuf { crate::global::paths().home.clone() }
    async fn get(id: &str) -> Result<session_info::Info, session_info::Error> { super::session::get(id).await }
    async fn get_any(id: &str) -> Result<session_info::Info, session_info::Error> { super::session::get_any(id).await }
    async fn create_next(input: session_engine::session_io::CreateInput) -> Result<session_info::Info, session_info::Error> { super::session::create_next(input).await }
    #[cfg(not(target_arch="wasm32"))]
    async fn reserve_next(input: session_engine::session_io::CreateInput) -> Result<session_info::Info, session_info::Error> { super::session::reserve_next(input).await }
    #[cfg(not(target_arch="wasm32"))]
    async fn reserve_visible_task(input: session_engine::session_io::CreateInput) -> Result<session_info::Info, session_info::Error> { super::session::reserve_visible_task(input).await }
    async fn children_including_pending(id: &str) -> Result<Vec<session_info::Info>, session_info::Error> { super::session::children_including_pending(id).await }
    async fn update(id: &str, f: impl FnOnce(&mut session_info::Info) + Send) -> Result<session_info::Info, session_info::Error> { super::session::update(id, f).await }
}
