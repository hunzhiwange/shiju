use super::TelegramChannel;

#[test]
fn channel_without_pairing_has_no_pairing_guard() {
    let channel = TelegramChannel::new("token".to_string(), vec![], false);

    assert!(channel.pairing.is_none());
}
// 本文件验证未配置配对策略时 Telegram 渠道不会启用配对守卫。
