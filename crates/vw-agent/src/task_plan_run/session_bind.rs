pub(crate) async fn ensure_plan_run_session_info(id: &str, parent: Option<&str>, root: &str) -> Result<(), String> {
    super::runner::ensure_host(); task_plan_runner::session_bind::ensure_plan_run_session_info(id, parent, root).await
}
