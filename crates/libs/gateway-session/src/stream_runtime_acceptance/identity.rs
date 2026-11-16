use super::*;

pub fn profile_edit_stream<H: RuntimeAcceptanceHost>(runtime: &tokio::runtime::Runtime, workspace: &std::path::Path, session_id: &str) {
    for rule in ["ORIGINAL_BINDING_RULE", "EDITED_PROFILE_RULE"] {
        runtime.block_on(H::update_config(serde_json::json!({
            "agents":{"main":{
                "system_prompt":rule,
                "defaults":{
                    "model":{"mode":"fixed","value":{"provider":"model-recovery","model":"original"}},
                    "skills":{"mode":"fixed","value":[]},
                    "mcp":{"mode":"fixed","value":[]}
                }
            }}
        }))).unwrap();
        H::enqueue(vec![
            llm_runtime::StreamEvent::Delta("ready".into()),
            llm_runtime::StreamEvent::Done {
                usage: Default::default(),
                finish_reason: Some("stop".into()),
            },
        ]);
        let request = GatewayChatStreamRequest {
            session_id: Some(session_id.to_owned().into()),
            agent: Some("main".into()),
            messages: vec![serde_json::json!({"role":"user","content":"hello"})],
            options: Some(serde_json::json!({"full_access":false})),
            ..Default::default()
        };
        let directory = workspace.to_string_lossy().into_owned();
        let tenant = session_store::ui_store::current_tenant_scope();
        runtime.block_on(async {
            let mut receiver = H::with_tenant_instance(tenant.clone(), directory.clone(), move || {
                Box::pin(async move {
                    crate::stream::start_gateway_chat_stream::<H>(tenant, directory, request, None, None).await
                })
            })
            .await
            .unwrap();
            loop {
                let payload =
                    tokio::time::timeout(std::time::Duration::from_secs(20), receiver.recv())
                        .await
                        .unwrap()
                        .expect("terminal gateway event");
                assert_ne!(payload["type"], "chat.error", "{payload}");
                if payload["type"] == "chat.done" {
                    break;
                }
            }
        });
        let captured = H::captured(&session_id);
        assert_eq!(captured.len(), 1);
        assert!(
            captured[0].system.iter().any(|body| body.contains("ORIGINAL_BINDING_RULE")),
            "resuming the same session must retain its original rules after profile edits"
        );
        assert!(!captured[0].system.iter().any(|body| body.contains("EDITED_PROFILE_RULE")));
    }
}
