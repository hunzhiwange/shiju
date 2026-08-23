// 本文件验证 Lark 与飞书配置元数据、默认值及群聊回复辅助规则。

use super::lark::{FeishuConfig, LarkConfig};
use crate::app::agent::config::traits::ChannelConfig;

#[test]
// 两种平台外形相近，但元数据须独立以选择正确端点。
fn lark_and_feishu_metadata_are_distinct() {
    assert_eq!(LarkConfig::name(), "Lark");
    assert_eq!(FeishuConfig::name(), "Feishu");
    assert_eq!(LarkConfig::desc(), "Lark Bot");
    assert_eq!(FeishuConfig::desc(), "Feishu Bot");
}

#[test]
fn lark_defaults_and_group_reply_helpers_are_available() {
    let lark: LarkConfig = serde_json::from_value(serde_json::json!({
        "app_id": "app",
        "app_secret": "secret",
        "mention_only": true,
        "group_reply": {
            "mode": "all_messages",
            "allowed_sender_ids": ["u1", "u2"]
        }
    }))
    .unwrap();

    assert_eq!(lark.receive_mode, Default::default());
    assert_eq!(lark.draft_update_interval_ms, 3000);
    assert_eq!(lark.max_draft_edits, 20);
    assert_eq!(format!("{:?}", lark.effective_group_reply_mode()), "AllMessages");
    assert_eq!(lark.group_reply_allowed_sender_ids(), vec!["u1".to_string(), "u2".to_string()]);
}
