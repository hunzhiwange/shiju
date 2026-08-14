use super::{FeishuConfig, LarkConfig, LarkReceiveMode};

fn lark_config(app_id: &str, app_secret: &str) -> LarkConfig {
    LarkConfig {
        app_id: app_id.to_owned(),
        app_secret: app_secret.to_owned(),
        encrypt_key: None,
        verification_token: None,
        allowed_users: Vec::new(),
        mention_only: false,
        group_reply: None,
        use_feishu: false,
        receive_mode: LarkReceiveMode::Websocket,
        port: None,
        draft_update_interval_ms: 3000,
        max_draft_edits: 20,
    }
}

fn feishu_config(app_id: &str, app_secret: &str) -> FeishuConfig {
    FeishuConfig {
        app_id: app_id.to_owned(),
        app_secret: app_secret.to_owned(),
        encrypt_key: None,
        verification_token: None,
        allowed_users: Vec::new(),
        group_reply: None,
        receive_mode: LarkReceiveMode::Websocket,
        port: None,
        draft_update_interval_ms: 3000,
        max_draft_edits: 20,
    }
}

#[test]
fn lark_and_feishu_required_credentials_reject_blank_or_partial_values() {
    let cases = [
        ("app-id", "app-secret", true),
        ("", "", false),
        (" \t", "\n", false),
        ("app-id", "", false),
        ("", "app-secret", false),
    ];

    for (app_id, app_secret, expected) in cases {
        assert_eq!(lark_config(app_id, app_secret).has_required_credentials(), expected);
        assert_eq!(feishu_config(app_id, app_secret).has_required_credentials(), expected);
    }
}
