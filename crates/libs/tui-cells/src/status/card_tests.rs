use std::path::PathBuf;

use super::*;
use crate::V3TokenUsage;

// 构造覆盖会话标题及各类令牌计数的稳定展示数据。
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
        resumed: true,
    }
}

#[test]
// 验证卡片只显示 v3 桥接数据，不混入服务限额等外部状态。
fn status_indicator_card_uses_vibe_data_without_service_limits() {
    let cell = StatusHistoryCell::new(fixture());
    let rendered = cell
        .display_lines(80)
        .into_iter()
        .map(|line| line.to_string())
        .collect::<Vec<_>>()
        .join("\n");
    insta::assert_snapshot!(rendered);
}

#[test]
// 验证窄终端下字段仍按卡片布局折叠并保持可读。
fn status_indicator_card_folds_at_narrow_width() {
    let cell = StatusHistoryCell::new(fixture());
    let rendered = cell
        .display_lines(36)
        .into_iter()
        .map(|line| line.to_string())
        .collect::<Vec<_>>()
        .join("\n");
    insta::assert_snapshot!(rendered);
}
