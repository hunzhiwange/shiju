//! Summary composition over the existing session IO owner.
pub use session_engine::summary::{Error, SummarizeInput, DiffInput, diff, compute_diff};
pub async fn summarize(input: SummarizeInput) -> Result<(), Error> {
    super::llm::prompt::ensure_host();
    session_engine::summary::summarize::<super::session_io::NativeSessionIo>(input).await
}
pub async fn refresh_session_diff_summary(session_id: &str) -> Result<super::session::Summary, Error> {
    session_engine::summary::refresh_session_diff_summary::<super::session_io::NativeSessionIo>(session_id).await
}
pub(crate) async fn refresh_stream_diff_summary(session_id: &str, parts: &[super::message::Part]) -> Result<Option<super::session::Summary>, Error> {
    session_engine::summary::refresh_stream_diff_summary::<super::session_io::NativeSessionIo>(session_id, parts).await
}
