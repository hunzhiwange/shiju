use super::storage::{Host, Context as ToolUseContext};
use crate::{child_agent::coordinator::ChildAgentCoordinator, child_agent_access::{send_team_message, consume_team_messages}};
use session_port::child_agent::TeamMessageKind;
#[tokio::test(flavor = "current_thread")]
async fn agcollab_mailbox_runtime_identity_and_tenant_are_enforced() {
    use agent_project as project;
    use super::storage::session;
    if run_in_temporary_storage("agcollab_mailbox_runtime_identity_and_tenant_are_enforced") { return; }
    let workspace = tempfile::tempdir().unwrap();
    let directory = workspace.path().to_string_lossy().to_string();
    project::instance::provide(directory.clone(), None, move || {
        Box::pin(async move {
            let root = session::create_next(session::CreateInput {
                parent_id: None,
                title: Some("mail root".into()),
                directory: directory.clone(),
                permission: None,
            })
            .await
            .unwrap();
            let child = session::create_next(session::CreateInput {
                parent_id: Some(root.id.clone()),
                title: Some("mail child".into()),
                directory: directory.clone(),
                permission: None,
            })
            .await
            .unwrap();
            let c = ChildAgentCoordinator::new(128);
            c.register(&root.id, &child.id, "worker", "mail").unwrap();
            let sender = ToolUseContext::new(&root.id, Some(directory.clone()));
            let receiver = ToolUseContext::new(&child.id, Some(directory));
            let message = send_team_message::<Host, Host>(&c, &sender, &child.id, 7, TeamMessageKind::Message, "data".into())
                .await
                .unwrap();
            assert_eq!(message.sender_session, root.id);
            assert_eq!(message.turn, 7);
            assert_eq!(consume_team_messages::<Host, Host>(&c, &receiver).await.unwrap(), vec![message]);
            assert!(consume_team_messages::<Host, Host>(&c, &receiver).await.unwrap().is_empty());
            let tenant =
                vw_api_transport::TenantScope::new("other-platform", "other-company").unwrap();
            let result = session_store::ui_store::with_tenant_scope(
                tenant,
                send_team_message::<Host, Host>(&c, &sender, &child.id, 7, TeamMessageKind::Message, "bad".into()),
            )
            .await;
            assert!(result.unwrap_err().to_string().contains("tenant mismatch"));
            let unknown = ToolUseContext::new("unknown", None);
            assert!(
                send_team_message::<Host, Host>(&c, &unknown, &child.id, 7, TeamMessageKind::Message, "bad".into())
                    .await
                    .is_err()
            );
        })
    })
    .await
    .unwrap();
}

fn run_in_temporary_storage(test: &str) -> bool {
    const CHILD_TEST: &str = "SHIJU_HOST_MIGRATION_19_CHILD_TEST";
    if std::env::var(CHILD_TEST).as_deref() == Ok(test) {
        return false;
    }
    let root = tempfile::tempdir().expect("temporary storage root");
    let status = std::process::Command::new(std::env::current_exe().expect("test executable"))
        .arg(format!("{}::{test}", module_path!().split_once("::").expect("crate module").1))
        .args(["--exact", "--nocapture", "--test-threads=1"])
        .env(CHILD_TEST, test)
        .env("VIBEWINDOW_TEST_ROOT", root.path())
        .env("VIBEWINDOW_TEST_HOME", root.path())
        .status()
        .expect("isolated test process");
    assert!(status.success(), "isolated test failed: {test}: {status}");
    true
}
