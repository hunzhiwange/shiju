use super::*;

fn prepare(f: &mut RuntimeFixture, full: bool) {
    f.client.agent_name = "dimcode".into();
    f.client.full_access = full;
    f.client.permission_mode = PermissionMode::ApproveAll;
    f.client.actor_state.lock().session_capabilities = Some(crate::SessionCapabilities {
        session_id: "session".into(),
        modes: None,
        models: None,
        config_options: Some(vec![acp::SessionConfigOption::select(
            "permission",
            "Permission",
            "read-only",
            ["read-only", "workspace-write", "full-access"]
                .into_iter()
                .map(|v| acp::SessionConfigSelectOption::new(v, v))
                .collect::<Vec<_>>(),
        )]),
    });
}

#[tokio::test]
async fn dimcode_full_grant_enables_host_mediated_writes() {
    let agent = TestAgent::default();
    with_fixture(agent.clone(), async |f| {
        prepare(f, true);
        f.client.sync_native_access(&f.runtime.as_ref().unwrap().conn, "session").await.unwrap();
        assert_eq!(
            agent.calls(),
            vec![RecordedCall::SetConfig {
                session_id: "session".into(),
                option_name: "permission".into(),
                value_id: "workspace-write".into()
            }]
        );
    })
    .await;
}

#[tokio::test]
async fn dimcode_never_accepts_native_full_access_or_ungranted_writes() {
    with_fixture(TestAgent::default(), async |f| {
        for (full, value) in [(true, "full-access"), (false, "workspace-write")] {
            prepare(f, full);
            assert!(
                f.client
                    .confirmed_config_control(
                        &f.runtime.as_ref().unwrap().conn,
                        "session",
                        "permission",
                        value
                    )
                    .await
                    .is_err()
            );
        }
    })
    .await;
}

#[tokio::test]
async fn dimcode_revoked_saved_permission_is_read_only() {
    with_fixture(TestAgent::default(), async |f| {
        prepare(f, false);
        assert_eq!(
            f.client.restored_permission_value("permission", "workspace-write"),
            "read-only"
        );
        assert_eq!(f.client.restricted_permission_value("permission", "full-access"), "read-only");
    })
    .await;
}

#[tokio::test]
async fn dimcode_unconfirmed_control_blocks_prompt() {
    let agent = TestAgent::default();
    agent.state.lock().wrong_config = true;
    with_fixture(agent, async |f| {
        prepare(f, true);
        assert!(
            f.client
                .sync_native_access(&f.runtime.as_ref().unwrap().conn, "session")
                .await
                .is_err()
        );
        assert!(f.client.ensure_session_controls_ready("session").is_err());
    })
    .await;
}

#[tokio::test]
async fn dimcode_no_grant_synchronizes_read_only() {
    let agent = TestAgent::default();
    with_fixture(agent.clone(), async |f| {
        prepare(f, false);
        f.client.sync_native_access(&f.runtime.as_ref().unwrap().conn, "session").await.unwrap();
        assert_eq!(
            agent.calls(),
            vec![RecordedCall::SetConfig {
                session_id: "session".into(),
                option_name: "permission".into(),
                value_id: "read-only".into()
            }]
        );
    })
    .await;
}
