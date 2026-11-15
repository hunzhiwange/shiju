//! Typed context updates produced when invoked skills are restored.
use serde_json::Value;

#[derive(Debug, Clone, PartialEq)]
pub struct InvokedSkillContextUpdate {
    pub key: String,
    pub value: Value,
}

use agent_skills::invoked::InvokedSkillReplay;
const INVOKED_SKILL_SOURCE_FIELD: &str = "vw_invoked_skill_source";

/// 在压缩清理后将已调用技能的临时元用户消息放回历史前方。
///
/// 该函数按稳定来源标识替换旧注入，因此可以在每次压缩后调用而不会重复注入。
pub fn restore_invoked_skill_context(
    messages: &mut Vec<Value>,
    replays: &[InvokedSkillReplay],
) -> Vec<InvokedSkillContextUpdate> {
    messages.retain(|message| {
        message.get(INVOKED_SKILL_SOURCE_FIELD).is_none()
            && message
                .get("_vw_source")
                .and_then(|source| source.get("form"))
                .and_then(Value::as_str)
                != Some("invoked_skill")
    });

    let mut restored_messages = replays
        .iter()
        .map(|replay| {
            serde_json::json!({
                "role": "user",
                "content": replay.content,
                INVOKED_SKILL_SOURCE_FIELD: replay.stable_id,
            })
        })
        .collect::<Vec<_>>();
    restored_messages.append(messages);
    *messages = restored_messages;

    replays
        .iter()
        .flat_map(|replay| {
            let mut updates = vec![InvokedSkillContextUpdate {
                key: "skill.name".to_string(),
                value: Value::String(replay.stable_id.clone()),
            }];
            if !replay.allowed_tools.is_empty() {
                updates.push(InvokedSkillContextUpdate {
                    key: "skill.allowed_tools".to_string(),
                    value: serde_json::json!(replay.allowed_tools),
                });
            }
            if let Some(model) = &replay.model {
                updates.push(InvokedSkillContextUpdate {
                    key: "skill.model".to_string(),
                    value: Value::String(model.clone()),
                });
            }
            if let Some(effort) = &replay.effort {
                updates.push(InvokedSkillContextUpdate {
                    key: "skill.effort".to_string(),
                    value: Value::String(effort.clone()),
                });
            }
            updates
        })
        .collect()
}
