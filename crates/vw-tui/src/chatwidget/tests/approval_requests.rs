//! 聚焦验证执行、网络与文件系统审批请求的展示和决策事件。
//! 这些测试从主 chatwidget 测试文件拆出，以满足源文件体积策略限制。
//! Approval-request focused tests extracted from the main chatwidget test file
//! to keep the primary module under blob-size policy limits.
//! 测试同时核对命令规范化、权限上下文保留和历史摘要截断边界。

use super::*;
use pretty_assertions::assert_eq;

#[tokio::test]
async fn exec_approval_emits_proposed_command_and_decision_history() {
    // 执行审批必须同时保留提议命令和最终决策，形成可审计历史。
    let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;

    // 以简短单行命令触发执行审批请求。
    // Trigger an exec approval request with a short, single-line command.
    let ev = ExecApprovalRequestEvent {
        call_id: "call-short".into(),
        approval_id: Some("call-short".into()),
        turn_id: "turn-short".into(),
        environment_id: Some("remote".to_string()),
        command: vec!["bash".into(), "-lc".into(), "echo hello world".into()],
        cwd: AbsolutePathBuf::current_dir().expect("current dir"),
        reason: Some(
            "this is a test reason such as one that would be produced by the model".into(),
        ),
        network_approval_context: None,
        proposed_execpolicy_amendment: None,
        proposed_network_policy_amendments: None,
        additional_permissions: None,
        available_decisions: None,
    };
    handle_exec_approval_request(&mut chat, "sub-short", ev);

    let proposed_cells = drain_insert_history(&mut rx);
    assert!(
        proposed_cells.is_empty(),
        "expected approval request to render via modal without emitting history cells"
    );

    let area = Rect::new(0, 0, 80, chat.desired_height(/*width*/ 80));
    let mut buf = ratatui::buffer::Buffer::empty(area);
    chat.render(area, &mut buf);
    assert_snapshot!("exec_approval_modal_exec", format!("{buf:?}"));

    chat.handle_key_event(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE));
    let decision = drain_insert_history(&mut rx)
        .pop()
        .expect("expected decision cell in history");
    assert_snapshot!(
        "exec_approval_history_decision_approved_short",
        lines_to_single_string(&decision)
    );
}

#[test]
fn app_server_exec_approval_request_splits_shell_wrapped_command() {
    // app-server 的 shell 包装命令拆分后仍应呈现真实命令边界。
    let script = r#"python3 -c 'print("Hello, world!")'"#;
    let request = exec_approval_request_from_params(
        AppServerCommandExecutionRequestApprovalParams {
            thread_id: "thread-1".to_string(),
            turn_id: "turn-1".to_string(),
            item_id: "item-1".to_string(),
            started_at_ms: 0,
            approval_id: Some("approval-1".to_string()),
            environment_id: None,
            reason: None,
            network_approval_context: None,
            command: Some(
                shlex::try_join(["/bin/zsh", "-lc", script])
                    .expect("round-trippable shell wrapper"),
            ),
            cwd: Some(test_path_buf("/tmp").abs().into()),
            command_actions: None,
            additional_permissions: None,
            proposed_execpolicy_amendment: None,
            proposed_network_policy_amendments: None,
            available_decisions: None,
        },
        &test_path_buf("/tmp").abs(),
    );

    assert_eq!(
        request.command,
        vec![
            "/bin/zsh".to_string(),
            "-lc".to_string(),
            script.to_string(),
        ]
    );
}

#[test]
fn app_server_exec_approval_request_preserves_permissions_context() {
    // 审批请求携带的权限上下文必须原样进入弹窗和后续决策路径。
    let read_path = AbsolutePathBuf::try_from(PathBuf::from(test_path_display("/tmp/read-only")))
        .expect("absolute read path");
    let write_path = AbsolutePathBuf::try_from(PathBuf::from(test_path_display("/tmp/write")))
        .expect("absolute write path");
    let read_api_path = LegacyAppPathString::from_abs_path(&read_path);
    let write_api_path = LegacyAppPathString::from_abs_path(&write_path);
    let request = exec_approval_request_from_params(
        AppServerCommandExecutionRequestApprovalParams {
            thread_id: "thread-1".to_string(),
            turn_id: "turn-1".to_string(),
            item_id: "item-1".to_string(),
            started_at_ms: 0,
            approval_id: Some("approval-1".to_string()),
            environment_id: None,
            reason: None,
            network_approval_context: Some(codex_app_server_protocol::NetworkApprovalContext {
                host: "example.com".to_string(),
                protocol: codex_app_server_protocol::NetworkApprovalProtocol::Socks5Tcp,
            }),
            command: Some("ls".to_string()),
            cwd: Some(test_path_buf("/tmp").abs().into()),
            command_actions: None,
            additional_permissions: Some(AppServerAdditionalPermissionProfile {
                network: Some(AppServerAdditionalNetworkPermissions {
                    enabled: Some(true),
                }),
                file_system: Some(AppServerAdditionalFileSystemPermissions {
                    read: Some(vec![read_api_path.clone()]),
                    write: Some(vec![write_api_path.clone()]),
                    glob_scan_max_depth: None,
                    entries: None,
                }),
            }),
            proposed_execpolicy_amendment: None,
            proposed_network_policy_amendments: None,
            available_decisions: None,
        },
        &test_path_buf("/tmp").abs(),
    );

    assert_eq!(
        request.network_approval_context,
        Some(codex_app_server_protocol::NetworkApprovalContext {
            host: "example.com".to_string(),
            protocol: codex_app_server_protocol::NetworkApprovalProtocol::Socks5Tcp,
        })
    );
    assert_eq!(
        request.additional_permissions,
        Some(AppServerAdditionalPermissionProfile {
            network: Some(AppServerAdditionalNetworkPermissions {
                enabled: Some(true),
            }),
            file_system: Some(AppServerAdditionalFileSystemPermissions {
                read: Some(vec![read_api_path]),
                write: Some(vec![write_api_path]),
                glob_scan_max_depth: None,
                entries: None,
            }),
        })
    );
}

#[tokio::test]
async fn network_exec_approval_history_describes_session_host_allowance() {
    // 会话级主机放行在历史中明确标记其持续范围。
    let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    let request = exec_approval_request_from_params(
        AppServerCommandExecutionRequestApprovalParams {
            thread_id: "thread-1".to_string(),
            turn_id: "turn-1".to_string(),
            item_id: "item-1".to_string(),
            started_at_ms: 0,
            approval_id: Some("approval-1".to_string()),
            environment_id: None,
            reason: None,
            network_approval_context: Some(codex_app_server_protocol::NetworkApprovalContext {
                host: "example.com".to_string(),
                protocol: codex_app_server_protocol::NetworkApprovalProtocol::Https,
            }),
            command: Some("network-access https://example.com:8443".to_string()),
            cwd: None,
            command_actions: None,
            additional_permissions: None,
            proposed_execpolicy_amendment: None,
            proposed_network_policy_amendments: None,
            available_decisions: Some(vec![
                codex_app_server_protocol::CommandExecutionApprovalDecision::AcceptForSession,
                codex_app_server_protocol::CommandExecutionApprovalDecision::Cancel,
            ]),
        },
        &test_path_buf("/tmp").abs(),
    );

    handle_exec_approval_request(&mut chat, "sub-network", request);
    chat.handle_key_event(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE));

    let decision = drain_insert_history(&mut rx)
        .pop()
        .expect("expected decision cell in history");
    assert_snapshot!(
        "network_exec_approval_history_session_host_allowance",
        lines_to_single_string(&decision)
    );
}

#[tokio::test]
async fn network_exec_approval_history_describes_one_time_host_allowance() {
    // 单次主机放行与会话级放行使用不同摘要，避免误解授权期限。
    let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    let request = exec_approval_request_from_params(
        AppServerCommandExecutionRequestApprovalParams {
            thread_id: "thread-1".to_string(),
            turn_id: "turn-1".to_string(),
            item_id: "item-1".to_string(),
            started_at_ms: 0,
            approval_id: Some("approval-1".to_string()),
            environment_id: None,
            reason: None,
            network_approval_context: Some(codex_app_server_protocol::NetworkApprovalContext {
                host: "example.com".to_string(),
                protocol: codex_app_server_protocol::NetworkApprovalProtocol::Http,
            }),
            command: None,
            cwd: None,
            command_actions: None,
            additional_permissions: None,
            proposed_execpolicy_amendment: None,
            proposed_network_policy_amendments: None,
            available_decisions: Some(vec![
                codex_app_server_protocol::CommandExecutionApprovalDecision::Accept,
                codex_app_server_protocol::CommandExecutionApprovalDecision::Cancel,
            ]),
        },
        &test_path_buf("/tmp").abs(),
    );

    handle_exec_approval_request(&mut chat, "sub-network", request);
    chat.handle_key_event(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE));

    let decision = drain_insert_history(&mut rx)
        .pop()
        .expect("expected decision cell in history");
    assert_snapshot!(
        "network_exec_approval_history_one_time_host_allowance",
        lines_to_single_string(&decision)
    );
}

#[tokio::test]
async fn network_exec_approval_history_describes_canceled_host_request() {
    // 取消的网络授权仍留下结果记录，但不得表现为已经放行。
    let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    let request = exec_approval_request_from_params(
        AppServerCommandExecutionRequestApprovalParams {
            thread_id: "thread-1".to_string(),
            turn_id: "turn-1".to_string(),
            item_id: "item-1".to_string(),
            started_at_ms: 0,
            approval_id: Some("approval-1".to_string()),
            environment_id: None,
            reason: None,
            network_approval_context: Some(codex_app_server_protocol::NetworkApprovalContext {
                host: "example.com".to_string(),
                protocol: codex_app_server_protocol::NetworkApprovalProtocol::Socks5Tcp,
            }),
            command: Some("network-access socks5-tcp://example.com:1080".to_string()),
            cwd: None,
            command_actions: None,
            additional_permissions: None,
            proposed_execpolicy_amendment: None,
            proposed_network_policy_amendments: None,
            available_decisions: Some(vec![
                codex_app_server_protocol::CommandExecutionApprovalDecision::Accept,
                codex_app_server_protocol::CommandExecutionApprovalDecision::Cancel,
            ]),
        },
        &test_path_buf("/tmp").abs(),
    );

    handle_exec_approval_request(&mut chat, "sub-network", request);
    chat.handle_key_event(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE));

    let decision = drain_insert_history(&mut rx)
        .pop()
        .expect("expected decision cell in history");
    assert_snapshot!(
        "network_exec_approval_history_canceled_host_request",
        lines_to_single_string(&decision)
    );
}

#[test]
fn app_server_request_permissions_preserves_file_system_permissions() {
    // 文件系统权限请求保留读写根和受限路径，防止扩大实际能力。
    let read_path = AbsolutePathBuf::try_from(PathBuf::from(test_path_display("/tmp/read-only")))
        .expect("absolute read path");
    let write_path = AbsolutePathBuf::try_from(PathBuf::from(test_path_display("/tmp/write")))
        .expect("absolute write path");
    let read_api_path = LegacyAppPathString::from_abs_path(&read_path);
    let write_api_path = LegacyAppPathString::from_abs_path(&write_path);
    let cwd =
        AbsolutePathBuf::try_from(PathBuf::from(test_path_display("/tmp"))).expect("absolute cwd");

    let request = request_permissions_from_params(AppServerPermissionsRequestApprovalParams {
        thread_id: "thread-1".to_string(),
        turn_id: "turn-1".to_string(),
        item_id: "item-1".to_string(),
        environment_id: Some("remote".to_string()),
        started_at_ms: 0,
        cwd: cwd.clone(),
        reason: Some("Select a workspace root".to_string()),
        permissions: codex_app_server_protocol::RequestPermissionProfile {
            network: Some(AppServerAdditionalNetworkPermissions {
                enabled: Some(true),
            }),
            file_system: Some(AppServerAdditionalFileSystemPermissions {
                read: Some(vec![read_api_path]),
                write: Some(vec![write_api_path]),
                glob_scan_max_depth: None,
                entries: None,
            }),
        },
    })
    .expect("API paths should convert to native paths");

    assert_eq!(
        request.permissions,
        RequestPermissionProfile {
            network: Some(NetworkPermissions {
                enabled: Some(true),
            }),
            file_system: Some(FileSystemPermissions::from_read_write_roots(
                Some(vec![read_path]),
                Some(vec![write_path]),
            )),
        }
    );
    assert_eq!(request.cwd, Some(cwd));
    assert_eq!(request.environment_id.as_deref(), Some("remote"));
}

#[tokio::test]
async fn exec_approval_uses_approval_id_when_present() {
    let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;

    handle_exec_approval_request(
        &mut chat,
        "sub-short",
        ExecApprovalRequestEvent {
            call_id: "call-parent".into(),
            approval_id: Some("approval-subcommand".into()),
            turn_id: "turn-short".into(),
            environment_id: None,
            command: vec!["bash".into(), "-lc".into(), "echo hello world".into()],
            cwd: AbsolutePathBuf::current_dir().expect("current dir"),
            reason: Some(
                "this is a test reason such as one that would be produced by the model".into(),
            ),
            network_approval_context: None,
            proposed_execpolicy_amendment: None,
            proposed_network_policy_amendments: None,
            additional_permissions: None,
            available_decisions: None,
        },
    );

    chat.handle_key_event(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE));

    let mut found = false;
    while let Ok(app_ev) = rx.try_recv() {
        if let AppEvent::SubmitThreadOp {
            op: Op::ExecApproval { id, decision, .. },
            ..
        } = app_ev
        {
            assert_eq!(id, "approval-subcommand");
            assert_matches!(
                decision,
                codex_app_server_protocol::CommandExecutionApprovalDecision::Accept
            );
            found = true;
            break;
        }
    }
    assert!(found, "expected ExecApproval op to be sent");
}

#[tokio::test]
async fn exec_approval_decision_truncates_multiline_and_long_commands() {
    // 超长或多行命令只截断历史摘要，不改变发送给审批逻辑的原始命令。
    let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;

    let ev_multi = ExecApprovalRequestEvent {
        call_id: "call-multi".into(),
        approval_id: Some("call-multi".into()),
        turn_id: "turn-multi".into(),
        environment_id: None,
        command: vec!["bash".into(), "-lc".into(), "echo line1\necho line2".into()],
        cwd: AbsolutePathBuf::current_dir().expect("current dir"),
        reason: Some(
            "this is a test reason such as one that would be produced by the model".into(),
        ),
        network_approval_context: None,
        proposed_execpolicy_amendment: None,
        proposed_network_policy_amendments: None,
        additional_permissions: None,
        available_decisions: None,
    };
    handle_exec_approval_request(&mut chat, "sub-multi", ev_multi);
    let proposed_multi = drain_insert_history(&mut rx);
    assert!(
        proposed_multi.is_empty(),
        "expected multiline approval request to render via modal without emitting history cells"
    );

    let area = Rect::new(0, 0, 80, chat.desired_height(/*width*/ 80));
    let mut buf = ratatui::buffer::Buffer::empty(area);
    chat.render(area, &mut buf);
    let mut saw_first_line = false;
    for y in 0..area.height {
        let mut row = String::new();
        for x in 0..area.width {
            row.push(buf[(x, y)].symbol().chars().next().unwrap_or(' '));
        }
        if row.contains("echo line1") {
            saw_first_line = true;
            break;
        }
    }
    assert!(
        saw_first_line,
        "expected modal to show first line of multiline snippet"
    );

    chat.handle_key_event(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE));
    let aborted_multi = drain_insert_history(&mut rx)
        .pop()
        .expect("expected aborted decision cell (multiline)");
    assert_snapshot!(
        "exec_approval_history_decision_aborted_multiline",
        lines_to_single_string(&aborted_multi)
    );

    let long = format!("echo {}", "a".repeat(200));
    let ev_long = ExecApprovalRequestEvent {
        call_id: "call-long".into(),
        approval_id: Some("call-long".into()),
        turn_id: "turn-long".into(),
        environment_id: None,
        command: vec!["bash".into(), "-lc".into(), long],
        cwd: AbsolutePathBuf::current_dir().expect("current dir"),
        reason: None,
        network_approval_context: None,
        proposed_execpolicy_amendment: None,
        proposed_network_policy_amendments: None,
        additional_permissions: None,
        available_decisions: None,
    };
    handle_exec_approval_request(&mut chat, "sub-long", ev_long);
    let proposed_long = drain_insert_history(&mut rx);
    assert!(
        proposed_long.is_empty(),
        "expected long approval request to avoid emitting history cells before decision"
    );
    chat.handle_key_event(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE));
    let aborted_long = drain_insert_history(&mut rx)
        .pop()
        .expect("expected aborted decision cell (long)");
    assert_snapshot!(
        "exec_approval_history_decision_aborted_long",
        lines_to_single_string(&aborted_long)
    );
}
