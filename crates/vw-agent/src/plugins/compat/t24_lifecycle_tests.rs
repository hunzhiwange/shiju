use super::*;

async fn finished(host: &McpFormSession) {
    tokio::time::timeout(Duration::from_secs(2), async {
        while !host.pending().is_empty() {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn codex_plugin_compat_t24_cancels_and_withdraws_handler_without_model_answers() {
    let root = tempfile::tempdir().unwrap();
    let pool = McpSessionConnections::default();
    let server = server(root.path(), "forms");
    let context =
        Arc::new(ToolUseContext::new("t24-capability", Some(root.path().display().to_string())));
    scope_tool_use_context(context, async {
        let unsupported = call(&pool, &server).await.unwrap();
        assert_eq!(unsupported["form_reply"]["error"]["code"], -32600);
        assert!(unsupported["initialize"]["capabilities"].get("elicitation").is_none());
        let host = McpFormSession::register("t24-capability", root.path()).unwrap();
        let mut events = host.subscribe();
        let operation = call(&pool, &server);
        tokio::pin!(operation);
        let req = request(&mut events, &mut operation.as_mut()).await;
        assert!(
            crate::tools::mcp_common::unsupported_mcp_capabilities_json()
                .as_array()
                .unwrap()
                .iter()
                .all(|c| c["capability"] != "elicitation")
        );
        host.respond(&req, McpFormResponse { action: McpFormAction::Cancel, content: None })
            .unwrap();
        let accepted = operation.await.unwrap();
        assert_eq!(accepted["form_reply"]["result"], json!({"action":"cancel"}));
        assert_ne!(
            accepted["pid"], unsupported["pid"],
            "installing a real host handler renegotiates its connection"
        );
        drop(host);
        let withdrawn = call(&pool, &server).await.unwrap();
        assert_eq!(withdrawn["form_reply"]["error"]["code"], -32600);
        assert!(withdrawn["initialize"]["capabilities"].get("elicitation").is_none());
        assert_ne!(withdrawn["pid"], accepted["pid"]);
    })
    .await;
    pool.shutdown();
    assert_eq!(
        std::fs::read(root.path().join("server.py")).unwrap(),
        include_bytes!("../../../testdata/plugin_compat/t24/server.py")
    );
    assert_eq!(
        std::fs::read_dir(root.path()).unwrap().count(),
        1,
        "forms never persist user data in the server package"
    );
}

#[tokio::test]
async fn codex_plugin_compat_t24_releases_requests_on_remote_cancel_disconnect_and_call_timeout() {
    for mode in ["cancel", "disconnect", "timeout", "complete_early"] {
        let root = tempfile::tempdir().unwrap();
        let host = McpFormSession::register(mode, root.path()).unwrap();
        let mut events = host.subscribe();
        let pool = McpSessionConnections::default();
        let server = server(root.path(), "forms");
        let context = Arc::new(ToolUseContext::new(mode, Some(root.path().display().to_string())));
        scope_tool_use_context(context, async {
            let args = json!({mode:true}).as_object().unwrap().clone();
            let operation = call_args(
                &pool,
                &server,
                Some(args),
                if mode == "timeout" { Duration::from_millis(100) } else { Duration::from_secs(5) },
            );
            tokio::pin!(operation);
            let req = request(&mut events, &mut operation.as_mut()).await;
            let result = operation.await;
            if mode == "cancel" {
                assert_eq!(result.unwrap()["form_reply"]["result"], json!({"action":"cancel"}));
            } else if mode == "complete_early" {
                assert_eq!(result.unwrap()["early"], true);
            } else {
                assert!(result.is_err(), "closed/timed-out operations must fail");
            }
            finished(&host).await;
            assert!(
                host.respond(&req, answer()).is_err(),
                "late user data cannot answer a cancelled/finished operation"
            );
        })
        .await;
        pool.shutdown();
    }
}

#[tokio::test]
async fn codex_plugin_compat_t24_dropping_ui_lease_cancels_waiting_server() {
    let root = tempfile::tempdir().unwrap();
    let host = McpFormSession::register("t24-ui-drop", root.path()).unwrap();
    let mut events = host.subscribe();
    let pool = McpSessionConnections::default();
    let server = server(root.path(), "forms");
    let context =
        Arc::new(ToolUseContext::new("t24-ui-drop", Some(root.path().display().to_string())));
    scope_tool_use_context(context, async {
        let operation = call(&pool, &server);
        tokio::pin!(operation);
        let req = request(&mut events, &mut operation.as_mut()).await;
        drop(host);
        assert_eq!(operation.await.unwrap()["form_reply"]["result"], json!({"action":"cancel"}));
        let event =
            tokio::time::timeout(Duration::from_secs(1), events.recv()).await.unwrap().unwrap();
        assert_eq!(event, McpFormEvent::Finished { request_id: req.id });
    })
    .await;
    pool.shutdown();
}

#[tokio::test]
async fn codex_plugin_compat_t24_routes_equal_wire_ids_from_concurrent_servers() {
    let root = tempfile::tempdir().unwrap();
    let host = McpFormSession::register("t24-concurrent", root.path()).unwrap();
    let mut events = host.subscribe();
    let pool = Arc::new(McpSessionConnections::default());
    let servers = [server(root.path(), "first"), server(root.path(), "second")];
    let mut tasks = Vec::new();
    for server in servers {
        let pool = pool.clone();
        let context = Arc::new(ToolUseContext::new(
            "t24-concurrent",
            Some(root.path().display().to_string()),
        ));
        tasks.push(tokio::spawn(scope_tool_use_context(context, async move {
            call(&pool, &server).await
        })));
    }
    let mut requests = Vec::new();
    for _ in 0..2 {
        let event =
            tokio::time::timeout(Duration::from_secs(3), events.recv()).await.unwrap().unwrap();
        if let McpFormEvent::Requested { request } = event {
            requests.push(request);
        } else {
            panic!("expected request");
        }
    }
    assert_ne!(requests[0].connection_id, requests[1].connection_id);
    assert_eq!(requests[0].server_request_id, requests[1].server_request_id);
    assert_eq!(host.pending().len(), 2);
    for req in requests {
        let mut response = answer();
        response.content.as_mut().unwrap()["name"] = json!(req.server_name);
        host.respond(&req, response).unwrap();
    }
    let first = tasks.remove(0).await.unwrap().unwrap();
    let second = tasks.remove(0).await.unwrap().unwrap();
    assert_eq!(first["form_reply"]["result"]["content"]["name"], "first");
    assert_eq!(second["form_reply"]["result"]["content"]["name"], "second");
    finished(&host).await;
    pool.shutdown();
}

#[tokio::test(start_paused = true)]
async fn codex_plugin_compat_t24_form_deadline_expires_without_consuming_late_input() {
    // Drive the actual broker's clock directly; local-process tests cover transport cancellation.
    let root = tempfile::tempdir().unwrap();
    let host = McpFormSession::register("t24-deadline", root.path()).unwrap();
    let mut events = host.subscribe();
    let context =
        Arc::new(ToolUseContext::new("t24-deadline", Some(root.path().display().to_string())));
    scope_tool_use_context(context, async {
        let binding = crate::plugins::compat::mcp_elicitation::current_host().unwrap().bind("forms", tokio_util::sync::CancellationToken::new());
        let _operation = binding.activate();
        let wire = crate::plugins::RawMcpResponse::default();
        let operation = binding.request(json!({"message":"confirm","requestedSchema":{"type":"object","properties":{}}}), rmcp::model::RequestId::Number(1),tokio_util::sync::CancellationToken::new(), tokio_util::sync::CancellationToken::new(), &wire);
        tokio::pin!(operation);
        let req = tokio::select! {event=events.recv()=>match event.unwrap(){McpFormEvent::Requested{request}=>request,_=>panic!("expected request")}, result=&mut operation=>panic!("must await user: {result:?}")};
        tokio::time::advance(Duration::from_secs(121)).await;
        assert!(host.respond(&req, McpFormResponse {action:McpFormAction::Accept,content:Some(json!({}))}).is_err());
        assert_eq!(serde_json::to_value(operation.await.unwrap()).unwrap(), json!({"action":"cancel"}));
        assert!(host.pending().is_empty());
    }).await;
}

#[tokio::test]
async fn codex_plugin_compat_t24_roundtrips_user_form_over_streamable_http() {
    use agent_client_protocol::McpServerHttp;
    let root = tempfile::tempdir().unwrap();
    let script = root.path().join("http_server.py");
    let port = root.path().join("port");
    std::fs::write(&script, include_bytes!("../../../testdata/plugin_compat/t24/http_server.py"))
        .unwrap();
    let mut child = tokio::process::Command::new("python3")
        .arg(script)
        .arg(&port)
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    let port = tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            if let Ok(value) = std::fs::read_to_string(&port) {
                break value;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    let host = McpFormSession::register("t24-http", root.path()).unwrap();
    let mut events = host.subscribe();
    let pool = McpSessionConnections::default();
    let server =
        McpServer::Http(McpServerHttp::new("http-forms", format!("http://127.0.0.1:{port}/mcp")));
    let context =
        Arc::new(ToolUseContext::new("t24-http", Some(root.path().display().to_string())));
    scope_tool_use_context(context, async {
        let operation = call(&pool, &server);
        tokio::pin!(operation);
        let req = request(&mut events, &mut operation.as_mut()).await;
        assert_eq!(req.server_request_id, "http-form");
        host.respond(
            &req,
            McpFormResponse {
                action: McpFormAction::Accept,
                content: Some(json!({"confirmed":true})),
            },
        )
        .unwrap();
        assert_eq!(
            operation.await.unwrap()["form_reply"]["result"],
            json!({"action":"accept","content":{"confirmed":true}})
        );
        assert!(host.pending().is_empty());
    })
    .await;
    pool.shutdown();
    child.kill().await.unwrap();
    child.wait().await.unwrap();
}
