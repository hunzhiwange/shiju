//! Codex permissions exercise the shared ACP JSON-RPC connection and prompt barrier.
use super::*;

#[tokio::test]
async fn acp_full_access_codex_explicit_full_mode_requires_current_host_grant() {
    let agent = TestAgent::default();
    with_fixture(agent.clone(), async |fixture| {
        fixture.client.agent_name = "codex".into();
        let result = fixture
            .client
            .confirmed_config_control(
                &fixture.runtime.as_ref().unwrap().conn,
                "session",
                "mode",
                "agent-full-access",
            )
            .await;
        assert!(result.is_err(), "Codex full mode must not bypass the host grant");
        assert!(agent.calls().is_empty(), "reject before sending a remote control");
    })
    .await;
}

#[tokio::test]
async fn acp_full_access_codex_deny_cannot_be_overridden_by_legacy_mode() {
    let agent = TestAgent::default();
    with_fixture(agent.clone(), async |fixture| {
        fixture.client.agent_name = "codex".into();
        fixture.client.full_access = true;
        fixture.client.permission_mode = PermissionMode::DenyAll;
        fixture.client.actor_state.lock().session_capabilities = Some(crate::SessionCapabilities {
            session_id: "session".into(),
            models: None,
            config_options: None,
            modes: Some(acp::SessionModeState::new(
                "read-only",
                vec![acp::SessionMode::new("agent-full-access", "Full access")],
            )),
        });
        let result = fixture
            .client
            .confirmed_mode_control(
                &fixture.runtime.as_ref().unwrap().conn,
                "session",
                "agent-full-access",
            )
            .await;
        assert!(result.is_err(), "host deny must dominate the native mode");
        assert!(agent.calls().is_empty());
    })
    .await;
}

pub(super) fn options(mode: &str) -> Vec<acp::SessionConfigOption> {
    vec![
        acp::SessionConfigOption::select(
            "mode",
            "Mode",
            mode.to_string(),
            ["read-only", "workspace-write", "agent", "agent-full-access"]
                .into_iter()
                .map(|value| acp::SessionConfigSelectOption::new(value, value))
                .collect::<Vec<_>>(),
        ),
        acp::SessionConfigOption::select(
            "model",
            "Model",
            "test-model",
            vec![acp::SessionConfigSelectOption::new("test-model", "Model")],
        ),
    ]
}

fn prepare_codex(fixture: &mut RuntimeFixture) {
    fixture.client.agent_name = "codex".into();
    let mut state = fixture.client.actor_state.lock();
    // The external process/identity boundary is replaced by this fixture only.
    state.codex_runtime_verified = true;
    state.session_capabilities = Some(crate::SessionCapabilities {
        session_id: "session".into(),
        models: None,
        modes: None,
        config_options: Some(options("agent")),
    });
}

#[tokio::test]
async fn acp_full_access_codex_new_load_resume_reapply_current_grant_and_preserve_model() {
    let agent = TestAgent::default();
    agent.state.lock().codex_controls = true;
    with_fixture(agent.clone(), async |fixture| {
        prepare_codex(fixture);
        fixture.client.desktop_workspace = Some(Path::new("/tmp/vw-acp-session-control").into());
        let conn = &fixture.runtime.as_ref().unwrap().conn;
        let cwd = Path::new("/tmp/vw-acp-session-control");
        let mut original_id = None;
        for (index, enabled) in [false, true, false].into_iter().enumerate() {
            fixture.client.full_access = enabled;
            let info = match index {
                0 => fixture.client.new_session_info(conn, cwd).await.unwrap(),
                1 => fixture
                    .client
                    .load_session_info(conn, cwd, original_id.clone().unwrap())
                    .await
                    .unwrap(),
                _ => fixture
                    .client
                    .resume_session_info(conn, cwd, original_id.clone().unwrap())
                    .await
                    .unwrap(),
            };
            if original_id.is_none() {
                original_id = Some(info.session_id.clone());
            }
            assert_eq!(Some(&info.session_id), original_id.as_ref());
            let desired = if enabled { "agent-full-access" } else { "read-only" };
            assert_eq!(
                crate::access_policy::codex::current_mode(info.config_options.as_deref()),
                Some(desired)
            );
            assert_eq!(info.config_options.as_ref().unwrap()[1], options(desired)[1]);
            let policy = fixture.client.access_policy(None, info.config_options.as_deref());
            assert_eq!(
                policy.support,
                if enabled {
                    crate::FullAccessSupport::Native
                } else {
                    crate::FullAccessSupport::Unavailable
                }
            );
        }
        let calls = agent.calls();
        assert_eq!(
            calls.iter().filter(|call| matches!(call, RecordedCall::SetConfig { .. })).count(),
            3
        );
    })
    .await;
}

#[tokio::test]
async fn acp_full_access_codex_wrong_readback_blocks_prompt_and_cannot_claim_native() {
    let agent = TestAgent::default();
    agent.state.lock().codex_controls = true;
    agent.state.lock().wrong_config = true;
    with_fixture(agent, async |fixture| {
        prepare_codex(fixture);
        fixture.client.full_access = true;
        let result = fixture
            .client
            .sync_codex_access(&fixture.runtime.as_ref().unwrap().conn, "session")
            .await;
        assert!(result.is_err());
        assert!(fixture.client.ensure_session_controls_ready("session").is_err());
        assert_eq!(
            fixture.client.access_policy(None, Some(&options("agent-full-access"))).support,
            crate::FullAccessSupport::ClientOnly
        );
    })
    .await;
}

#[tokio::test]
async fn acp_full_access_codex_missing_control_never_falls_back_to_mode() {
    let agent = TestAgent::default();
    with_fixture(agent.clone(), async |fixture| {
        prepare_codex(fixture);
        fixture.client.full_access = true;
        fixture.client.actor_state.lock().session_capabilities.as_mut().unwrap().config_options =
            None;
        assert!(
            fixture
                .client
                .sync_codex_access(&fixture.runtime.as_ref().unwrap().conn, "session")
                .await
                .is_err()
        );
        assert!(fixture.client.ensure_session_controls_ready("session").is_err());
        assert!(agent.calls().is_empty());
    })
    .await;
}

#[tokio::test]
async fn acp_full_access_codex_unknown_runtime_stays_client_only_without_native_requests() {
    let agent = TestAgent::default();
    with_fixture(agent.clone(), async |fixture| {
        fixture.client.agent_name = "codex".into();
        fixture.client.full_access = true;
        fixture
            .client
            .sync_codex_access(&fixture.runtime.as_ref().unwrap().conn, "session")
            .await
            .unwrap();
        assert_eq!(
            fixture.client.access_policy(None, Some(&options("agent-full-access"))).support,
            crate::FullAccessSupport::ClientOnly
        );
        assert!(agent.calls().is_empty());
    })
    .await;
}

#[tokio::test]
async fn acp_full_access_codex_active_turn_mode_change_requests_cancel_and_rejects_control() {
    let agent = TestAgent::default();
    with_fixture(agent.clone(), async |fixture| {
        prepare_codex(fixture);
        let (tx, rx) = watch::channel(false);
        let (_, completed) = watch::channel(false);
        fixture.client.register_active_prompt("session".into(), tx, completed);
        let error = fixture
            .client
            .confirmed_config_control(
                &fixture.runtime.as_ref().unwrap().conn,
                "session",
                "mode",
                "read-only",
            )
            .await
            .unwrap_err();
        assert!(error.to_string().contains("cannot revoke an active turn"));
        assert!(*rx.borrow());
        assert!(fixture.client.ensure_session_controls_ready("session").is_err());
        assert!(agent.calls().is_empty());
        fixture.client.clear_active_prompt("session");
    })
    .await;
}

#[tokio::test]
async fn acp_full_access_codex_host_deny_blocks_native_prompt_even_without_verified_runtime() {
    let agent = TestAgent::default();
    with_fixture(agent.clone(), async |fixture| {
        fixture.client.agent_name = "codex".into();
        fixture.client.full_access = true;
        fixture.client.permission_mode = PermissionMode::DenyAll;
        let (tx, _) = mpsc::unbounded_channel();
        let result = fixture
            .client
            .run_actor_prompt(
                fixture.runtime.as_mut().unwrap(),
                PromptRequest::new(PathBuf::from("/tmp/vw-acp-session-control"), "do work"),
                tx,
            )
            .await;
        assert!(result.unwrap_err().to_string().contains("Host deny"));
        assert!(!agent.calls().iter().any(|call| matches!(call, RecordedCall::SetConfig { .. })));
    })
    .await;
}

#[tokio::test]
async fn acp_full_access_codex_read_approval_cannot_enable_native_full_access() {
    let agent = TestAgent::default();
    agent.state.lock().codex_controls = true;
    with_fixture(agent.clone(), async |fixture| {
        prepare_codex(fixture);
        fixture.client.desktop_workspace = Some(Path::new("/tmp/vw-acp-session-control").into());
        fixture.client.full_access = true;
        fixture.client.permission_mode = PermissionMode::ApproveReads;
        fixture
            .client
            .sync_codex_access(&fixture.runtime.as_ref().unwrap().conn, "session")
            .await
            .unwrap();
        let options = fixture
            .client
            .actor_state
            .lock()
            .session_capabilities
            .as_ref()
            .unwrap()
            .config_options
            .clone();
        assert_eq!(
            crate::access_policy::codex::current_mode(options.as_deref()),
            Some("read-only")
        );
        assert_eq!(
            fixture.client.access_policy(None, options.as_deref()).support,
            crate::FullAccessSupport::Partial
        );
        assert_eq!(
            agent.calls(),
            vec![RecordedCall::SetConfig {
                session_id: "session".into(),
                option_name: "mode".into(),
                value_id: "read-only".into()
            }]
        );
    })
    .await;
}

#[tokio::test]
async fn acp_full_access_codex_process_exit_invalidates_native_claim() {
    let agent = TestAgent::default();
    agent.state.lock().codex_controls = true;
    with_fixture(agent, async |fixture| {
        prepare_codex(fixture);
        fixture.client.full_access = true;
        fixture
            .client
            .sync_codex_access(&fixture.runtime.as_ref().unwrap().conn, "session")
            .await
            .unwrap();
        let options = fixture
            .client
            .actor_state
            .lock()
            .session_capabilities
            .as_ref()
            .unwrap()
            .config_options
            .clone();
        assert_eq!(
            fixture.client.access_policy(None, options.as_deref()).support,
            crate::FullAccessSupport::Native
        );
        fixture.client.record_actor_exit(ChildExitSummary::default(), Some("test"), false);
        assert_eq!(
            fixture.client.access_policy(None, options.as_deref()).support,
            crate::FullAccessSupport::ClientOnly
        );
    })
    .await;
}

#[tokio::test]
async fn acp_full_access_codex_unverified_handshake_clears_prior_runtime_identity() {
    with_fixture(TestAgent::default(), async |fixture| {
        prepare_codex(fixture);
        fixture
            .client
            .initialize_connection(&fixture.runtime.as_ref().unwrap().conn)
            .await
            .unwrap();
        assert!(!fixture.client.actor_state.lock().codex_runtime_verified);
        fixture.client.full_access = true;
        assert_eq!(
            fixture.client.access_policy(None, Some(&options("agent-full-access"))).support,
            crate::FullAccessSupport::ClientOnly
        );
    })
    .await;
}

#[tokio::test]
async fn acp_full_access_codex_unverified_elevated_restore_refuses_to_continue() {
    let agent = TestAgent::default();
    with_fixture(agent.clone(), async |fixture| {
        for (mode, config_surface) in [
            ("agent-full-access", true),
            ("agent-full-access", false),
            ("workspace-write", true),
            ("agent", false),
        ] {
            prepare_codex(fixture);
            if mode != "agent-full-access" {
                fixture.client.desktop_workspace =
                    Some(Path::new("/tmp/vw-acp-session-control").into());
            }
            {
                let mut state = fixture.client.actor_state.lock();
                state.codex_runtime_verified = false;
                let info = state.session_capabilities.as_mut().unwrap();
                if config_surface {
                    info.config_options = Some(options(mode));
                } else {
                    info.config_options = None;
                    info.modes = Some(acp::SessionModeState::new(
                        mode,
                        vec![acp::SessionMode::new(mode, mode)],
                    ));
                }
            }
            let result = fixture
                .client
                .sync_codex_access(&fixture.runtime.as_ref().unwrap().conn, "session")
                .await;
            assert!(
                result.is_err(),
                "an unverified elevated restore has no safe automatic revocation path"
            );
            assert!(fixture.client.ensure_session_controls_ready("session").is_err());
            assert!(agent.calls().is_empty(), "do not guess controls for an unverified runtime");
        }
    })
    .await;
}
