use std::path::PathBuf;

use super::*;
use crate::V3TokenUsage;

fn fixture() -> V3SessionDisplayData {
    V3SessionDisplayData {
        version: "0.4.0".to_string(),
        cwd: PathBuf::from("/workspace/vibe-window"),
        gateway_endpoint: "127.0.0.1:34112".to_string(),
        model: "openai/gpt-5.6".to_string(),
        permissions: "Plan · workspace write".to_string(),
        session_id: Some("session-14".to_string()),
        session_title: Some("Port welcome surfaces".to_string()),
        token_usage: V3TokenUsage { input: 1_024, output: 512, cached: 128, reasoning: 64 },
        resumed: false,
    }
}

#[test]
fn session_header_keeps_copied_border_and_narrow_fold() {
    let cell = SessionHistoryCell::header(fixture());
    let wide = cell
        .display_lines(80)
        .into_iter()
        .map(|line| line.to_string())
        .collect::<Vec<_>>()
        .join("\n");
    let narrow = cell
        .display_lines(34)
        .into_iter()
        .map(|line| line.to_string())
        .collect::<Vec<_>>()
        .join("\n");
    insta::assert_snapshot!("session_header_vibe_wide", wide);
    insta::assert_snapshot!("session_header_vibe_narrow", narrow);
}

#[test]
fn v3_welcome_status_and_session_header_keep_codex_layout_with_vibe_data() {
    let data = fixture();
    let header = SessionHistoryCell::header(data.clone());
    let header_text = header
        .display_lines(80)
        .into_iter()
        .map(|line| line.to_string())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(header_text.contains(">_ ShiJu (v0.4.0)"));
    assert!(header_text.contains("model:       openai/gpt-5.6"));
    assert!(header_text.contains("directory:   /workspace/vibe-window"));

    let status = crate::StatusHistoryCell::new(data);
    let status_text = status
        .display_lines(80)
        .into_iter()
        .map(|line| line.to_string())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(status_text.contains(">_ ShiJu (v0.4.0)"));
    assert!(status_text.contains("Gateway:"));
    assert!(!status_text.contains("OpenAI"));
    assert!(!status_text.contains("ChatGPT"));
}
