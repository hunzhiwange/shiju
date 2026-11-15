// 本文件验证未配置配对策略时 Telegram 渠道不会启用配对守卫。

use super::TelegramChannel;

#[test]
// 未配置配对时保持守卫缺席，避免无意改变认证流程。
fn channel_without_pairing_has_no_pairing_guard() {
    let channel = TelegramChannel::new("token".to_string(), vec![], false);

    assert!(channel.pairing.is_none());
}
