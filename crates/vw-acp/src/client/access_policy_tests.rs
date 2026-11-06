//! 使用真实 ACP JSON-RPC 内存连接验证控制响应，供应商权限能力另行验收。
use super::*;

fn capabilities(id: &str) -> crate::SessionCapabilities {
    crate::SessionCapabilities {
        session_id: id.into(),
        modes: Some(acp::SessionModeState::new(
            "default",
            vec![acp::SessionMode::new("high", "High")],
        )),
        models: None,
        config_options: Some(vec![acp::SessionConfigOption::select(
            "access",
            "Access",
            "default",
            vec![acp::SessionConfigSelectOption::new("high", "High")],
        )]),
    }
}

#[tokio::test]
async fn acp_full_access_protocol_wrong_value_is_rejected_without_changing_effective_state() {
    let agent = TestAgent::default();
    agent.state.lock().wrong_config = true;
    with_fixture(agent, async |fixture| {
        fixture.client.store_reusable_session(Some("session".into()));
        *fixture.runtime.as_ref().unwrap().expected_session_id.lock() = Some("session".into());
        fixture.client.actor_state.lock().session_capabilities = Some(capabilities("session"));
        let result = fixture
            .client
            .actor_set_session_config_option(
                &mut fixture.runtime,
                "session".into(),
                PathBuf::from("/tmp/vw-acp-session-control"),
                "access".into(),
                "high".into(),
            )
            .await;
        assert!(result.is_err(), "a successful RPC with the wrong value is not confirmation");
        let state = fixture.client.actor_state.lock();
        assert_eq!(
            state.session_capabilities.as_ref().unwrap().config_options,
            capabilities("session").config_options
        );
    })
    .await;
}

#[tokio::test]
async fn acp_full_access_protocol_empty_mode_response_does_not_fabricate_readback() {
    with_fixture(TestAgent::default(), async |fixture| {
        fixture.client.store_reusable_session(Some("session".into()));
        *fixture.runtime.as_ref().unwrap().expected_session_id.lock() = Some("session".into());
        fixture.client.actor_state.lock().session_capabilities = Some(capabilities("session"));
        fixture
            .client
            .actor_set_session_mode(
                &mut fixture.runtime,
                "session".into(),
                PathBuf::from("/tmp/vw-acp-session-control"),
                "high".into(),
            )
            .await
            .unwrap();
        assert_eq!(
            fixture
                .client
                .actor_state
                .lock()
                .session_capabilities
                .as_ref()
                .unwrap()
                .modes
                .as_ref()
                .unwrap()
                .current_mode_id
                .to_string(),
            "default"
        );
    })
    .await;
}

async fn prepare(fixture: &mut RuntimeFixture) {
    fixture.client.store_reusable_session(Some("session".into()));
    *fixture.runtime.as_ref().unwrap().expected_session_id.lock() = Some("session".into());
    fixture.client.actor_state.lock().session_capabilities = Some(capabilities("session"));
}

#[tokio::test]
async fn acp_full_access_protocol_config_precedes_mode_and_confirms_value() {
    let agent = TestAgent::default();
    with_fixture(agent.clone(), async |fixture| {
        prepare(fixture).await;
        fixture.client.actor_state.lock().session_capabilities.as_mut().unwrap().config_options.as_mut().unwrap()[0].category = Some(acp::SessionConfigOptionCategory::Mode);
        fixture.client.actor_set_session_mode(&mut fixture.runtime, "session".into(), PathBuf::from("/tmp/vw-acp-session-control"), "high".into()).await.unwrap();
        assert_eq!(agent.calls(), vec![RecordedCall::SetConfig { session_id: "session".into(), option_name: "access".into(), value_id: "high".into() }]);
        assert!(fixture.client.ensure_session_controls_ready("session").is_ok());
        let state = fixture.client.actor_state.lock();
        let option = &state.session_capabilities.as_ref().unwrap().config_options.as_ref().unwrap()[0];
        assert!(matches!(&option.kind, acp::SessionConfigKind::Select(select) if select.current_value.to_string() == "high"));
        assert_eq!(fixture.client.access_policy(None, Some(std::slice::from_ref(option))).support, crate::FullAccessSupport::Unavailable);
    }).await;
}

#[tokio::test]
async fn acp_full_access_protocol_rejection_blocks_prompt_until_successful_retry() {
    let agent = TestAgent::default();
    agent.state.lock().fail_set_config = true;
    with_fixture(agent.clone(), async |fixture| {
        prepare(fixture).await;
        assert!(
            fixture
                .client
                .actor_set_session_config_option(
                    &mut fixture.runtime,
                    "session".into(),
                    PathBuf::from("/tmp/vw-acp-session-control"),
                    "access".into(),
                    "high".into()
                )
                .await
                .is_err()
        );
        let (tx, _) = mpsc::unbounded_channel();
        let request = PromptRequest::new(PathBuf::from("/tmp/vw-acp-session-control"), "hello")
            .with_session_strategy(SessionStrategy::ResumeOrLoad("session".into()));
        let err = fixture
            .client
            .run_actor_prompt(fixture.runtime.as_mut().unwrap(), request, tx)
            .await
            .unwrap_err();
        assert!(err.to_string().contains("retry the failed control"));
        agent.state.lock().fail_set_config = false;
        fixture
            .client
            .actor_set_session_config_option(
                &mut fixture.runtime,
                "session".into(),
                PathBuf::from("/tmp/vw-acp-session-control"),
                "access".into(),
                "high".into(),
            )
            .await
            .unwrap();
        assert!(fixture.client.ensure_session_controls_ready("session").is_ok());
        assert_eq!(agent.calls().len(), 2, "blocked prompt must not restore or reach the agent");
    })
    .await;
}

#[tokio::test]
async fn acp_full_access_protocol_timeout_keeps_control_pending() {
    let agent = TestAgent::default();
    agent.state.lock().delay_config = true;
    with_fixture(agent, async |fixture| {
        prepare(fixture).await;
        let err = fixture
            .client
            .actor_set_session_config_option(
                &mut fixture.runtime,
                "session".into(),
                PathBuf::from("/tmp/vw-acp-session-control"),
                "access".into(),
                "high".into(),
            )
            .await
            .unwrap_err();
        assert!(err.to_string().contains("timed out"));
        assert!(fixture.client.ensure_session_controls_ready("session").is_err());
    })
    .await;
}

#[tokio::test]
async fn acp_full_access_protocol_cancellation_never_starts_another_restore() {
    let agent = TestAgent::default();
    with_fixture(agent.clone(), async |fixture| {
        fixture.client.cancelling_session_ids.lock().insert("session".into());
        let runtime = fixture.runtime.as_ref().unwrap();
        assert!(
            fixture
                .client
                .resolve_existing_session(
                    &runtime.conn,
                    &runtime.cwd,
                    "session".into(),
                    &runtime.expected_session_id
                )
                .await
                .is_err()
        );
        assert!(agent.calls().is_empty());
    })
    .await;
}

#[tokio::test]
async fn acp_full_access_protocol_response_does_not_change_another_session() {
    with_fixture(TestAgent::default(), async |fixture| {
        prepare(fixture).await;
        fixture.client.actor_state.lock().session_capabilities = Some(capabilities("other"));
        fixture
            .client
            .actor_set_session_config_option(
                &mut fixture.runtime,
                "session".into(),
                PathBuf::from("/tmp/vw-acp-session-control"),
                "access".into(),
                "high".into(),
            )
            .await
            .unwrap();
        assert_eq!(
            fixture.client.actor_state.lock().session_capabilities.as_ref().unwrap().config_options,
            capabilities("other").config_options
        );
    })
    .await;
}

#[tokio::test]
async fn acp_full_access_protocol_rejected_config_never_falls_back_to_mode() {
    let agent = TestAgent::default();
    agent.state.lock().fail_set_config = true;
    with_fixture(agent.clone(), async |fixture| {
        prepare(fixture).await;
        fixture
            .client
            .actor_state
            .lock()
            .session_capabilities
            .as_mut()
            .unwrap()
            .config_options
            .as_mut()
            .unwrap()[0]
            .category = Some(acp::SessionConfigOptionCategory::Mode);
        assert!(
            fixture
                .client
                .actor_set_session_mode(
                    &mut fixture.runtime,
                    "session".into(),
                    PathBuf::from("/tmp/vw-acp-session-control"),
                    "high".into()
                )
                .await
                .is_err()
        );
        assert_eq!(
            agent.calls(),
            vec![RecordedCall::SetConfig {
                session_id: "session".into(),
                option_name: "access".into(),
                value_id: "high".into()
            }]
        );
        assert!(fixture.client.ensure_session_controls_ready("session").is_err());
    })
    .await;
}

#[tokio::test]
async fn acp_full_access_protocol_unverified_controls_remain_client_only() {
    with_fixture(TestAgent::default(), async |fixture| {
        fixture.client.full_access = true;
        let info = fixture
            .client
            .actor_create_session(
                &mut fixture.runtime,
                PathBuf::from("/tmp/vw-acp-session-control"),
            )
            .await
            .unwrap();
        let advertised = capabilities(&info.session_id);
        let policy = fixture
            .client
            .access_policy(advertised.modes.as_ref(), advertised.config_options.as_deref());
        assert_eq!(policy.support, crate::FullAccessSupport::ClientOnly);
        assert_eq!(policy.reason, crate::AccessPolicyReason::UnverifiedControls);
    })
    .await;
}

#[tokio::test]
async fn acp_full_access_protocol_stale_notifications_cannot_retarget_session() {
    use acp::Client as _;
    with_fixture(TestAgent::default(), async |fixture| {
        prepare(fixture).await;
        for id in ["other", "session"] {
            fixture
                .agent_conn
                .session_notification(acp::SessionNotification::new(
                    id,
                    acp::SessionUpdate::CurrentModeUpdate(acp::CurrentModeUpdate::new("high")),
                ))
                .await
                .unwrap();
            // A request on the same ordered stream ensures the preceding notification
            // was consumed before inspecting the callback state.
            fixture
                .client
                .confirmed_config_control(
                    &fixture.runtime.as_ref().unwrap().conn,
                    "session",
                    "access",
                    "high",
                )
                .await
                .unwrap();
            assert_eq!(expected_session_id(&fixture.runtime).as_deref(), Some("session"));
            assert_eq!(
                fixture
                    .client
                    .actor_state
                    .lock()
                    .session_capabilities
                    .as_ref()
                    .unwrap()
                    .modes
                    .as_ref()
                    .unwrap()
                    .current_mode_id
                    .to_string(),
                if id == "other" { "default" } else { "high" }
            );
        }
    })
    .await;
}
