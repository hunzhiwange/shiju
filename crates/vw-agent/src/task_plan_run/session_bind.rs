pub(crate) async fn ensure_plan_run_session_info(id: &str, parent: Option<&str>, root: &str) -> Result<(), String> {
    super::runner::ensure_host(); task_plan_runner::session_bind::ensure_plan_run_session_info(id, parent, root).await
}
pub(crate) async fn persist_plan_run_session_history(id: &str, root: &str, title: &str, user: &str, assistant: &str) -> Result<(), String> {
    super::runner::ensure_host(); task_plan_runner::session_bind::persist_plan_run_session_history(id, root, title, user, assistant).await
}
