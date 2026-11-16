use super::*;

#[tokio::test]
async fn agent_binding_inherited_history_cannot_override_fork_selection() {
    let _env = crate::test_support::global_async_env_lock().lock().await;
    let _fixture = crate::session::llm::recovery_test_support::RecoveryConfig::new(
        "https://example.test/v1",
        None,
    )
    .await;
    let workspace = tempfile::tempdir().unwrap();
    crate::project::instance::provide(workspace.path(), None, move || {
        Box::pin(async move {
            session_engine::host_migration_23_binding_contract::agent_binding_inherited_history_cannot_override_fork_selection::<AdmissionHost>().await;
        })
    })
    .await
    .unwrap();
}

impl session_engine::host_migration_23_binding_contract::BindingBoundaryHost for AdmissionHost {
    async fn create_next(input:session_engine::session_io::CreateInput)->Result<crate::session::session::Info,crate::session::session::Error> { crate::session::session::create_next(input).await }
    async fn create_fork(project:&str,session:&str,ids:Vec<String>)->anyhow::Result<()> { crate::session::fork_execution::create(project,session,ids).await }
    async fn start_fork(project:&str,session:&str)->anyhow::Result<()> { crate::session::fork_execution::start(project,session).await }
}
