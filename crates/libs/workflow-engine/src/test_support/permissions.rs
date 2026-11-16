use crate::{WorkflowToolProvider, WorkflowToolRequest, WorkflowAgentProvider, WorkflowAgentRequest, WorkflowAgentTool};
use std::sync::Arc;

/// Invoke the real native tool adapter supplied by the host.
pub async fn native_workflow_provider_names_preserve_tool_restrictions(tools: &dyn WorkflowToolProvider) {
    for provider in ["shiju", "vibewindow", "other"] {
        for (action, credential_id) in [(None, None), (Some("run"), None), (None, Some("secret"))] {
            let error = tools
                .call(WorkflowToolRequest {
                    provider: provider.into(),
                    tool_name: "bash".into(),
                    action: action.map(str::to_string),
                    credential_id: credential_id.map(str::to_string),
                    inputs: Default::default(),
                })
                .await
                .unwrap_err();
            if provider == "shiju" && action.is_none() && credential_id.is_none() {
                assert!(error.contains("不在当前会话能力范围内"), "{error}");
            } else {
                assert!(error.contains("仅支持 shiju"), "{error}");
            }
        }
    }
}

/// Invoke the real native agent adapter, including its empty-message validation.
pub async fn native_workflow_provider_names_preserve_agent_restrictions(native: &dyn WorkflowAgentProvider, tools: Arc<dyn WorkflowToolProvider>) {
    for provider in ["shiju", "vibewindow", "other"] {
        for (name, action, credential_id) in [
            ("bash", None, None),
            ("forbidden", None, None),
            ("bash", Some("run"), None),
            ("bash", None, Some("secret")),
        ] {
            let error = native
                .run(
                    WorkflowAgentRequest {
                        messages: vec![],
                        tools: vec![WorkflowAgentTool {
                            provider: provider.into(),
                            tool_name: name.into(),
                            action: action.map(str::to_string),
                            credential_id: credential_id.map(str::to_string),
                        }],
                        max_iterations: 1,
                        model: "unused".into(),
                        temperature: 0.0,
                        strategy: String::new(),
                    },
                    tools.clone(),
                )
                .await
                .unwrap_err();
            if provider != "shiju" || action.is_some() || credential_id.is_some() {
                assert!(error.contains("仅支持当前会话的 shiju 工具"), "{error}");
            } else if name == "forbidden" {
                assert!(error.contains("不在当前会话能力范围内"), "{error}");
            } else {
                assert_eq!(error, "工作流 agent 缺少消息");
            }
        }
    }
}
