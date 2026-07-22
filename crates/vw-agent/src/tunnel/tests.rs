//! 隧道模块的单元测试
//!
//! 本模块包含对隧道工厂函数和各隧道实现的测试用例，覆盖以下方面：
//! - 工厂函数对不同配置的响应（空配置、未知 provider、缺失必需配置等）
//! - 各隧道实现的基本属性（名称、公共 URL）
//! - 进程生命周期管理（启动、终止）
//! - 健康检查行为

use super::*;
use crate::app::agent::config::{
    CloudflareTunnelConfig, CustomTunnelConfig, NgrokTunnelConfig, TunnelConfig,
};
use tokio::process::Command;

/// 断言创建隧道时返回包含指定文本的错误
///
/// # 参数
///
/// * `cfg` - 隧道配置引用
/// * `needle` - 期望错误消息中包含的子串
///
/// # 行为
///
/// 如果 `create_tunnel(cfg)` 返回错误且错误消息包含 `needle`，测试通过。
/// 如果返回成功或错误消息不包含 `needle`，测试失败。
fn assert_tunnel_err(cfg: &TunnelConfig, needle: &str) {
    match create_tunnel(cfg) {
        Err(e) => assert!(
            e.to_string().contains(needle),
            "Expected error containing \"{needle}\", got: {e}"
        ),
        Ok(_) => panic!("Expected error containing \"{needle}\", but got Ok"),
    }
}

/// 测试工厂函数：默认配置应返回 None
///
/// 当 TunnelConfig 使用默认值时（provider 为空），
/// create_tunnel 应返回 Ok(None)，表示不创建任何隧道。
#[test]
fn factory_none_returns_none() {
    let cfg = TunnelConfig::default();
    let t = create_tunnel(&cfg).unwrap();
    assert!(t.is_none());
}

/// 测试工厂函数：空字符串 provider 应返回 None
///
/// 当 provider 显式设置为空字符串时，
/// create_tunnel 应返回 Ok(None)，表示不创建任何隧道。
#[test]
fn factory_empty_string_returns_none() {
    let cfg = TunnelConfig { provider: String::new(), ..TunnelConfig::default() };
    let t = create_tunnel(&cfg).unwrap();
    assert!(t.is_none());
}

/// 测试工厂函数：未知 provider 应返回错误
///
/// 当 provider 设置为不支持的值（如 "wireguard"）时，
/// create_tunnel 应返回包含 "Unknown tunnel provider" 的错误。
#[test]
fn factory_unknown_provider_errors() {
    let cfg = TunnelConfig { provider: "wireguard".into(), ..TunnelConfig::default() };
    assert_tunnel_err(&cfg, "Unknown tunnel provider");
}

/// 测试工厂函数：Cloudflare 隧道缺少配置应返回错误
///
/// 当 provider 设置为 "cloudflare" 但未提供 cloudflare 配置时，
/// create_tunnel 应返回包含 "[tunnel.cloudflare]" 的错误。
#[test]
fn factory_cloudflare_missing_config_errors() {
    let cfg = TunnelConfig { provider: "cloudflare".into(), ..TunnelConfig::default() };
    assert_tunnel_err(&cfg, "[tunnel.cloudflare]");
}

/// 测试工厂函数：Cloudflare 隧道有效配置应成功创建
///
/// 当 provider 设置为 "cloudflare" 且提供了有效的 cloudflare 配置时，
/// create_tunnel 应返回 Ok(Some(...))，且隧道名称为 "cloudflare"。
#[test]
fn factory_cloudflare_with_config_ok() {
    let cfg = TunnelConfig {
        provider: "cloudflare".into(),
        cloudflare: Some(CloudflareTunnelConfig { token: "test-token".into() }),
        ..TunnelConfig::default()
    };
    let t = create_tunnel(&cfg).unwrap();
    assert!(t.is_some());
    assert_eq!(t.unwrap().name(), "cloudflare");
}

/// 测试工厂函数：Tailscale 隧道使用默认配置应成功创建
///
/// Tailscale 隧道不需要额外配置，仅设置 provider 为 "tailscale"
/// 即可成功创建，返回的隧道名称为 "tailscale"。
#[test]
fn factory_tailscale_defaults_ok() {
    let cfg = TunnelConfig { provider: "tailscale".into(), ..TunnelConfig::default() };
    let t = create_tunnel(&cfg).unwrap();
    assert!(t.is_some());
    assert_eq!(t.unwrap().name(), "tailscale");
}

/// 测试工厂函数：Ngrok 隧道缺少配置应返回错误
///
/// 当 provider 设置为 "ngrok" 但未提供 ngrok 配置时，
/// create_tunnel 应返回包含 "[tunnel.ngrok]" 的错误。
#[test]
fn factory_ngrok_missing_config_errors() {
    let cfg = TunnelConfig { provider: "ngrok".into(), ..TunnelConfig::default() };
    assert_tunnel_err(&cfg, "[tunnel.ngrok]");
}

/// 测试工厂函数：Ngrok 隧道有效配置应成功创建
///
/// 当 provider 设置为 "ngrok" 且提供了有效的 ngrok 配置（至少包含 auth_token）时，
/// create_tunnel 应返回 Ok(Some(...))，且隧道名称为 "ngrok"。
#[test]
fn factory_ngrok_with_config_ok() {
    let cfg = TunnelConfig {
        provider: "ngrok".into(),
        ngrok: Some(NgrokTunnelConfig { auth_token: "tok".into(), domain: None }),
        ..TunnelConfig::default()
    };
    let t = create_tunnel(&cfg).unwrap();
    assert!(t.is_some());
    assert_eq!(t.unwrap().name(), "ngrok");
}

/// 测试工厂函数：自定义隧道缺少配置应返回错误
///
/// 当 provider 设置为 "custom" 但未提供 custom 配置时，
/// create_tunnel 应返回包含 "[tunnel.custom]" 的错误。
#[test]
fn factory_custom_missing_config_errors() {
    let cfg = TunnelConfig { provider: "custom".into(), ..TunnelConfig::default() };
    assert_tunnel_err(&cfg, "[tunnel.custom]");
}

/// 测试工厂函数：自定义隧道有效配置应成功创建
///
/// 当 provider 设置为 "custom" 且提供了有效的 custom 配置（至少包含 start_command）时，
/// create_tunnel 应返回 Ok(Some(...))，且隧道名称为 "custom"。
#[test]
fn factory_custom_with_config_ok() {
    let cfg = TunnelConfig {
        provider: "custom".into(),
        custom: Some(CustomTunnelConfig {
            url: None,
            auth_token: None,
            start_command: "echo tunnel".into(),
            health_url: None,
            url_pattern: None,
        }),
        ..TunnelConfig::default()
    };
    let t = create_tunnel(&cfg).unwrap();
    assert!(t.is_some());
    assert_eq!(t.unwrap().name(), "custom");
}

/// 测试工厂函数：空白的自定义隧道配置应视为未启用
///
/// 当 provider 为 "custom" 但 custom 配置中的所有字段都为空时，
/// create_tunnel 应返回 Ok(None)，避免网关启动时先报 warning 再回退本地模式。
#[test]
fn factory_custom_blank_config_returns_none() {
    let cfg = TunnelConfig {
        provider: "custom".into(),
        custom: Some(CustomTunnelConfig {
            url: None,
            auth_token: None,
            start_command: "   ".into(),
            health_url: None,
            url_pattern: None,
        }),
        ..TunnelConfig::default()
    };
    let t = create_tunnel(&cfg).unwrap();
    assert!(t.is_none());
}

#[test]
fn factory_custom_each_optional_field_enables_tunnel() {
    let optional_configs = [
        CustomTunnelConfig {
            url: Some("https://public.example".into()),
            auth_token: None,
            start_command: String::new(),
            health_url: None,
            url_pattern: None,
        },
        CustomTunnelConfig {
            url: None,
            auth_token: Some("secret".into()),
            start_command: String::new(),
            health_url: None,
            url_pattern: None,
        },
        CustomTunnelConfig {
            url: None,
            auth_token: None,
            start_command: String::new(),
            health_url: Some("https://health.example".into()),
            url_pattern: None,
        },
        CustomTunnelConfig {
            url: None,
            auth_token: None,
            start_command: String::new(),
            health_url: None,
            url_pattern: Some("https://[^ ]+".into()),
        },
    ];

    for custom in optional_configs {
        let cfg = TunnelConfig {
            provider: "custom".into(),
            custom: Some(custom),
            ..TunnelConfig::default()
        };

        let tunnel = create_tunnel(&cfg).unwrap();
        assert!(tunnel.is_some());
        assert_eq!(tunnel.unwrap().name(), "custom");
    }
}

/// 测试 NoneTunnel 的名称属性
///
/// NoneTunnel 的 name() 方法应返回 "none"。
#[test]
fn none_tunnel_name() {
    let t = NoneTunnel;
    assert_eq!(t.name(), "none");
}

/// 测试 NoneTunnel 的公共 URL 属性
///
/// NoneTunnel 的 public_url() 方法应返回 None，表示它不提供公共访问地址。
#[test]
fn none_tunnel_public_url_is_none() {
    let t = NoneTunnel;
    assert!(t.public_url().is_none());
}

/// 测试 NoneTunnel 的健康检查始终返回 true
///
/// NoneTunnel 不依赖任何外部服务，因此健康检查应始终成功。
#[tokio::test]
async fn none_tunnel_health_always_true() {
    let t = NoneTunnel;
    assert!(t.health_check().await);
}

/// 测试 NoneTunnel 的 start 方法返回本地地址
///
/// NoneTunnel 的 start 方法应直接返回传入的本地地址，
/// 不进行任何实际的隧道建立操作。
///
/// # 参数
/// - host: 本地主机地址
/// - port: 本地端口
///
/// # 返回
/// 格式为 "http://{host}:{port}" 的 URL 字符串
#[tokio::test]
async fn none_tunnel_start_returns_local() {
    let t = NoneTunnel;
    let url = t.start("127.0.0.1", 8080).await.unwrap();
    assert_eq!(url, "http://127.0.0.1:8080");
}

/// 测试 CloudflareTunnel 的基本属性
///
/// CloudflareTunnel 创建后应：
/// - name() 返回 "cloudflare"
/// - public_url() 返回 None（启动前无公共 URL）
#[test]
fn cloudflare_tunnel_name() {
    let t = CloudflareTunnel::new("tok".into());
    assert_eq!(t.name(), "cloudflare");
    assert!(t.public_url().is_none());
}

/// 测试 TailscaleTunnel 的基本属性（非 Funnel 模式）
///
/// TailscaleTunnel 创建后应：
/// - name() 返回 "tailscale"
/// - public_url() 返回 None（启动前无公共 URL）
#[test]
fn tailscale_tunnel_name() {
    let t = TailscaleTunnel::new(false, None);
    assert_eq!(t.name(), "tailscale");
    assert!(t.public_url().is_none());
}

/// 测试 TailscaleTunnel 的 Funnel 模式
///
/// 当启用 Funnel 模式并指定主机名时，TailscaleTunnel 应正常创建，
/// name() 仍返回 "tailscale"。
#[test]
fn tailscale_funnel_mode() {
    let t = TailscaleTunnel::new(true, Some("myhost".into()));
    assert_eq!(t.name(), "tailscale");
}

/// 测试 NgrokTunnel 的基本属性（无自定义域名）
///
/// NgrokTunnel 创建后应：
/// - name() 返回 "ngrok"
/// - public_url() 返回 None（启动前无公共 URL）
#[test]
fn ngrok_tunnel_name() {
    let t = NgrokTunnel::new("tok".into(), None);
    assert_eq!(t.name(), "ngrok");
    assert!(t.public_url().is_none());
}

/// 测试 NgrokTunnel 使用自定义域名
///
/// 当指定自定义域名时，NgrokTunnel 应正常创建，
/// name() 仍返回 "ngrok"。
#[test]
fn ngrok_with_domain() {
    let t = NgrokTunnel::new("tok".into(), Some("my.ngrok.io".into()));
    assert_eq!(t.name(), "ngrok");
}

/// 测试 CustomTunnel 的基本属性
///
/// CustomTunnel 创建后应：
/// - name() 返回 "custom"
/// - public_url() 返回 None（启动前无公共 URL）
#[test]
fn custom_tunnel_name() {
    let t = CustomTunnel::new("echo hi".into(), None, None);
    assert_eq!(t.name(), "custom");
    assert!(t.public_url().is_none());
}

/// 测试 kill_shared 函数：无进程时可正常处理
///
/// 当共享进程中没有子进程时（即 Arc<Mutex<Option<TunnelProcess>>> 值为 None），
/// 调用 kill_shared 应返回 Ok(())，不会发生错误。
#[tokio::test]
async fn kill_shared_no_process_is_ok() {
    let proc = new_shared_process();
    let result = kill_shared(&proc).await;

    assert!(result.is_ok());
    assert!(proc.lock().await.is_none());
}

/// 测试 kill_shared 函数：正确终止并清理子进程
///
/// 当共享进程中存在正在运行的子进程时：
/// 1. 调用 kill_shared 应成功终止该进程
/// 2. 终止后共享进程应被清空（值变为 None）
///
/// 测试步骤：
/// - 创建一个新的共享进程句柄
/// - 启动一个 sleep 30 秒的子进程
/// - 将子进程存入共享进程
/// - 调用 kill_shared 终止进程
/// - 验证共享进程已被清空
#[tokio::test]
async fn kill_shared_terminates_and_clears_child() {
    let proc = new_shared_process();

    // 启动一个长时间运行的进程用于测试生命周期
    let child = Command::new("sleep")
        .arg("30")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("sleep should spawn for lifecycle test");

    // 将子进程存入共享进程句柄
    {
        let mut guard = proc.lock().await;
        *guard = Some(TunnelProcess { child, public_url: "https://example.test".into() });
    }

    // 终止进程并清理
    kill_shared(&proc).await.unwrap();

    // 验证进程已被清空
    let guard = proc.lock().await;
    assert!(guard.is_none());
}

/// 测试 CloudflareTunnel 的健康检查：启动前返回 false
///
/// CloudflareTunnel 在调用 start() 之前，
/// health_check() 应返回 false，因为隧道尚未建立。
#[tokio::test]
async fn cloudflare_health_false_before_start() {
    let tunnel = CloudflareTunnel::new("tok".into());
    assert!(!tunnel.health_check().await);
}

/// 测试 NgrokTunnel 的健康检查：启动前返回 false
///
/// NgrokTunnel 在调用 start() 之前，
/// health_check() 应返回 false，因为隧道尚未建立。
#[tokio::test]
async fn ngrok_health_false_before_start() {
    let tunnel = NgrokTunnel::new("tok".into(), None);
    assert!(!tunnel.health_check().await);
}

/// 测试 TailscaleTunnel 的健康检查：启动前返回 false
///
/// TailscaleTunnel 在调用 start() 之前，
/// health_check() 应返回 false，因为隧道尚未建立。
#[tokio::test]
async fn tailscale_health_false_before_start() {
    let tunnel = TailscaleTunnel::new(false, None);
    assert!(!tunnel.health_check().await);
}

/// 测试 CustomTunnel 的健康检查：无 health_url 时启动前返回 false
///
/// CustomTunnel 在没有配置 health_url 且未启动时，
/// health_check() 应返回 false。
/// 即使设置了 url_pattern，没有 health_url 也无法进行健康检查。
#[tokio::test]
async fn custom_health_false_before_start_without_health_url() {
    let tunnel = CustomTunnel::new("echo hi".into(), None, Some("https://".into()));
    assert!(!tunnel.health_check().await);
}
