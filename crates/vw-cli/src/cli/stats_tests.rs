//! CLI 会话统计标题的格式契约测试。
//! 区分尚无用量时的占位状态与已有输入、输出 token 时的汇总展示，并保留 Provider 和模型标识。

use super::stats::{CliStats, build_session_title};

#[test]
fn build_session_title_uses_token_placeholder_until_usage_exists() {
    let empty = CliStats::default();
    assert!(build_session_title(&empty, "openai", "gpt").contains("Context --"));

    let stats = CliStats { input_tokens: 10, output_tokens: 7, ..CliStats::default() };
    let title = build_session_title(&stats, "openai", "gpt");
    assert!(title.contains("Context 17 tokens"));
    assert!(title.contains("openai / gpt"));
}
