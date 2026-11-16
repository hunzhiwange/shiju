use super::*;

pub fn video_template_ledger(rows: &[String], a: &std::path::Path, b: &std::path::Path, initial_digest: &str, edited_digest: &str) {
    let runs = rows
        .iter()
        .map(|r| serde_json::from_str::<serde_json::Value>(r).unwrap())
        .collect::<Vec<_>>();
    assert!(
        runs.iter()
            .any(|r| r["source"]["path"] == a.canonicalize().unwrap().to_string_lossy().as_ref()
                && r["template_sha256"] == initial_digest)
    );
    assert!(
        runs.iter()
            .any(|r| r["source"]["path"] == b.canonicalize().unwrap().to_string_lossy().as_ref()
                && r["template_sha256"] == edited_digest)
    );
}

pub fn task_agent_defaults(agent: &str, resolved: &DelegateRequestOverrides, request: &GatewayChatStreamRequest, binding: &session_engine::agent_binding::AgentBinding) {
    assert_eq!(resolved.agent.as_deref(), Some(agent));
    assert_eq!(binding.agent, agent);
    if agent != "acp-codex" {
        if agent == "task-reviewer" {
            assert_eq!(resolved.model.as_deref(), Some("model-recovery/original"));
        }
        assert_eq!(binding.allowed_tools, vec!["read"]);
        assert_eq!(request.options.as_ref().unwrap()["full_access"], false);
    } else {
        assert_eq!(request.acp_agent.as_deref(), Some("codex"));
    }
}

pub async fn resumed_profile_selection<H: crate::stream_host::StreamHost>(session_id: &str) {
                let mut resumed = GatewayChatStreamRequest {
                    session_id: Some(session_id.to_owned().into()),
                    agent: Some("main".into()),
                    ..Default::default()
                };
                let overrides = crate::stream::prepare_gateway_agent_request::<H>(&mut resumed, None).await.unwrap();
                assert_eq!(overrides.chat_system_prompt.as_deref(), Some("ORIGINAL_BINDING_RULE"));
                let mut switched = resumed.clone();
                switched.model = Some("model-recovery/different".into());
                let error = crate::stream::prepare_gateway_agent_request::<H>(&mut switched, None).await.err().unwrap();
                assert!(error.message.contains("changing a session model requires a new session"));
}
