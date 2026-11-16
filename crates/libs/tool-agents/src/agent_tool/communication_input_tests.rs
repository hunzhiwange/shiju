//! Communication input validation stops before any host service is invoked.

use super::{AgentTool, input_test_support::InputHost};
use serde_json::json;
use std::collections::HashMap;
use tool_types::Tool;

#[tokio::test]
async fn agcollab_tool_rejects_invalid_action_fields() {
    let tool = AgentTool::<InputHost>::new(HashMap::new(), Some("root".into()), ".".into());
    for args in [
        json!({"action":"list_agents","unknown":true}),
        json!({"action":"send_message","target":"","message":"hello"}),
        json!({"action":"followup_task","target":"child"}),
        json!({"action":"status","message":"ignored"}),
        json!({"action":12}),
    ] {
        assert!(tool.execute(args).await.is_err());
    }
}
