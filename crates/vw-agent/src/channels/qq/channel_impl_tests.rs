use super::*;
use crate::app::agent::channels::traits::{Channel, SendMessage};
use crate::app::agent::config::{
    ProxyConfig, ProxyScope, runtime_proxy_config, set_runtime_proxy_config,
};

struct RuntimeProxyConfigGuard {
    _lock: std::sync::MutexGuard<'static, ()>,
    previous: ProxyConfig,
}

impl RuntimeProxyConfigGuard {
    fn install_failing_channel_proxy() -> Self {
        let lock = crate::test_support::global_runtime_proxy_lock()
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let previous = runtime_proxy_config();
        set_runtime_proxy_config(ProxyConfig {
            enabled: true,
            https_proxy: Some("http://127.0.0.1:1".to_string()),
            scope: ProxyScope::Services,
            services: vec!["channel.qq".to_string()],
            ..Default::default()
        });
        Self { _lock: lock, previous }
    }
}

impl Drop for RuntimeProxyConfigGuard {
    fn drop(&mut self) {
        set_runtime_proxy_config(self.previous.clone());
    }
}

#[test]
fn qq_channel_trait_name_is_static() {
    let channel = QQChannel::new("app".to_string(), "secret".to_string(), vec![]);
    let as_trait: &dyn Channel = &channel;

    assert_eq!(as_trait.name(), "qq");
}

#[tokio::test]
async fn qq_channel_trait_send_reuses_cached_token_for_empty_message() {
    let channel = QQChannel::new("app".to_string(), "secret".to_string(), vec![]);
    *channel.token_cache.write().await = Some(("cached-token".to_string(), u64::MAX));

    let message = SendMessage::new("", "user:recipient");
    let as_trait: &dyn Channel = &channel;

    assert!(as_trait.send(&message).await.is_ok());
}

#[tokio::test]
async fn qq_channel_trait_listen_surfaces_gateway_setup_errors() {
    let _guard = RuntimeProxyConfigGuard::install_failing_channel_proxy();
    let channel = QQChannel::new("app".to_string(), "secret".to_string(), vec![]);
    let as_trait: &dyn Channel = &channel;
    let (tx, mut rx) = tokio::sync::mpsc::channel(1);

    let result = as_trait.listen(tx).await;

    assert!(result.is_err());
    assert!(rx.try_recv().is_err());
}

#[tokio::test]
async fn qq_channel_trait_health_check_returns_false_when_auth_request_fails() {
    let _guard = RuntimeProxyConfigGuard::install_failing_channel_proxy();
    let channel = QQChannel::new("app".to_string(), "secret".to_string(), vec![]);
    let as_trait: &dyn Channel = &channel;

    assert!(!as_trait.health_check().await);
}
// 本文件验证 QQ 渠道接口的令牌复用、监听错误与健康检查。
