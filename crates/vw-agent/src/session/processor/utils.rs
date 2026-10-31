//! 会话处理器工具模块。

mod docs;
mod file_links;
mod message_format;
mod output_compaction;
mod session_ingest;
mod tool_parsing;

pub(crate) use docs::{is_docs_request, list_docs};
#[allow(unused_imports)]
pub(crate) use file_links::{
    build_file_link, compact_file_link, extract_file_link_blocks, extract_file_path_from_input,
    maybe_inject_file_link, resolve_full_path,
};
pub(crate) use message_format::{
    assistant_message_with_reasoning, now_ms, tool_calls_to_assistant_message,
    tool_result_to_message,
};
#[allow(unused_imports)]
pub(crate) use output_compaction::{
    compact_tool_output, compact_tool_output_for_ui, rewrite_todowrite_completed_when_no_work,
    sanitize_tool_input, sanitize_tool_input_for_ui, tool_fingerprint, truncate_string,
};
#[allow(unused_imports)]
pub(crate) use session_ingest::{ingest_assistant_answer, ingest_user_query, push_user_dedup};
#[allow(unused_imports)]
pub(crate) use tool_parsing::{
    is_valid_tool_name, parse_tool_at, query_has_any_tool_calls,
    query_has_any_tool_calls_with_allowed,
};
#[cfg(test)]
#[path = "utils_tests.rs"]
mod utils_tests;
