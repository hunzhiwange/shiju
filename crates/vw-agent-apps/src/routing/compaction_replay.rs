//! Codex compaction history replay for upstream protocols without opaque Responses state.
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};

const COMPACTION_PREFIX: &str = "ccswitch-compaction-v1:";
const SUMMARY_PREFIX: &str = concat!(
    "Another language model started to solve this problem and produced a summary of its thinking process. ",
    "You also have access to the state of the tools that were used to solve the problem. ",
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
    let summary = payload
        .strip_prefix(COMPACTION_PREFIX)
        .and_then(|encoded| URL_SAFE_NO_PAD.decode(encoded).ok())
        .and_then(|bytes| String::from_utf8(bytes).ok());
    Some(match summary {
        Some(summary) => format!("{SUMMARY_PREFIX}\n{summary}"),
        None => OPAQUE_COMPACTION_NOTE.to_owned(),
    })
}
