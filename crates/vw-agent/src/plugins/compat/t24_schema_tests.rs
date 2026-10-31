use super::*;

#[tokio::test]
async fn codex_plugin_compat_t24_rejects_unsupported_raw_schemas_without_weakening_them() {
    let root = tempfile::tempdir().unwrap();
    let host = McpFormSession::register("t24-raw", root.path()).unwrap();
    let mut events = host.subscribe();
    let pool = McpSessionConnections::default();
    let server = server(root.path(), "forms");
    let context = Arc::new(ToolUseContext::new("t24-raw", Some(root.path().display().to_string())));
    scope_tool_use_context(context, async {
        let schemas = [
            json!({"type":"object","properties":{"value":{"type":"string","pattern":"remote-secret-sentinel"}}}),
            json!({"type":"object","properties":{"value":{"type":"object","properties":{}}}}),
            json!({"type":"object","properties":{"value":{"type":"string","format":"password"}}}),
            json!({"type":"object","properties":{"value":{"type":"string","enum":["x","x"]}}}),
            json!({"type":"object","properties":{"value":{"type":"string","enum":["x"],"enumNames":[]}}}),
            json!({"type":"object","properties":{"value":{"type":"number","minimum":4,"maximum":2}}}),
            json!({"type":"object","properties":{"value":{"type":"boolean","minimum":1}}}),
            json!({"type":"object","properties":{},"required":["missing"]}),
            json!({"type":"object","properties":{},"$ref":"https://example.invalid/schema"}),
            json!({"type":"object","properties":{},"additionalProperties":true}),
            json!({"type":"object","properties":{},"description":"x".repeat(70_000)}),
        ];
        for schema in schemas {
            let result = call_args(&pool, &server, Some(json!({"schema":schema}).as_object().unwrap().clone()), Duration::from_secs(5)).await.unwrap();
            assert_eq!(result["form_reply"]["error"]["code"],-32602);
            assert!(!result.to_string().contains("remote-secret-sentinel"));
            assert!(host.pending().is_empty());
            assert!(events.try_recv().is_err(), "invalid schemas must not be presented to users");
        }
        let result = call_args(&pool, &server, Some(json!({"params":{"mode":"url","message":"open","url":"https://example.invalid/authorize"}}).as_object().unwrap().clone()),Duration::from_secs(5)).await.unwrap();
        assert_eq!(result["form_reply"]["error"]["code"],-32602,"URL/auth interaction is not a form approval");
        assert!(host.pending().is_empty());
    }).await;
    pool.shutdown();
}

#[tokio::test]
async fn codex_plugin_compat_t24_validates_enums_formats_numbers_and_user_supplied_defaults() {
    let root = tempfile::tempdir().unwrap();
    let host = McpFormSession::register("t24-schema", root.path()).unwrap();
    let mut events = host.subscribe();
    let pool = McpSessionConnections::default();
    let server = server(root.path(), "forms");
    let context =
        Arc::new(ToolUseContext::new("t24-schema", Some(root.path().display().to_string())));
    let schema = json!({"type":"object","properties":{
        "choice":{"type":"string","enum":["one","two"],"enumNames":["第一项","第二项"],"default":"one"},
        "name":{"type":"string","minLength":2,"maxLength":4},
        "approved":{"type":"boolean","default":false},
        "age":{"type":"integer","minimum":18,"maximum":120},
        "value":{"type":"number","minimum":0.5,"maximum":2.5},
        "large":{"type":"integer","minimum":18446744073709551614_u64,"maximum":18446744073709551615_u64},
        "email":{"type":"string","format":"email"},
        "uri":{"type":"string","format":"uri"},
        "date":{"type":"string","format":"date"},
        "time":{"type":"string","format":"date-time"}
    },"required":["choice","name","approved","age","value","large","email","uri","date","time"]});
    let content = json!({"choice":"two","name":"用户","approved":true,"age":18.0,"value":1.5,"large":18446744073709551615_u64,
        "email":"user@example.com","uri":"https://example.com/resource","date":"2026-10-31","time":"2026-10-31T08:00:00+08:00"});
    scope_tool_use_context(context, async {
        let operation = call_args(
            &pool,
            &server,
            Some(json!({"schema":schema}).as_object().unwrap().clone()),
            Duration::from_secs(5),
        );
        tokio::pin!(operation);
        let req = request(&mut events, &mut operation.as_mut()).await;
        assert_eq!(req.requested_schema["properties"]["choice"]["enum"], json!(["one", "two"]));
        for (key, value) in [
            ("choice", json!("outside")),
            ("name", json!("一")),
            ("approved", json!("true")),
            ("age", json!(18.5)),
            ("value", json!(0.4)),
            ("large", json!(18446744073709551613_u64)),
            ("email", json!("not-an-email")),
            ("uri", json!("relative/path")),
            ("date", json!("2026-02-29")),
            ("date", json!("2026-1-2")),
            ("time", json!("2026-10-31")),
        ] {
            let mut bad = content.clone();
            bad[key] = value;
            assert!(
                host.respond(
                    &req,
                    McpFormResponse { action: McpFormAction::Accept, content: Some(bad) }
                )
                .is_err()
            );
            assert_eq!(host.pending().len(), 1);
        }
        let mut missing = content.clone();
        missing.as_object_mut().unwrap().remove("approved");
        assert!(
            host.respond(
                &req,
                McpFormResponse { action: McpFormAction::Accept, content: Some(missing) }
            )
            .is_err(),
            "a default is not a user's answer"
        );
        let mut extra = content.clone();
        extra["undeclared"] = json!("value");
        assert!(
            host.respond(
                &req,
                McpFormResponse { action: McpFormAction::Accept, content: Some(extra) }
            )
            .is_err()
        );
        host.respond(
            &req,
            McpFormResponse { action: McpFormAction::Accept, content: Some(content.clone()) },
        )
        .unwrap();
        assert_eq!(operation.await.unwrap()["form_reply"]["result"]["content"], content);
    })
    .await;
    pool.shutdown();
}

#[tokio::test]
async fn codex_plugin_compat_t24_requires_explicit_accept_even_for_empty_forms() {
    let root = tempfile::tempdir().unwrap();
    let host = McpFormSession::register("t24-empty", root.path()).unwrap();
    let mut events = host.subscribe();
    let pool = McpSessionConnections::default();
    let server = server(root.path(), "forms");
    let context =
        Arc::new(ToolUseContext::new("t24-empty", Some(root.path().display().to_string())));
    scope_tool_use_context(context, async {
        let operation = call_args(
            &pool,
            &server,
            Some(json!({"schema":{"type":"object","properties":{}}}).as_object().unwrap().clone()),
            Duration::from_secs(5),
        );
        tokio::pin!(operation);
        let req = request(&mut events, &mut operation.as_mut()).await;
        assert_eq!(host.pending().len(), 1);
        for response in [
            McpFormResponse { action: McpFormAction::Accept, content: None },
            McpFormResponse { action: McpFormAction::Cancel, content: Some(json!({})) },
        ] {
            assert!(host.respond(&req, response).is_err());
        }
        host.respond(
            &req,
            McpFormResponse { action: McpFormAction::Accept, content: Some(json!({})) },
        )
        .unwrap();
        assert_eq!(
            operation.await.unwrap()["form_reply"]["result"],
            json!({"action":"accept","content":{}})
        );
    })
    .await;
    pool.shutdown();
}

#[tokio::test]
async fn codex_plugin_compat_t24_binds_tenant_workspace_and_remote_metadata_authority() {
    use crate::session::ui_store::with_tenant_scope;
    use vw_api_types::id::TenantScope;
    let root = tempfile::tempdir().unwrap();
    let elsewhere = tempfile::tempdir().unwrap();
    let tenant = TenantScope::new("t24-platform", "first").unwrap();
    let other = TenantScope::new("t24-platform", "second").unwrap();
    with_tenant_scope(tenant.clone(),async {
        let host=McpFormSession::register("shared-name",root.path()).unwrap();
        let mut events=host.subscribe();
        let pool=McpSessionConnections::default();
        let server=server(root.path(),"actual-server");
        let context=Arc::new(ToolUseContext::new("shared-name",Some(root.path().display().to_string())));
        scope_tool_use_context(context,async {
            let args=json!({"params":{"message":"confirm","requestedSchema":{"type":"object","properties":{}},"_meta":{"server_name":"spoofed","session_id":"other"}}}).as_object().unwrap().clone();
            let operation=call_args(&pool,&server,Some(args),Duration::from_secs(5));
            tokio::pin!(operation);
            let req=request(&mut events,&mut operation.as_mut()).await;
            assert_eq!(req.server_name,"actual-server");assert_eq!(req.session_id,"shared-name");
            assert_eq!(req.meta.as_ref().unwrap()["server_name"],"spoofed");
            with_tenant_scope(other,async {
                assert!(host.pending().is_empty());
                assert!(host.respond(&req,McpFormResponse {action:McpFormAction::Accept,content:Some(json!({}))}).is_err());
                let another=McpFormSession::register("shared-name",root.path()).unwrap();
                assert!(another.respond(&req,answer()).is_err());
            }).await;
            let mut connection=req.clone();connection.connection_id.push_str("-different");
            let mut request_id=req.clone();request_id.server_request_id=json!("different");
            let mut schema=req.clone();schema.requested_schema=json!({"type":"object","properties":{"forged":{"type":"boolean"}}});
            for forged in [connection,request_id,schema] {
                assert!(host.respond(&forged,answer()).is_err());
            }
            host.respond(&req,McpFormResponse{action:McpFormAction::Decline,content:None}).unwrap();
            assert_eq!(operation.await.unwrap()["form_reply"]["result"]["action"],"decline");
        }).await;
        let context=Arc::new(ToolUseContext::new("shared-name",Some(elsewhere.path().display().to_string())));
        let result=scope_tool_use_context(context,call(&pool,&server)).await.unwrap();
        assert_eq!(result["form_reply"]["error"]["code"],-32600,"another workspace cannot borrow the UI handler");
        pool.shutdown();
    }).await;
}

#[tokio::test]
async fn codex_plugin_compat_t24_rejects_null_constraints_instead_of_ignoring_them() {
    let root = tempfile::tempdir().unwrap();
    let host = McpFormSession::register("t24-null", root.path()).unwrap();
    let mut events = host.subscribe();
    let pool = McpSessionConnections::default();
    let server = server(root.path(), "forms");
    let context =
        Arc::new(ToolUseContext::new("t24-null", Some(root.path().display().to_string())));
    scope_tool_use_context(context,async {
        for field in [json!({"type":"integer","minimum":null}),json!({"type":"string","enum":null}),json!({"type":"boolean","default":null})] {
            let operation=call_args(&pool,&server,Some(json!({"schema":{"type":"object","properties":{"value":field}}}).as_object().unwrap().clone()),Duration::from_secs(5));
            tokio::pin!(operation);
            tokio::select! {
                event=events.recv()=>{
                    let McpFormEvent::Requested{request}=event.unwrap() else{panic!("expected request")};
                    host.respond(&request,McpFormResponse{action:McpFormAction::Decline,content:None}).unwrap();
                    operation.await.unwrap();
                    panic!("target: invalid raw null constraints must not be silently ignored");
                },
                result=&mut operation=>assert_eq!(result.unwrap()["form_reply"]["error"]["code"],-32602),
            }
        }
    }).await;
    pool.shutdown();
}
