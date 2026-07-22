//! Agent IPC 消息发送测试。
//!
//! 这些测试使用临时 IPC 数据库验证工具契约、单播/广播/团队路由、权限拒绝，以及
//! 无效载荷和目标的输入校验。

use super::super::agents_ipc::{AgentsInboxTool, IpcDb};
use super::super::*;
use super::*;
use crate::app::agent::config::AgentsIpcConfig;
use crate::app::agent::security::{AutonomyLevel, SecurityPolicy};
use serde_json::{Value, json};
use std::sync::Arc;
use tempfile::TempDir;

fn test_ipc_db(root: &TempDir, workspace_name: &str) -> Arc<IpcDb> {
    let workspace = root.path().join(workspace_name);
    std::fs::create_dir_all(&workspace).expect("workspace should be created");
    // 多个 agent 共享同一个临时数据库，但使用不同 workspace 生成不同 agent_id。
    let config = AgentsIpcConfig {
        enabled: true,
        db_path: root.path().join("agents.db").to_string_lossy().to_string(),
        staleness_secs: 300,
    };
    Arc::new(IpcDb::open(&workspace, &config).expect("ipc db should open"))
}

#[test]
fn send_message_exposes_stable_schema_and_spec() {
    let root = TempDir::new().expect("temp dir should be created");
    let tool =
        SendMessageTool::new(test_ipc_db(&root, "agent-a"), Arc::new(SecurityPolicy::default()));

    assert_eq!(tool.name(), "SendMessage");
    assert_eq!(tool.description(), "向单个 agent、团队成员或全部在线 agent 发送 IPC 消息。");

    let schema = tool.parameters_schema();
    assert_eq!(schema["type"], "object");
    assert_eq!(schema["additionalProperties"], false);
    assert_eq!(schema["properties"]["to_agent"]["type"], "string");
    assert_eq!(schema["properties"]["team_id"]["type"], "string");
    assert_eq!(schema["properties"]["payload"]["type"], "string");
    assert_eq!(schema["required"], json!(["payload"]));

    let spec = tool.spec();
    assert_eq!(spec.id, "SendMessage");
    assert_eq!(spec.display_name, "SendMessage");
    assert_eq!(spec.name, "SendMessage");
    assert_eq!(spec.description, tool.description());
    assert_eq!(spec.input_schema, schema);
    assert_eq!(spec.parameters, schema);
    assert_eq!(spec.aliases, vec!["send_message"]);
    assert!(!spec.read_only);
    assert!(!spec.destructive);
    assert!(!spec.concurrency_safe);
    assert!(!spec.requires_user_interaction);
    assert!(spec.strict);
}

#[tokio::test]
async fn direct_send_trims_route_and_payload_and_reports_target() {
    let root = TempDir::new().expect("temp dir should be created");
    let db_a = test_ipc_db(&root, "agent-a");
    let db_b = test_ipc_db(&root, "agent-b");
    let sender_id = db_a.agent_id().to_string();
    let target_id = db_b.agent_id().to_string();
    let send_tool = SendMessageTool::new(db_a, Arc::new(SecurityPolicy::default()));

    let sent = send_tool
        .execute(json!({
            "to_agent": format!("  {target_id}  "),
            "payload": "  hello agent  "
        }))
        .await
        .expect("direct send should return result");

    assert!(sent.success);
    assert!(sent.error.is_none());
    let sent_json: Value = serde_json::from_str(&sent.output).expect("send output should be json");
    assert_eq!(sent_json["targets"], json!([target_id]));
    assert_eq!(sent_json["count"], 1);

    let inbox =
        AgentsInboxTool::new(db_b).execute(json!({})).await.expect("inbox should be readable");
    let messages: Vec<Value> =
        serde_json::from_str(&inbox.output).expect("inbox output should be json");
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0]["from_agent"], sender_id);
    assert_eq!(messages[0]["payload"], "hello agent");
}

#[tokio::test]
async fn broadcast_send_reaches_each_agent() {
    let root = TempDir::new().expect("temp dir should be created");
    let db_a = test_ipc_db(&root, "agent-a");
    let db_b = test_ipc_db(&root, "agent-b");
    let db_c = test_ipc_db(&root, "agent-c");
    let send_tool = SendMessageTool::new(db_a, Arc::new(SecurityPolicy::default()));

    let sent = send_tool
        .execute(json!({
            "to_agent": "*",
            "payload": "announcement"
        }))
        .await
        .expect("broadcast send should return result");

    assert!(sent.success);
    let sent_json: Value = serde_json::from_str(&sent.output).expect("send output should be json");
    assert_eq!(sent_json["targets"], json!(["*"]));
    assert_eq!(sent_json["count"], 1);

    for recipient in [db_b, db_c] {
        let inbox = AgentsInboxTool::new(recipient)
            .execute(json!({}))
            .await
            .expect("inbox should be readable");
        let messages: Vec<Value> =
            serde_json::from_str(&inbox.output).expect("inbox output should be json");
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0]["payload"], "announcement");
    }
}

#[tokio::test]
async fn team_send_routes_messages_to_all_members() {
    let root = TempDir::new().expect("temp dir should be created");
    let db_a = test_ipc_db(&root, "agent-a");
    let db_b = test_ipc_db(&root, "agent-b");
    let db_c = test_ipc_db(&root, "agent-c");
    let security = Arc::new(SecurityPolicy::default());

    let team_create = TeamCreateTool::new(db_a.clone(), security.clone());
    let send_tool = SendMessageTool::new(db_a.clone(), security);

    let created = team_create
        .execute(json!({
            "id": "reviewers",
            "members": [
                db_b.agent_id(),
                " ",
                db_c.agent_id(),
                db_b.agent_id()
            ]
        }))
        .await
        .expect("team create should return result");
    assert!(created.success);
    let created_json: Value =
        serde_json::from_str(&created.output).expect("team create output should be json");
    let members = created_json["members"].as_array().expect("members should be returned as array");
    assert_eq!(members.len(), 2);
    assert!(members.iter().any(|member| member == db_b.agent_id()));
    assert!(members.iter().any(|member| member == db_c.agent_id()));

    let sent = send_tool
        .execute(json!({
            "team_id": "reviewers",
            "payload": "hello reviewers"
        }))
        .await
        .expect("team send should return result");
    assert!(sent.success);
    let sent_json: Value = serde_json::from_str(&sent.output).expect("send output should be json");
    assert_eq!(sent_json["count"], 2);

    let inbox_b = AgentsInboxTool::new(db_b);
    let inbox_b_result = inbox_b.execute(json!({})).await.expect("inbox should be readable");
    let inbox_b_messages: Vec<Value> =
        serde_json::from_str(&inbox_b_result.output).expect("inbox output should be json");
    assert_eq!(inbox_b_messages.len(), 1);
    assert_eq!(inbox_b_messages[0]["payload"], "hello reviewers");

    let inbox_c = AgentsInboxTool::new(db_c);
    let inbox_c_result = inbox_c.execute(json!({})).await.expect("inbox should be readable");
    let inbox_c_messages: Vec<Value> =
        serde_json::from_str(&inbox_c_result.output).expect("inbox output should be json");
    assert_eq!(inbox_c_messages.len(), 1);
    assert_eq!(inbox_c_messages[0]["payload"], "hello reviewers");
}

#[tokio::test]
async fn team_route_takes_precedence_over_direct_target() {
    let root = TempDir::new().expect("temp dir should be created");
    let db_a = test_ipc_db(&root, "agent-a");
    let db_b = test_ipc_db(&root, "agent-b");
    let db_c = test_ipc_db(&root, "agent-c");
    let security = Arc::new(SecurityPolicy::default());
    let team_create = TeamCreateTool::new(db_a.clone(), security.clone());
    let send_tool = SendMessageTool::new(db_a, security);

    let created = team_create
        .execute(json!({
            "id": "reviewers",
            "members": [db_b.agent_id()]
        }))
        .await
        .expect("team create should return result");
    assert!(created.success);

    let sent = send_tool
        .execute(json!({
            "team_id": "reviewers",
            "to_agent": db_c.agent_id(),
            "payload": "team only"
        }))
        .await
        .expect("team send should return result");
    assert!(sent.success);
    let sent_json: Value = serde_json::from_str(&sent.output).expect("send output should be json");
    assert_eq!(sent_json["targets"], json!([db_b.agent_id()]));

    let inbox_b = AgentsInboxTool::new(db_b)
        .execute(json!({}))
        .await
        .expect("team member inbox should be readable");
    let messages_b: Vec<Value> =
        serde_json::from_str(&inbox_b.output).expect("inbox output should be json");
    assert_eq!(messages_b.len(), 1);
    assert_eq!(messages_b[0]["payload"], "team only");

    let inbox_c = AgentsInboxTool::new(db_c)
        .execute(json!({}))
        .await
        .expect("direct target inbox should be readable");
    let messages_c: Vec<Value> =
        serde_json::from_str(&inbox_c.output).expect("inbox output should be json");
    assert!(messages_c.is_empty());
}

#[tokio::test]
async fn team_delete_removes_message_route() {
    let root = TempDir::new().expect("temp dir should be created");
    let db_a = test_ipc_db(&root, "agent-a");
    let db_b = test_ipc_db(&root, "agent-b");
    let security = Arc::new(SecurityPolicy::default());

    let team_create = TeamCreateTool::new(db_a.clone(), security.clone());
    let team_delete = TeamDeleteTool::new(db_a.clone(), security.clone());
    let send_tool = SendMessageTool::new(db_a, security);

    let created = team_create
        .execute(json!({
            "id": "reviewers",
            "members": [db_b.agent_id()]
        }))
        .await
        .expect("team create should return result");
    assert!(created.success);

    let deleted = team_delete
        .execute(json!({ "id": "reviewers" }))
        .await
        .expect("team delete should return result");
    assert!(deleted.success);
    let deleted_json: Value =
        serde_json::from_str(&deleted.output).expect("team delete output should be json");
    assert_eq!(deleted_json["deleted"], true);

    let error = send_tool
        .execute(json!({
            "team_id": "reviewers",
            "payload": "should fail"
        }))
        .await
        .expect_err("deleted team should not resolve");
    assert!(error.to_string().contains("unknown team"));
}

#[tokio::test]
async fn readonly_policy_rejects_send_without_delivery() {
    let root = TempDir::new().expect("temp dir should be created");
    let db_a = test_ipc_db(&root, "agent-a");
    let db_b = test_ipc_db(&root, "agent-b");
    let readonly =
        Arc::new(SecurityPolicy { autonomy: AutonomyLevel::ReadOnly, ..SecurityPolicy::default() });
    let send_tool = SendMessageTool::new(db_a, readonly);

    let rejected = send_tool
        .execute(json!({
            "to_agent": db_b.agent_id(),
            "payload": "must not be delivered"
        }))
        .await
        .expect("policy rejection should return a tool result");

    assert!(!rejected.success);
    assert!(rejected.output.is_empty());
    assert_eq!(
        rejected.error.as_deref(),
        Some("Security policy: read-only mode, cannot perform 'agents_send'")
    );

    let inbox =
        AgentsInboxTool::new(db_b).execute(json!({})).await.expect("inbox should be readable");
    let messages: Vec<Value> =
        serde_json::from_str(&inbox.output).expect("inbox output should be json");
    assert!(messages.is_empty());
}

#[tokio::test]
async fn send_message_rejects_missing_payload_before_routing() {
    let root = TempDir::new().expect("temp dir should be created");
    let db_a = test_ipc_db(&root, "agent-a");
    let db_b = test_ipc_db(&root, "agent-b");
    let send_tool = SendMessageTool::new(db_a, Arc::new(SecurityPolicy::default()));

    let error = send_tool
        .execute(json!({ "to_agent": db_b.agent_id() }))
        .await
        .expect_err("missing payload should fail deserialization");

    assert!(error.to_string().contains("missing field `payload`"));
}

#[tokio::test]
async fn send_message_rejects_blank_payload_and_invalid_routes() {
    let root = TempDir::new().expect("temp dir should be created");
    let db_a = test_ipc_db(&root, "agent-a");
    let db_b = test_ipc_db(&root, "agent-b");
    let send_tool = SendMessageTool::new(db_a, Arc::new(SecurityPolicy::default()));

    let blank_payload = send_tool
        .execute(json!({
            "to_agent": db_b.agent_id(),
            "payload": "   "
        }))
        .await
        .expect_err("blank payload should be rejected");
    assert!(blank_payload.to_string().contains("payload must not be empty"));

    let missing_target = send_tool
        .execute(json!({
            "payload": "hello"
        }))
        .await
        .expect_err("missing route should be rejected");
    assert!(missing_target.to_string().contains("either 'to_agent' or 'team_id' is required"));

    let blank_target = send_tool
        .execute(json!({
            "to_agent": "  ",
            "payload": "hello"
        }))
        .await
        .expect_err("blank direct target should be rejected");
    assert!(blank_target.to_string().contains("either 'to_agent' or 'team_id' is required"));

    let blank_team = send_tool
        .execute(json!({
            "team_id": "  ",
            "to_agent": db_b.agent_id(),
            "payload": "hello"
        }))
        .await
        .expect_err("blank team target should be rejected");
    assert!(blank_team.to_string().contains("team id must not be empty"));
}
