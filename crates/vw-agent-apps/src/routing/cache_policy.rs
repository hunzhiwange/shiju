//! Cache 断点注入器
//!
//! 在请求转发前自动注入 cache_control 标记，启用 Bedrock Prompt Caching

use serde_json::{Value, json};

/// 在请求体关键位置注入 cache_control 断点
pub fn inject_messages(body: &mut Value, enabled: bool) {
    if !enabled {
        return;
    }

    let existing = count_existing(body);

    let mut budget = 4_usize.saturating_sub(existing);
    if budget == 0 {
        return;
    }

    // (a) tools 末尾
    if budget > 0 {
        if let Some(tools) = body.get_mut("tools").and_then(|t| t.as_array_mut()) {
            if let Some(last) = tools.last_mut() {
                if last.get("cache_control").is_none() {
                    if let Some(o) = last.as_object_mut() {
                        o.insert("cache_control".to_string(), make_cache_control());
                    }
                    budget -= 1;
                }
            }
        }
    }

    // (b) system 末尾
    if budget > 0 {
        // 字符串 system → 转为数组
        if let Some(text) = body.get("system").and_then(|s| s.as_str()).map(str::to_string) {
            body["system"] = json!([{"type": "text", "text": text}]);
        }

        if let Some(system) = body.get_mut("system").and_then(|s| s.as_array_mut()) {
            if let Some(last) = system.last_mut() {
                if last.get("cache_control").is_none() {
                    if let Some(o) = last.as_object_mut() {
                        o.insert("cache_control".to_string(), make_cache_control());
                    }
                    budget -= 1;
                }
            }
        }
    }

    // (c) 最后一条可缓存消息的最后一个非 thinking block。工具循环通常以
    // user/tool_result 结束；只标 assistant 会让最新稳定前缀无法命中缓存。
    if budget > 0 {
        if let Some(messages) = body.get_mut("messages").and_then(|m| m.as_array_mut()) {
            for message in messages.iter_mut().rev() {
                if inject_message_breakpoint(message) {
                    budget -= 1;
                    break;
                }
            }

            // (d) A second, older user anchor helps long tool-result turns where
            // the stable prefix falls outside Anthropic's 20-block lookback from
            // the newest breakpoint. Keep this best-effort and inside the 4-BP cap.
            if budget > 0 && messages.len() >= 4 {
                let mut user_count = 0;
                for message in messages.iter_mut().rev() {
                    if message.get("role").and_then(Value::as_str) != Some("user") {
                        continue;
                    }
                    user_count += 1;
                    if user_count == 2 {
                        inject_message_breakpoint(message);
                        break;
                    }
                }
            }
        }
    }
}

fn inject_message_breakpoint(message: &mut Value) -> bool {
    let Some(content) = message.get_mut("content").and_then(Value::as_array_mut) else {
        return false;
    };
    let Some(block) = content.iter_mut().rev().find(|block| {
        !matches!(block.get("type").and_then(Value::as_str), Some("thinking" | "redacted_thinking"))
    }) else {
        return false;
    };
    if block.get("cache_control").is_some() {
        return false;
    }
    let Some(object) = block.as_object_mut() else {
        return false;
    };
    object.insert("cache_control".to_string(), make_cache_control());
    true
}

fn make_cache_control() -> Value {
    json!({"type": "ephemeral"})
}

fn count_existing(body: &Value) -> usize {
    let mut count = 0;

    if let Some(tools) = body.get("tools").and_then(|t| t.as_array()) {
        count += tools.iter().filter(|t| t.get("cache_control").is_some()).count();
    }

    if let Some(system) = body.get("system").and_then(|s| s.as_array()) {
        count += system.iter().filter(|b| b.get("cache_control").is_some()).count();
    }

    if let Some(messages) = body.get("messages").and_then(|m| m.as_array()) {
        for msg in messages {
            if let Some(content) = msg.get("content").and_then(|c| c.as_array()) {
                count += content.iter().filter(|b| b.get("cache_control").is_some()).count();
            }
        }
    }

    count
}

/// Generate bounded opaque identities from the authorized scope and a real
/// client session. Explicit caller keys remain untouched.
pub fn inject_key(body: &mut Value, enabled: bool, scope: &[u8], session: Option<&str>) {
    if !enabled || body.get("prompt_cache_key").is_some() {
        return;
    }
    let Some(session) = session.map(str::trim).filter(|s| !s.is_empty() && s.len() <= 4096) else {
        return;
    };
    use sha2::{Digest, Sha256};
    let mut hash = Sha256::new();
    hash.update(b"shiju-cache-v1");
    hash.update((scope.len() as u64).to_be_bytes());
    hash.update(scope);
    hash.update(session.as_bytes());
    let key: String = hash.finalize().iter().map(|b| format!("{b:02x}")).collect();
    if let Some(object) = body.as_object_mut() {
        object.insert("prompt_cache_key".into(), Value::String(key));
    }
}
