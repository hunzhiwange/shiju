//! Tailscale 隧道模块的单元测试
//!
//! 本模块包含 `TailscaleTunnel` 结构体的测试用例，验证其构造函数、
//! 公共 URL 获取、健康检查和停止等功能的基本行为。
//!
//! # 测试覆盖范围
//!
//! - 构造函数参数存储验证
//! - 启动前的公共 URL 状态
//! - 启动前的健康检查状态
//! - 未启动时的停止操作安全性

use super::*;
use serde_json::json;

/// 测试构造函数是否正确存储 hostname 和 mode 参数
///
/// # 验证内容
///
/// - `funnel` 标志应被正确存储
/// - `hostname` 应被正确存储且可通过 `as_deref()` 访问
///
/// # 示例
///
/// ```
/// let tunnel = TailscaleTunnel::new(true, Some("myhost.tailnet.ts.net".into()));
/// assert!(tunnel.funnel);
/// assert_eq!(tunnel.hostname.as_deref(), Some("myhost.tailnet.ts.net"));
/// ```
#[test]
fn constructor_stores_hostname_and_mode() {
    // 创建一个启用 funnel 模式并指定 hostname 的隧道实例
    let tunnel = TailscaleTunnel::new(true, Some("myhost.tailnet.ts.net".into()));

    // 验证 funnel 标志已正确存储
    assert!(tunnel.funnel);

    // 验证 hostname 已正确存储，且可通过 as_deref() 获取原始字符串引用
    assert_eq!(tunnel.hostname.as_deref(), Some("myhost.tailnet.ts.net"));
}

/// 测试在调用 start() 之前，public_url() 应返回 None
///
/// # 验证内容
///
/// - 未启动的隧道实例，其公共 URL 应为 `None`
/// - 确保初始状态的一致性和可预测性
///
/// # 设计意图
///
/// 此测试确保在隧道未启动时，调用者无法获取有效的公共 URL，
/// 从而避免使用未初始化的连接信息。
#[test]
fn public_url_is_none_before_start() {
    // 创建一个未启用 funnel 且未指定 hostname 的隧道实例
    let tunnel = TailscaleTunnel::new(false, None);

    // 验证在启动前，公共 URL 应为 None
    assert!(tunnel.public_url().is_none());
}

/// 测试在调用 start() 之前，health_check() 应返回 false
///
/// # 验证内容
///
/// - 未启动的隧道实例，其健康检查应返回 `false`
/// - 确保健康检查能够正确反映隧道的实际运行状态
///
/// # 异步说明
///
/// 此测试使用 `#[tokio::test]` 标记，因为 `health_check()` 是异步方法。
///
/// # 设计意图
///
/// 健康检查应在隧道未启动时返回 false，以便调用者能够识别服务不可用状态，
/// 而不是抛出错误或返回不确定的结果。
#[tokio::test]
async fn health_check_is_false_before_start() {
    // 创建一个未启用 funnel 且未指定 hostname 的隧道实例
    let tunnel = TailscaleTunnel::new(false, None);

    // 验证在启动前，健康检查应返回 false
    assert!(!tunnel.health_check().await);
}

/// 测试在未启动进程的情况下调用 stop() 应安全返回 Ok(())
///
/// # 验证内容
///
/// - 对未启动的隧道调用 `stop()` 不应导致错误
/// - 返回值应为 `Ok(())`，表示操作成功完成
///
/// # 异步说明
///
/// 此测试使用 `#[tokio::test]` 标记，因为 `stop()` 是异步方法。
///
/// # 设计意图
///
/// 此测试确保 `stop()` 方法是幂等的且安全的：
/// - 即使在没有运行中的进程时调用，也不应引发 panic 或错误
/// - 允许调用者在不检查启动状态的情况下安全地停止隧道
/// - 符合"优雅降级"的设计原则
#[tokio::test]
async fn stop_without_started_process_is_ok() {
    // 创建一个未启用 funnel 且未指定 hostname 的隧道实例
    let tunnel = TailscaleTunnel::new(false, None);

    // 尝试停止未启动的隧道，应安全返回 Ok
    let result = tunnel.stop().await;

    // 验证停止操作成功完成，即使没有运行中的进程
    assert!(result.is_ok());
}

#[test]
fn public_urls_from_status_prefers_funnel_enabled_entry() {
    let status = json!({
        "Web": {
            "xiangminmac-mini.tail47e3db.ts.net:443": {
                "Handlers": {
                    "/": {
                        "Proxy": "http://127.0.0.1:42617"
                    }
                }
            },
            "xiangminmac-mini.tail47e3db.ts.net:8443": {
                "Handlers": {
                    "/": {
                        "Proxy": "http://127.0.0.1:42617"
                    }
                }
            },
            "other-node.tail47e3db.ts.net:8443": {
                "Handlers": {
                    "/": {
                        "Proxy": "http://127.0.0.1:42617"
                    }
                }
            }
        },
        "AllowFunnel": {
            "xiangminmac-mini.tail47e3db.ts.net:443": true
        }
    });

    let urls =
        public_urls_from_status(&status, Some("xiangminmac-mini.tail47e3db.ts.net"), 42617, true);

    assert_eq!(urls, vec!["https://xiangminmac-mini.tail47e3db.ts.net".to_string()]);
}

#[test]
fn public_urls_from_status_prefers_tailnet_endpoint_for_serve() {
    let status = json!({
        "Web": {
            "xiangminmac-mini.tail47e3db.ts.net:443": {
                "Handlers": {
                    "/": {
                        "Proxy": "http://127.0.0.1:42617"
                    }
                }
            },
            "xiangminmac-mini.tail47e3db.ts.net:8443": {
                "Handlers": {
                    "/": {
                        "Proxy": "http://127.0.0.1:42617"
                    }
                }
            }
        },
        "AllowFunnel": {
            "xiangminmac-mini.tail47e3db.ts.net:443": true
        }
    });

    let urls =
        public_urls_from_status(&status, Some("xiangminmac-mini.tail47e3db.ts.net"), 42617, false);

    assert_eq!(urls, vec!["https://xiangminmac-mini.tail47e3db.ts.net:8443".to_string()]);
}

#[test]
fn public_urls_from_status_filters_out_other_backend_ports() {
    let status = json!({
        "Web": {
            "xiangminmac-mini.tail47e3db.ts.net:8443": {
                "Handlers": {
                    "/": {
                        "Proxy": "http://127.0.0.1:3000"
                    }
                }
            }
        }
    });

    let urls =
        public_urls_from_status(&status, Some("xiangminmac-mini.tail47e3db.ts.net"), 42617, false);

    assert!(urls.is_empty());
}

#[test]
fn public_urls_from_status_without_hostname_accepts_matching_backends() {
    let status = json!({
        "Web": {
            "alpha.tailnet.ts.net:8443": {
                "Handlers": {
                    "/": { "Proxy": "http://127.0.0.1:42617" }
                }
            },
            "beta.tailnet.ts.net:8443": {
                "Handlers": {
                    "/": { "Proxy": "http://127.0.0.1:42617" }
                }
            }
        }
    });

    assert_eq!(
        public_urls_from_status(&status, None, 42617, false),
        vec![
            "https://alpha.tailnet.ts.net:8443".to_string(),
            "https://beta.tailnet.ts.net:8443".to_string(),
        ]
    );
}

#[test]
fn fallback_public_url_uses_expected_frontend_port() {
    assert_eq!(
        fallback_public_url("xiangminmac-mini.tail47e3db.ts.net.", false),
        "https://xiangminmac-mini.tail47e3db.ts.net:8443"
    );
    assert_eq!(
        fallback_public_url("xiangminmac-mini.tail47e3db.ts.net.", true),
        "https://xiangminmac-mini.tail47e3db.ts.net"
    );
}

#[test]
fn normalize_dns_name_removes_all_trailing_dots() {
    assert_eq!(normalize_dns_name("node.tailnet.ts.net..."), "node.tailnet.ts.net");
    assert_eq!(normalize_dns_name("node.tailnet.ts.net"), "node.tailnet.ts.net");
    assert_eq!(normalize_dns_name("..."), "");
}

#[test]
fn host_and_port_parses_only_valid_u16_suffixes() {
    assert_eq!(host_and_port("node.tailnet.ts.net:8443"), ("node.tailnet.ts.net", Some(8443)));
    assert_eq!(host_and_port("node.tailnet.ts.net:0"), ("node.tailnet.ts.net", Some(0)));
    assert_eq!(host_and_port("node.tailnet.ts.net"), ("node.tailnet.ts.net", None));
    assert_eq!(
        host_and_port("node.tailnet.ts.net:not-a-port"),
        ("node.tailnet.ts.net:not-a-port", None)
    );
    assert_eq!(host_and_port("node.tailnet.ts.net:65536"), ("node.tailnet.ts.net:65536", None));
}

#[test]
fn proxy_targets_local_port_accepts_scheme_and_path_variants() {
    assert!(proxy_targets_local_port("http://127.0.0.1:42617", 42617));
    assert!(proxy_targets_local_port("https://localhost:42617/api", 42617));
    assert!(proxy_targets_local_port("localhost:42617/health", 42617));
    assert!(!proxy_targets_local_port("http://127.0.0.1:42618", 42617));
    assert!(!proxy_targets_local_port("http://127.0.0.1:not-a-port", 42617));
    assert!(!proxy_targets_local_port("http://127.0.0.1", 42617));
}

#[test]
fn host_matching_ignores_frontend_port_and_trailing_dot() {
    assert!(host_matches("node.tailnet.ts.net:8443.", "node.tailnet.ts.net."));
    assert!(host_matches("node.tailnet.ts.net.", "node.tailnet.ts.net"));
    assert!(!host_matches("other.tailnet.ts.net:8443", "node.tailnet.ts.net"));
}

#[test]
fn host_port_to_https_url_omits_default_port_only() {
    assert_eq!(host_port_to_https_url("node.tailnet.ts.net"), "https://node.tailnet.ts.net");
    assert_eq!(host_port_to_https_url("node.tailnet.ts.net:443."), "https://node.tailnet.ts.net");
    assert_eq!(
        host_port_to_https_url("node.tailnet.ts.net:8443."),
        "https://node.tailnet.ts.net:8443"
    );
}

#[test]
fn funnel_enabled_requires_matching_true_boolean() {
    let status = json!({
        "AllowFunnel": {
            "node.tailnet.ts.net:443.": true,
            "node.tailnet.ts.net:8443": false,
            "node.tailnet.ts.net:9443": "true"
        }
    });

    assert!(host_port_has_funnel_enabled(&status, "node.tailnet.ts.net:443"));
    assert!(!host_port_has_funnel_enabled(&status, "node.tailnet.ts.net:8443"));
    assert!(!host_port_has_funnel_enabled(&status, "node.tailnet.ts.net:9443"));
    assert!(!host_port_has_funnel_enabled(&json!({}), "node.tailnet.ts.net:443"));
}

#[test]
fn public_url_rank_prioritizes_hostname_then_mode_port() {
    assert!(
        public_url_rank("https://node.tailnet.ts.net:9443", Some("node.tailnet.ts.net"), true)
            < public_url_rank("https://other.tailnet.ts.net", Some("node.tailnet.ts.net"), true)
    );
    assert!(
        public_url_rank("https://node.tailnet.ts.net", None, true)
            < public_url_rank("https://node.tailnet.ts.net:8443", None, true)
    );
    assert!(
        public_url_rank("https://node.tailnet.ts.net:8443", None, false)
            < public_url_rank("https://node.tailnet.ts.net", None, false)
    );
}

#[test]
fn public_urls_from_status_rejects_missing_or_malformed_web_entries() {
    assert!(public_urls_from_status(&json!({}), None, 42617, false).is_empty());

    let status = json!({
        "Web": {
            "wrong.tailnet.ts.net:8443": {
                "Handlers": {
                    "/": { "Proxy": "http://127.0.0.1:42617" }
                }
            },
            "node.tailnet.ts.net:8443": {},
            "node.tailnet.ts.net:9443": {
                "Handlers": {
                    "/": { "Proxy": 42617 }
                }
            }
        }
    });

    assert!(public_urls_from_status(&status, Some("node.tailnet.ts.net"), 42617, false).is_empty());
}

#[test]
fn public_urls_from_status_sorts_and_deduplicates_serve_urls() {
    let status = json!({
        "Web": {
            "node.tailnet.ts.net:9443": {
                "Handlers": {
                    "/": { "Proxy": "http://127.0.0.1:42617" }
                }
            },
            "node.tailnet.ts.net:443": {
                "Handlers": {
                    "/": { "Proxy": "127.0.0.1:42617/path" }
                }
            },
            "node.tailnet.ts.net:8443.": {
                "Handlers": {
                    "/": { "Proxy": "http://localhost:42617" }
                }
            },
            "node.tailnet.ts.net:8443": {
                "Handlers": {
                    "/": { "Proxy": "http://127.0.0.1:42617" }
                }
            }
        }
    });

    assert_eq!(
        public_urls_from_status(&status, Some("node.tailnet.ts.net"), 42617, false),
        vec![
            "https://node.tailnet.ts.net:8443".to_string(),
            "https://node.tailnet.ts.net".to_string(),
            "https://node.tailnet.ts.net:9443".to_string(),
        ]
    );
}

#[test]
fn public_urls_from_status_sorts_funnel_frontend_ports() {
    let status = json!({
        "Web": {
            "node.tailnet.ts.net:9443": {
                "Handlers": {
                    "/": { "Proxy": "http://127.0.0.1:42617" }
                }
            },
            "node.tailnet.ts.net:8443": {
                "Handlers": {
                    "/": { "Proxy": "http://127.0.0.1:42617" }
                }
            },
            "node.tailnet.ts.net:443": {
                "Handlers": {
                    "/": { "Proxy": "http://127.0.0.1:42617" }
                }
            }
        },
        "AllowFunnel": {
            "node.tailnet.ts.net:9443": true,
            "node.tailnet.ts.net:8443": true,
            "node.tailnet.ts.net:443": true
        }
    });

    assert_eq!(
        public_urls_from_status(&status, Some("node.tailnet.ts.net"), 42617, true),
        vec![
            "https://node.tailnet.ts.net".to_string(),
            "https://node.tailnet.ts.net:8443".to_string(),
            "https://node.tailnet.ts.net:9443".to_string(),
        ]
    );
}

#[test]
fn status_hostname_normalizes_valid_dns_name() {
    let status = json!({ "Self": { "DNSName": "node.tailnet.ts.net." } });

    assert_eq!(status_hostname(&status).unwrap(), "node.tailnet.ts.net");
}

#[test]
fn status_hostname_rejects_missing_or_empty_dns_name() {
    for status in [
        json!({}),
        json!({ "Self": {} }),
        json!({ "Self": { "DNSName": 42 } }),
        json!({ "Self": { "DNSName": "..." } }),
    ] {
        assert_eq!(
            status_hostname(&status).unwrap_err().to_string(),
            "tailscale status missing Self.DNSName"
        );
    }
}

#[tokio::test]
async fn resolve_hostname_uses_configured_normalized_name_without_status_lookup() {
    let tunnel = TailscaleTunnel::new(false, Some("node.tailnet.ts.net.".to_string()));

    assert_eq!(tunnel.resolve_hostname().await.unwrap(), "node.tailnet.ts.net");
}

#[test]
fn public_url_returns_stored_state_url() {
    let tunnel = TailscaleTunnel::new(false, None);
    let mut state = tunnel.state.try_lock().unwrap();
    *state = Some(TailscaleState {
        local_port: 42617,
        public_url: "https://node.tailnet.ts.net:8443".to_string(),
    });
    drop(state);

    assert_eq!(tunnel.public_url().as_deref(), Some("https://node.tailnet.ts.net:8443"));
}

#[test]
fn public_url_returns_none_while_state_is_locked() {
    let tunnel = TailscaleTunnel::new(false, None);
    let _state = tunnel.state.try_lock().unwrap();

    assert!(tunnel.public_url().is_none());
}

#[cfg(unix)]
#[tokio::test]
async fn start_with_fake_tailscale_uses_status_url_and_health_check_matches_state() {
    let _lock = tailscale_env_lock();
    let bin = tempfile::tempdir().expect("fake tailscale bin dir");
    write_fake_tailscale(bin.path());
    let _path = EnvGuard::set_os("PATH", prepend_path(bin.path()).as_os_str());
    let _mode = EnvGuard::set("TAILSCALE_FAKE_MODE", "success");
    let tunnel = TailscaleTunnel::new(false, None);

    let url = tunnel.start("127.0.0.1", 42617).await.expect("fake serve starts");

    assert_eq!(url, "https://node.tailnet.ts.net:8443");
    assert_eq!(tunnel.public_url().as_deref(), Some("https://node.tailnet.ts.net:8443"));
    assert!(tunnel.health_check().await);
}

#[cfg(unix)]
#[tokio::test]
async fn start_with_fake_tailscale_rejects_tailnet_only_funnel_status() {
    let _lock = tailscale_env_lock();
    let bin = tempfile::tempdir().expect("fake tailscale bin dir");
    write_fake_tailscale(bin.path());
    let _path = EnvGuard::set_os("PATH", prepend_path(bin.path()).as_os_str());
    let _mode = EnvGuard::set("TAILSCALE_FAKE_MODE", "tailnet_only");
    let tunnel = TailscaleTunnel::new(true, Some("node.tailnet.ts.net.".to_string()));

    let error = tunnel
        .start("127.0.0.1", 42617)
        .await
        .expect_err("tailnet-only status is rejected for funnel");

    assert!(error.to_string().contains("did not report an internet-facing URL"));
    assert!(tunnel.public_url().is_none());
}

#[cfg(unix)]
#[tokio::test]
async fn start_with_fake_tailscale_reports_command_failure_without_status_url() {
    let _lock = tailscale_env_lock();
    let bin = tempfile::tempdir().expect("fake tailscale bin dir");
    write_fake_tailscale(bin.path());
    let _path = EnvGuard::set_os("PATH", prepend_path(bin.path()).as_os_str());
    let _mode = EnvGuard::set("TAILSCALE_FAKE_MODE", "fail");
    let tunnel = TailscaleTunnel::new(false, Some("node.tailnet.ts.net".to_string()));

    let error =
        tunnel.start("127.0.0.1", 42617).await.expect_err("failed serve command is reported");

    assert!(error.to_string().contains("tailscale serve failed: fake serve failed"));
    assert!(tunnel.public_url().is_none());
}

#[cfg(unix)]
#[tokio::test]
async fn tailscale_status_json_reports_cli_failure_and_parse_errors() {
    let _lock = tailscale_env_lock();
    let bin = tempfile::tempdir().expect("fake tailscale bin dir");
    write_fake_tailscale(bin.path());
    let _path = EnvGuard::set_os("PATH", prepend_path(bin.path()).as_os_str());

    let _mode = EnvGuard::set("TAILSCALE_FAKE_MODE", "status_fail");
    let error =
        tailscale_status_json(&["status", "--json"]).await.expect_err("nonzero status should fail");
    assert!(error.to_string().contains("tailscale status --json failed: fake status failed"));
    drop(_mode);

    let _mode = EnvGuard::set("TAILSCALE_FAKE_MODE", "invalid_json");
    let error =
        tailscale_status_json(&["status", "--json"]).await.expect_err("invalid JSON should fail");
    assert!(error.to_string().contains("failed to parse tailscale status JSON"));
}

#[cfg(unix)]
fn tailscale_env_lock() -> std::sync::MutexGuard<'static, ()> {
    crate::test_support::global_env_lock().lock().unwrap_or_else(|e| e.into_inner())
}

#[cfg(unix)]
struct EnvGuard {
    key: &'static str,
    original: Option<std::ffi::OsString>,
}

#[cfg(unix)]
impl EnvGuard {
    fn set(key: &'static str, value: &str) -> Self {
        Self::set_os(key, std::ffi::OsStr::new(value))
    }

    fn set_os(key: &'static str, value: &std::ffi::OsStr) -> Self {
        let original = std::env::var_os(key);
        unsafe { std::env::set_var(key, value) };
        Self { key, original }
    }
}

#[cfg(unix)]
impl Drop for EnvGuard {
    fn drop(&mut self) {
        match &self.original {
            Some(value) => unsafe { std::env::set_var(self.key, value) },
            None => unsafe { std::env::remove_var(self.key) },
        }
    }
}

#[cfg(unix)]
fn prepend_path(dir: &std::path::Path) -> std::ffi::OsString {
    let mut paths = vec![dir.to_path_buf()];
    if let Some(existing) = std::env::var_os("PATH") {
        paths.extend(std::env::split_paths(&existing));
    }
    std::env::join_paths(paths).expect("join fake PATH")
}

#[cfg(unix)]
fn write_fake_tailscale(dir: &std::path::Path) {
    use std::os::unix::fs::PermissionsExt;

    let path = dir.join("tailscale");
    std::fs::write(
        &path,
        r#"#!/bin/sh
mode="${TAILSCALE_FAKE_MODE:-success}"

if [ "$mode" = "status_fail" ] && [ "$1" = "status" ] && [ "$2" = "--json" ]; then
  echo "fake status failed" >&2
  exit 23
fi

if [ "$mode" = "invalid_json" ] && [ "$1" = "status" ] && [ "$2" = "--json" ]; then
  printf 'not json'
  exit 0
fi

if [ "$1" = "status" ] && [ "$2" = "--json" ]; then
  printf '{"Self":{"DNSName":"node.tailnet.ts.net."}}'
  exit 0
fi

if [ "$1" = "serve" ] && [ "$2" = "--bg" ] && [ "$mode" = "fail" ]; then
  echo "fake serve failed" >&2
  exit 42
fi

if [ "$2" = "status" ] && [ "$3" = "--json" ]; then
  case "$mode" in
    fail)
      printf '{"Web":{}}'
      ;;
    tailnet_only)
      printf '{"Web":{"node.tailnet.ts.net:8443":{"Handlers":{"/":{"Proxy":"http://127.0.0.1:42617"}}}}}'
      ;;
    *)
      printf '{"Web":{"node.tailnet.ts.net:443":{"Handlers":{"/":{"Proxy":"http://127.0.0.1:42617"}}},"node.tailnet.ts.net:8443":{"Handlers":{"/":{"Proxy":"http://127.0.0.1:42617"}}}},"AllowFunnel":{"node.tailnet.ts.net:443":true}}'
      ;;
  esac
  exit 0
fi

exit 0
"#,
    )
    .expect("write fake tailscale");
    let mut perms = std::fs::metadata(&path).expect("fake tailscale metadata").permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&path, perms).expect("fake tailscale executable");
}
