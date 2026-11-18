use super::*;

#[tokio::test]
async fn managed_corust_does_not_switch_out_of_tool_approval_mode() {
    let agent = TestAgent::default();
    with_fixture(agent.clone(), async |fixture| {
        fixture.client.agent_name = "corust-agent".into();
        fixture.client.full_access = true;
        fixture.client.permission_mode = PermissionMode::ApproveAll;
        fixture.client.config.env.insert("VW_ACP_LAUNCH_REFERENCE".into(), "fixture".into());
        fixture.client.actor_state.lock().session_capabilities = Some(crate::SessionCapabilities {
            session_id: "session".into(),
            models: None,
            config_options: None,
            modes: Some(acp::SessionModeState::new(
                "default",
                vec![acp::SessionMode::new("yolo", "Full access")],
            )),
        });
        let result = fixture
            .client
            .confirmed_mode_control(&fixture.runtime.as_ref().unwrap().conn, "session", "yolo")
            .await;
        assert!(result.is_err());
        assert!(agent.calls().is_empty());
        assert_eq!(fixture.client.restored_permission_value("mode", "yolo"), "default");
    })
    .await;
}
