//! Codex compaction history replay for upstream protocols without opaque Responses state.
use serde_json::{Value, json};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};

const COMPACTION_PREFIX: &str = "ccswitch-compaction-v1:";
const SUMMARY_PREFIX: &str = concat!(
    "Another language model started to solve this problem and produced a summary of its thinking process. ",
    "You also have access to the state of the tools that were used by that language model. ",
    "Use this to build on the work that has already been done and avoid duplicating work. ",
    "Here is the summary produced by the other language model, use the information in this summary to assist with your own analysis:"
);
const OPAQUE_COMPACTION_NOTE: &str = "[earlier conversation was compacted; the summary is stored in a format this model cannot read]";

/// Encode the UTF-8 summary in the compaction envelope that Codex can replay.
pub fn encode_summary(summary: &str) -> String {
    format!("{COMPACTION_PREFIX}{}", URL_SAFE_NO_PAD.encode(summary))
}

/// Replay a compatible summary or explain that opaque compaction state cannot be read.
pub fn replay_text(payload: &str) -> Option<String> {
    if payload.is_empty() {
        return None;
    }
    // This legacy envelope stores UTF-8 summary text. Chat has no field for
    // opaque Responses state, so only the known envelope is interpreted.
    let summary = decode_summary(payload);
    Some(match summary {
        Some(summary) => format!("{SUMMARY_PREFIX}\n{summary}"),
        None => OPAQUE_COMPACTION_NOTE.to_owned(),
    })
}

fn decode_summary(payload: &str) -> Option<String> {
    let encoded = payload.strip_prefix(COMPACTION_PREFIX)?;
    String::from_utf8(URL_SAFE_NO_PAD.decode(encoded).ok()?).ok()
}

fn is_compaction(item: &Value) -> bool {
    matches!(item["type"].as_str(), Some("compaction" | "compaction_summary" | "context_compaction"))
}

/// Replay only recognized compaction item types. Payloadless markers carry no text.
pub fn replay_item(item: &Value) -> Option<String> {
    if !is_compaction(item) {
        return None;
    }
    replay_text(item["encrypted_content"].as_str()?)
}

/// Native Responses can understand their own opaque state. Decode only our known
/// envelope; leave unrecognized ciphertext for the upstream's explicit rejection.
/// Official destinations additionally cannot consume converted reasoning state.
pub fn prepare_native(body: &mut Value, official: bool) {
    let stored = body["store"] == true;
    let Some(items) = body.get_mut("input").and_then(Value::as_array_mut) else {
        return;
    };
    let mut replayed = Vec::with_capacity(items.len());
    for mut item in std::mem::take(items) {
        if is_compaction(&item)
            && let Some(summary) = item["encrypted_content"].as_str().and_then(decode_summary)
        {
            replayed.push(json!({"type":"message", "role":"user",
                "content":[{"type":"input_text", "text":format!("{SUMMARY_PREFIX}\n{summary}")}]}));
            continue;
        }
        if official {
            let kind = item["type"].as_str().unwrap_or_default();
            if kind == "reasoning" {
                let incompatible = match item["encrypted_content"].as_str().filter(|s| !s.is_empty()) {
                    Some(payload) => payload.starts_with("ccswitch-"),
                    None => !stored,
                };
                if incompatible {
                    continue;
                }
            }
            let prefix = match kind {
                "message" => Some("msg_"),
                "agent_message" => Some("amsg_"),
                "reasoning" => Some("rs_"),
                "function_call" => Some("fc_"),
                "custom_tool_call" => Some("ctc_"),
                "tool_search_call" => Some("tsc_"),
                "web_search_call" => Some("ws_"),
                _ => None,
            };
            if let Some(prefix) = prefix
                && item.get("id").is_some_and(|id| !id.as_str().is_some_and(|id| id.starts_with(prefix)))
                && let Some(object) = item.as_object_mut()
            {
                object.remove("id");
            }
        }
        replayed.push(item);
    }
    *items = replayed;
}
