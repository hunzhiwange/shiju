//! Gemini's legacy mode ACK cannot prove safe native permission revocation.
use super::*;

pub(super) fn modes(mode: &str) -> acp::SessionModeState {
    acp::SessionModeState::new(
        mode.to_string(),
        ["default", "autoEdit", "yolo", "plan"]
            .into_iter()
            .map(|value| acp::SessionMode::new(value, value))
            .collect(),
    )
}

fn prepare_gemini(fixture: &mut RuntimeFixture, mode: &str) {
    fixture.client.agent_name = "gemini".into();
    fixture.client.actor_state.lock().session_capabilities = Some(crate::SessionCapabilities {
        session_id: "session".into(),
        models: None,
        config_options: None,
        modes: Some(modes(mode)),
    });
}

#[tokio::test]
async fn acp_full_access_gemini_unconfirmed_native_modes_are_rejected_before_rpc() {
    for mode in ["yolo", "autoEdit"] {
        let agent = TestAgent::default();
        with_fixture(agent.clone(), async |fixture| {
            prepare_gemini(fixture, "default");
            fixture.client.full_access = true;
            let result = fixture
                .client
                .confirmed_mode_control(&fixture.runtime.as_ref().unwrap().conn, "session", mode)
                .await;
            assert!(result.is_err(), "an empty legacy ACK cannot confirm native access");
            assert!(agent.calls().is_empty());
        })
        .await;
    }
}

#[tokio::test]
async fn acp_full_access_gemini_elevated_restore_blocks_prompt_even_with_host_grant() {
    for mode in ["yolo", "autoEdit"] {
        let agent = TestAgent::default();
        with_fixture(agent.clone(), async |fixture| {
            prepare_gemini(fixture, mode);
            fixture.client.full_access = true;
            assert!(
                fixture
                    .client
                    .sync_native_access(&fixture.runtime.as_ref().unwrap().conn, "session",)
                    .await
                    .is_err(),
                "no safe readback/revocation for restored native mode"
            );
            assert!(agent.calls().is_empty());
        })
        .await;
    }
}

#[tokio::test]
async fn acp_full_access_gemini_default_and_plan_keep_client_only_without_mode_replacement() {
    for mode in ["default", "plan"] {
        let agent = TestAgent::default();
        with_fixture(agent.clone(), async |fixture| {
            prepare_gemini(fixture, mode);
            for enabled in [false, true, false] {
                fixture.client.full_access = enabled;
                fixture
                    .client
                    .sync_native_access(&fixture.runtime.as_ref().unwrap().conn, "session")
                    .await
                    .unwrap();
                let info = fixture.client.actor_state.lock().session_capabilities.clone().unwrap();
                assert_eq!(info.modes.as_ref().unwrap().current_mode_id.0.as_ref(), mode);
                assert_eq!(
                    fixture.client.access_policy(info.modes.as_ref(), None).support,
                    if enabled {
                        crate::FullAccessSupport::ClientOnly
                    } else {
                        crate::FullAccessSupport::Unavailable
                    }
                );
            }
            assert!(agent.calls().is_empty());
        })
        .await;
    }
}

#[tokio::test]
async fn acp_full_access_gemini_host_deny_prevents_native_prompt() {
    let agent = TestAgent::default();
    with_fixture(agent.clone(), async |fixture| {
        prepare_gemini(fixture, "default");
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
        assert!(agent.calls().is_empty());
    })
    .await;
}

#[test]
fn acp_full_access_gemini_launch_never_injects_unconfirmed_yolo_or_trust() {
    for command in ["gemini", "npx"] {
        let args = if command == "npx" {
            vec!["@google/gemini-cli@latest".into(), "--experimental-acp".into()]
        } else {
            vec!["--experimental-acp".into()]
        };
        let client = AcpClient::new(
            "gemini",
            AcpAgentConfig { command: command.into(), args: args.clone(), env: HashMap::new() },
        )
        .with_full_access(true);
        assert_eq!(client.launch_args().unwrap(), args);
    }
}

#[test]
fn acp_full_access_gemini_explicit_startup_bypass_is_rejected_even_with_grant() {
    let client = AcpClient::new(
        "gemini",
        AcpAgentConfig {
            command: "gemini".into(),
            args: vec!["--experimental-acp".into(), "--approval-mode".into(), "yolo".into()],
            env: HashMap::new(),
        },
    )
    .with_full_access(true);
    assert!(client.launch_args().is_err());
}

#[tokio::test]
async fn acp_full_access_gemini_new_load_and_resume_fallback_preserve_session_and_client_policy() {
    let agent = TestAgent::with_fail_resume();
    agent.state.lock().gemini_controls = true;
    with_fixture(agent.clone(), async |fixture| {
        fixture.client.agent_name = "gemini".into();
        let conn = &fixture.runtime.as_ref().unwrap().conn;
        let cwd = Path::new("/tmp/vw-acp-session-control");
        let info = fixture.client.new_session_info(conn, cwd).await.unwrap();
        for enabled in [false, true, false] {
            fixture.client.full_access = enabled;
            fixture.client.resolve_existing_session(conn, cwd, info.session_id.clone(),
                &fixture.runtime.as_ref().unwrap().expected_session_id).await.unwrap();
            let state = fixture.client.actor_state.lock();
            let caps = state.session_capabilities.as_ref().unwrap();
            assert_eq!(caps.session_id, info.session_id);
            assert_eq!(caps.modes.as_ref().unwrap().current_mode_id.0.as_ref(), "default");
            drop(state);
            assert_eq!(fixture.client.access_policy(Some(&modes("default")), None).support,
                if enabled { crate::FullAccessSupport::ClientOnly } else { crate::FullAccessSupport::Unavailable });
        }
        let calls = agent.calls();
        assert!(!calls.iter().any(|call| matches!(call, RecordedCall::SetMode { .. } | RecordedCall::SetConfig { .. })));
        assert!(calls.iter().any(|call| matches!(call, RecordedCall::LoadSession { session_id, .. } if session_id == &info.session_id)));
    }).await;
}

#[tokio::test]
async fn acp_full_access_gemini_missing_or_foreign_readback_refuses_prompt() {
    let agent = TestAgent::default();
    with_fixture(agent, async |fixture| {
        prepare_gemini(fixture, "default");
        fixture.client.actor_state.lock().session_capabilities.as_mut().unwrap().modes = None;
        assert!(
            fixture
                .client
                .sync_native_access(&fixture.runtime.as_ref().unwrap().conn, "session")
                .await
                .is_err()
        );
        prepare_gemini(fixture, "default");
        assert!(
            fixture
                .client
                .sync_native_access(&fixture.runtime.as_ref().unwrap().conn, "other-session")
                .await
                .is_err()
        );
    })
    .await;
}

#[tokio::test]
async fn acp_full_access_gemini_active_mode_change_cancels_before_rpc() {
    let agent = TestAgent::default();
    with_fixture(agent.clone(), async |fixture| {
        prepare_gemini(fixture, "default");
        let (tx, rx) = watch::channel(false);
        let (_, completed) = watch::channel(false);
        fixture.client.register_active_prompt("session".into(), tx, completed);
        assert!(
            fixture
                .client
                .confirmed_mode_control(&fixture.runtime.as_ref().unwrap().conn, "session", "plan")
                .await
                .is_err()
        );
        assert!(*rx.borrow());
        assert!(agent.calls().is_empty());
        fixture.client.clear_active_prompt("session");
    })
    .await;
}

#[tokio::test]
async fn acp_full_access_gemini_workflow_confirmation_is_not_automatic_tool_approval() {
    use acp::Client as _;
    with_fixture(TestAgent::default(), async |fixture| {
        fixture.client.agent_name = "gemini".into();
        fixture.client.full_access = true;
        let (tx, _) = mpsc::unbounded_channel();
        let event_client =
            fixture.client.build_event_client(Path::new("/tmp"), Arc::new(Mutex::new(None)), tx);
        // Gemini maps Plan and Communicate to Other, without a typed workflow payload.
        for kind in [Some("other"), Some("switch_mode"), None] {
            let mut call = serde_json::json!({"toolCallId":"workflow", "title":"Workflow"});
            if let Some(kind) = kind {
                call["kind"] = kind.into();
            }
            let args: acp::RequestPermissionRequest = serde_json::from_value(serde_json::json!({
                "sessionId":"session", "toolCall":call,
                "options":[
                    {"optionId":"proceed_once", "name":"Allow", "kind":"allow_once"},
                    {"optionId":"cancel", "name":"Reject", "kind":"reject_once"}
                ]
            }))
            .unwrap();
            assert!(
                event_client.request_permission(args).await.is_err(),
                "untyped plan/question confirmations need explicit interaction"
            );
        }
    })
    .await;
}
