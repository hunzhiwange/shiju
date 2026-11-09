//! Claude permissions exercise the shared ACP JSON-RPC connection and prompt barrier.
use super::*;

#[tokio::test]
async fn acp_full_access_claude_explicit_full_mode_requires_current_host_grant() {
    let agent = TestAgent::default();
    with_fixture(agent.clone(), async |fixture| {
        fixture.client.agent_name = "claude".into();
        let result = fixture
            .client
            .confirmed_config_control(
                &fixture.runtime.as_ref().unwrap().conn,
                "session",
                "mode",
                "bypassPermissions",
            )
            .await;
        assert!(result.is_err(), "Claude full mode must not bypass the host grant");
        assert!(agent.calls().is_empty(), "reject before sending a remote control");
    })
    .await;
}

#[tokio::test]
async fn acp_full_access_claude_deny_cannot_be_overridden_by_legacy_mode() {
    let agent = TestAgent::default();
    with_fixture(agent.clone(), async |fixture| {
        fixture.client.agent_name = "claude".into();
        fixture.client.full_access = true;
        fixture.client.permission_mode = PermissionMode::DenyAll;
        fixture.client.actor_state.lock().session_capabilities = Some(crate::SessionCapabilities {
            session_id: "session".into(),
            models: None,
            config_options: None,
            modes: Some(acp::SessionModeState::new(
                "default",
                vec![acp::SessionMode::new("bypassPermissions", "Full access")],
            )),
        });
        let result = fixture
            .client
            .confirmed_mode_control(
                &fixture.runtime.as_ref().unwrap().conn,
                "session",
                "bypassPermissions",
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
            ["default", "acceptEdits", "plan", "bypassPermissions"]
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

fn prepare_claude(fixture: &mut RuntimeFixture) {
    fixture.client.agent_name = "claude".into();
    let mut state = fixture.client.actor_state.lock();
    // The external process/identity boundary is replaced by this fixture only.
    state.claude_runtime_verified = true;
    state.session_capabilities = Some(crate::SessionCapabilities {
        session_id: "session".into(),
        models: None,
        modes: None,
        config_options: Some(options("default")),
    });
}

#[tokio::test]
async fn acp_full_access_claude_new_load_resume_reapply_current_grant_and_preserve_model() {
    let agent = TestAgent::default();
    agent.state.lock().claude_controls = true;
    with_fixture(agent.clone(), async |fixture| {
        prepare_claude(fixture);
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
            let desired = if enabled { "bypassPermissions" } else { "default" };
            assert_eq!(
                crate::access_policy::claude::current_mode(info.config_options.as_deref()),
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
async fn acp_full_access_claude_wrong_readback_blocks_prompt_and_cannot_claim_native() {
    let agent = TestAgent::default();
    agent.state.lock().claude_controls = true;
    agent.state.lock().wrong_config = true;
    with_fixture(agent, async |fixture| {
        prepare_claude(fixture);
        fixture.client.full_access = true;
        let result = fixture
            .client
            .sync_claude_access(&fixture.runtime.as_ref().unwrap().conn, "session")
            .await;
        assert!(result.is_err());
        assert!(fixture.client.ensure_session_controls_ready("session").is_err());
        assert_eq!(
            fixture.client.access_policy(None, Some(&options("bypassPermissions"))).support,
            crate::FullAccessSupport::ClientOnly
        );
    })
    .await;
}

#[tokio::test]
async fn acp_full_access_claude_missing_control_never_falls_back_to_mode() {
    let agent = TestAgent::default();
    with_fixture(agent.clone(), async |fixture| {
        prepare_claude(fixture);
        fixture.client.full_access = true;
        fixture.client.actor_state.lock().session_capabilities.as_mut().unwrap().config_options =
            None;
        assert!(
            fixture
                .client
                .sync_claude_access(&fixture.runtime.as_ref().unwrap().conn, "session")
                .await
                .is_err()
        );
        assert!(fixture.client.ensure_session_controls_ready("session").is_err());
        assert!(agent.calls().is_empty());
    })
    .await;
}

#[tokio::test]
async fn acp_full_access_claude_unknown_runtime_stays_client_only_without_native_requests() {
    let agent = TestAgent::default();
    with_fixture(agent.clone(), async |fixture| {
        fixture.client.agent_name = "claude".into();
        fixture.client.full_access = true;
        fixture
            .client
            .sync_claude_access(&fixture.runtime.as_ref().unwrap().conn, "session")
            .await
            .unwrap();
        assert_eq!(
            fixture.client.access_policy(None, Some(&options("bypassPermissions"))).support,
            crate::FullAccessSupport::ClientOnly
        );
        assert!(agent.calls().is_empty());
    })
    .await;
}

#[tokio::test]
async fn acp_full_access_claude_active_turn_mode_change_requests_cancel_and_rejects_control() {
    let agent = TestAgent::default();
    with_fixture(agent.clone(), async |fixture| {
        prepare_claude(fixture);
        let (tx, rx) = watch::channel(false);
        let (_, completed) = watch::channel(false);
        fixture.client.register_active_prompt("session".into(), tx, completed);
        let error = fixture
            .client
            .confirmed_config_control(
                &fixture.runtime.as_ref().unwrap().conn,
                "session",
                "mode",
                "default",
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
async fn acp_full_access_claude_host_deny_blocks_native_prompt_even_without_verified_runtime() {
    let agent = TestAgent::default();
    with_fixture(agent.clone(), async |fixture| {
        fixture.client.agent_name = "claude".into();
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
async fn acp_full_access_claude_read_approval_cannot_enable_native_full_access() {
    let agent = TestAgent::default();
    agent.state.lock().claude_controls = true;
    with_fixture(agent.clone(), async |fixture| {
        prepare_claude(fixture);
        fixture.client.full_access = true;
        fixture.client.permission_mode = PermissionMode::ApproveReads;
        fixture
            .client
            .sync_claude_access(&fixture.runtime.as_ref().unwrap().conn, "session")
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
        assert_eq!(crate::access_policy::claude::current_mode(options.as_deref()), Some("default"));
        assert_eq!(
            fixture.client.access_policy(None, options.as_deref()).support,
            crate::FullAccessSupport::Partial
        );
        assert_eq!(
            agent.calls(),
            vec![RecordedCall::SetConfig {
                session_id: "session".into(),
                option_name: "mode".into(),
                value_id: "default".into()
            }]
        );
    })
    .await;
}

#[tokio::test]
async fn acp_full_access_claude_process_exit_invalidates_native_claim() {
    let agent = TestAgent::default();
    agent.state.lock().claude_controls = true;
    with_fixture(agent, async |fixture| {
        prepare_claude(fixture);
        fixture.client.full_access = true;
        fixture
            .client
            .sync_claude_access(&fixture.runtime.as_ref().unwrap().conn, "session")
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
async fn acp_full_access_claude_unverified_handshake_clears_prior_runtime_identity() {
    with_fixture(TestAgent::default(), async |fixture| {
        prepare_claude(fixture);
        fixture
            .client
            .initialize_connection(&fixture.runtime.as_ref().unwrap().conn)
            .await
            .unwrap();
        assert!(!fixture.client.actor_state.lock().claude_runtime_verified);
        fixture.client.full_access = true;
        assert_eq!(
            fixture.client.access_policy(None, Some(&options("bypassPermissions"))).support,
            crate::FullAccessSupport::ClientOnly
        );
    })
    .await;
}

#[tokio::test]
async fn acp_full_access_claude_unverified_elevated_restore_refuses_to_continue() {
    let agent = TestAgent::default();
    with_fixture(agent.clone(), async |fixture| {
        for config_surface in [true, false] {
            prepare_claude(fixture);
            {
                let mut state = fixture.client.actor_state.lock();
                state.claude_runtime_verified = false;
                let info = state.session_capabilities.as_mut().unwrap();
                if config_surface {
                    info.config_options = Some(options("bypassPermissions"));
                } else {
                    info.config_options = None;
                    info.modes = Some(acp::SessionModeState::new(
                        "bypassPermissions",
                        vec![acp::SessionMode::new("bypassPermissions", "Full access")],
                    ));
                }
            }
            let result = fixture
                .client
                .sync_claude_access(&fixture.runtime.as_ref().unwrap().conn, "session")
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

#[tokio::test]
async fn acp_full_access_claude_plan_is_not_replaced_by_permission_sync() {
    let agent = TestAgent::default();
    agent.state.lock().claude_controls = true;
    with_fixture(agent.clone(), async |fixture| {
        prepare_claude(fixture);
        fixture.client.actor_state.lock().session_capabilities.as_mut().unwrap().config_options =
            Some(options("plan"));
        fixture
            .client
            .sync_claude_access(&fixture.runtime.as_ref().unwrap().conn, "session")
            .await
            .unwrap();
        assert!(agent.calls().is_empty(), "closing full access must not exit plan mode");
        fixture.client.full_access = true;
        assert!(
            fixture
                .client
                .sync_claude_access(&fixture.runtime.as_ref().unwrap().conn, "session")
                .await
                .is_err(),
            "full access cannot implicitly replace planning"
        );
        assert!(agent.calls().is_empty());
    })
    .await;
}

#[tokio::test]
async fn acp_full_access_claude_plan_confirmation_is_not_automatic_tool_approval() {
    use acp::Client as _;
    with_fixture(TestAgent::default(), async |fixture| {
        fixture.client.agent_name = "claude".into();
        fixture.client.full_access = true;
        let (tx, _) = mpsc::unbounded_channel();
        let event_client = fixture.client.build_event_client(Path::new("/tmp"), Arc::new(Mutex::new(None)), tx);
        let args: acp::RequestPermissionRequest = serde_json::from_value(serde_json::json!({
            "sessionId":"session", "toolCall":{"toolCallId":"plan", "title":"Exit plan mode", "kind":"switch_mode"},
            "options":[
                {"optionId":"bypassPermissions", "name":"Bypass", "kind":"allow_always"},
                {"optionId":"default", "name":"Default", "kind":"allow_once"},
                {"optionId":"acceptEdits", "name":"Edits", "kind":"allow_always"},
                {"optionId":"plan", "name":"Keep planning", "kind":"reject_once"}
            ]
        })).unwrap();
        assert!(event_client.request_permission(args).await.is_err(), "plan confirmation needs explicit interaction even with full access");
    }).await;
}

#[tokio::test]
async fn acp_full_access_claude_desktop_restores_default_instead_of_accept_edits() {
    let agent = TestAgent::default();
    agent.state.lock().claude_controls = true;
    with_fixture(agent.clone(), async |fixture| {
        prepare_claude(fixture);
        fixture.client.desktop_workspace = Some(PathBuf::from("/tmp/vw-acp-session-control"));
        fixture.client.permission_mode = PermissionMode::ApproveReads;
        fixture.client.actor_state.lock().session_capabilities.as_mut().unwrap().config_options =
            Some(options("acceptEdits"));
        let conn = &fixture.runtime.as_ref().unwrap().conn;
        assert_eq!(fixture.client.restored_permission_value("mode", "acceptEdits"), "default");
        assert!(
            fixture
                .client
                .confirmed_config_control(conn, "session", "mode", "acceptEdits")
                .await
                .is_err()
        );
        assert!(agent.calls().is_empty());
        fixture.client.sync_claude_access(conn, "session").await.unwrap();
        let state = fixture.client.actor_state.lock();
        let options = state.session_capabilities.as_ref().unwrap().config_options.as_deref();
        assert_eq!(crate::access_policy::claude::current_mode(options), Some("default"));
    })
    .await;
}

#[tokio::test]
async fn acp_full_access_claude_desktop_unverified_mode_fails_closed() {
    let agent = TestAgent::default();
    with_fixture(agent.clone(), async |fixture| {
        prepare_claude(fixture);
        fixture.client.desktop_workspace = Some(PathBuf::from("/tmp/vw-acp-session-control"));
        fixture.client.permission_mode = PermissionMode::ApproveReads;
        let conn = &fixture.runtime.as_ref().unwrap().conn;
        for mode in [Some("default"), Some("plan"), Some("auto"), Some("unknown"), None] {
            {
                let mut state = fixture.client.actor_state.lock();
                state.claude_runtime_verified = false;
                state.pending_controls.clear();
                state.session_capabilities.as_mut().unwrap().config_options = mode.map(options);
            }
            let result = fixture.client.sync_claude_access(conn, "session").await;
            let restricted = matches!(mode, Some("default" | "plan"));
            assert_eq!(result.is_ok(), restricted, "mode {mode:?}");
            assert_eq!(
                fixture.client.ensure_session_controls_ready("session").is_ok(),
                restricted,
            );
        }
        assert!(agent.calls().is_empty());
    })
    .await;
}

#[tokio::test]
async fn acp_full_access_claude_desktop_unknown_mode_cannot_be_set_or_restored() {
    let agent = TestAgent::default();
    with_fixture(agent.clone(), async |fixture| {
        prepare_claude(fixture);
        fixture.client.desktop_workspace = Some(PathBuf::from("/tmp/vw-acp-session-control"));
        fixture.client.permission_mode = PermissionMode::ApproveReads;
        let conn = &fixture.runtime.as_ref().unwrap().conn;
        assert_eq!(fixture.client.restored_permission_value("mode", "auto"), "default");
        assert_eq!(fixture.client.restored_permission_value("mode", "plan"), "plan");
        assert!(
            fixture.client.confirmed_config_control(conn, "session", "mode", "auto").await.is_err()
        );
        assert!(agent.calls().is_empty());
    })
    .await;
}
