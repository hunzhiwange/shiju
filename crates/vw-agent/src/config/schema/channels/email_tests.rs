// 本文件验证邮件渠道元数据及 TLS、端口和收件箱默认值。

use super::email::EmailConfig;
use crate::app::agent::config::traits::ChannelConfig;

#[test]
fn email_channel_metadata_is_stable() {
    assert_eq!(EmailConfig::name(), "Email");
    assert_eq!(EmailConfig::desc(), "Email over IMAP/SMTP");
}

#[test]
// 默认端口与 TLS 组合须一致，避免初始配置产生不安全连接。
fn email_default_keeps_tls_ports_and_inbox_defaults() {
    let config = EmailConfig::default();

    assert_eq!(config.imap_port, 993);
    assert_eq!(config.smtp_port, 465);
    assert_eq!(config.imap_folder, "INBOX");
    assert!(config.smtp_tls);
    assert_eq!(config.idle_timeout_secs, 1740);
}
