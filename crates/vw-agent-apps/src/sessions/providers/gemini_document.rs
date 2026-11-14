use std::collections::HashMap;
use serde_json::Value;

/// 把会话文件还原成旧版单个 JSON 对象的形状 `{sessionId, ..., messages: [...]}`。
///
/// 旧版 `.json` 整个文件就是这个对象，原样返回。新版 `.jsonl` 按 Gemini CLI 的
/// `loadConversationRecord` 回放：带 `sessionId` 的行是元数据；带 `id` 的行是消息，
/// 同 id 后写覆盖先写（位置不变）；`{"$set": {...}}` 合并元数据，带 `messages` 时整体
/// 替换消息；`{"$rewindTo": id}` 删掉该条及之后的消息，找不到 id 时清空。
pub(crate) fn parse_session_document(data: &str) -> Option<Value> {
    // 只有元数据一行的 `.jsonl` 也能整体解析成对象，补上空的 messages
    if let Ok(Value::Object(mut map)) = serde_json::from_str::<Value>(data) {
        map.entry("messages")
            .or_insert_with(|| Value::Array(Vec::new()));
        return Some(Value::Object(map));
    }

    let mut metadata = serde_json::Map::new();
    let mut log = MessageLog::default();
    let mut seen_record = false;

    for line in data.lines() {
        if line.trim().is_empty() { continue; }
        let Ok(Value::Object(mut record)) = serde_json::from_str::<Value>(line) else {
            return None;
        };
        seen_record = true;
        if let Some(id) = record.get("$rewindTo").and_then(Value::as_str) {
            log.rewind_to(id);
        } else if record.get("id").is_some_and(Value::is_string) {
            log.upsert(Value::Object(record));
        } else if let Some(Value::Object(mut set)) = record.remove("$set") {
            if let Some(Value::Array(list)) = set.remove("messages") {
                log.reset(list);
            }
            metadata.extend(set);
        } else if record.get("sessionId").is_some_and(Value::is_string) {
            if let Some(Value::Array(list)) = record.remove("messages") {
                for msg in list {
                    log.upsert(msg);
                }
            }
            metadata.extend(record);
        }
    }

    if !seen_record {
        return None;
    }
    metadata.insert("messages".to_string(), Value::Array(log.messages));
    Some(Value::Object(metadata))
}

/// JSONL 回放中的消息表：同 id 原位覆盖、保持首次出现的顺序（同上游的 JS `Map`），
/// 按 id 建索引，整体回放是线性的。
#[derive(Default)]
struct MessageLog {
    messages: Vec<Value>,
    index: HashMap<String, usize>,
}

impl MessageLog {
    /// 没有字符串 `id` 的不是消息，忽略
    fn upsert(&mut self, msg: Value) {
        let Some(id) = msg.get("id").and_then(Value::as_str).map(str::to_string) else {
            return;
        };
        match self.index.get(&id) {
            Some(&i) => self.messages[i] = msg,
            None => {
                self.index.insert(id, self.messages.len());
                self.messages.push(msg);
            }
        }
    }

    /// 删掉该条及之后的消息；找不到 id 时清空
    fn rewind_to(&mut self, id: &str) {
        let len = self.index.get(id).copied().unwrap_or(0);
        self.messages.truncate(len);
        self.index.retain(|_, i| *i < len);
    }

    /// `$set.messages` 检查点：整体替换
    fn reset(&mut self, list: Vec<Value>) {
        self.messages.clear();
        self.index.clear();
        for msg in list {
            self.upsert(msg);
        }
    }
}

