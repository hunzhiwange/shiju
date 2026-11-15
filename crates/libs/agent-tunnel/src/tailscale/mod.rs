//! Tailscale 隧道模块
//!
//! 本模块提供基于 Tailscale 的隧道实现，用于通过 Tailscale 网络暴露本地服务。
//! 支持两种模式：
//! - **serve 模式**：仅在 Tailscale 网络内部暴露服务
//! - **funnel 模式**：通过 Tailscale 的 Funnel 功能将服务暴露到公网
//!
//! # 主要组件
//!
//! - [`TailscaleTunnel`]：Tailscale 隧道实现，封装了进程管理与 URL 生成逻辑
//!
//! # 使用示例
//!
//! ```ignore
//! use agent_tunnel::TailscaleTunnel;
//! use agent_tunnel::Tunnel;
//!
//! // 创建 funnel 模式隧道（公网访问）
//! let tunnel = TailscaleTunnel::new(true, Some("my-node".to_string()));
//!
//! // 启动隧道，将本地 8080 端口暴露出去
//! let public_url = tunnel.start("127.0.0.1", 8080).await?;
//! println!("服务已暴露于: {}", public_url);
//!
//! // 停止隧道
//! tunnel.stop().await?;
//! ```

use super::Tunnel;
use shell_command::tokio_command;
use anyhow::{Context, Result, bail};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Mutex;

struct TailscaleState {
    local_port: u16,
    public_url: String,
}

type SharedState = Arc<Mutex<Option<TailscaleState>>>;

fn new_shared_state() -> SharedState {
    Arc::new(Mutex::new(None))
}

fn normalize_dns_name(value: &str) -> &str {
    value.trim_end_matches('.')
}

fn host_and_port(host_port: &str) -> (&str, Option<u16>) {
    if let Some((host, port)) = host_port.rsplit_once(':') {
        if let Ok(port) = port.parse::<u16>() {
            return (host, Some(port));
        }
    }

    (host_port, None)
}

fn proxy_targets_local_port(proxy: &str, local_port: u16) -> bool {
    let authority = proxy.split_once("://").map_or(proxy, |(_, rest)| rest);
    let port = authority
        .rsplit(':')
        .next()
        .and_then(|segment| segment.split('/').next())
        .and_then(|segment| segment.parse::<u16>().ok());

    port == Some(local_port)
}

fn host_matches(host_port: &str, hostname: &str) -> bool {
    let normalized_host_port = normalize_dns_name(host_port);
    let (host, _) = host_and_port(normalized_host_port);
    normalize_dns_name(host) == normalize_dns_name(hostname)
}

fn host_port_to_https_url(host_port: &str) -> String {
    let normalized_host_port = normalize_dns_name(host_port);
    let (host, port) = host_and_port(normalized_host_port);

    match port {
        Some(443) | None => format!("https://{host}"),
        Some(port) => format!("https://{host}:{port}"),
    }
}

fn host_port_has_funnel_enabled(status: &Value, host_port: &str) -> bool {
    status.get("AllowFunnel").and_then(Value::as_object).is_some_and(|allow_funnel| {
        allow_funnel.iter().any(|(configured_host_port, enabled)| {
            enabled.as_bool() == Some(true)
                && normalize_dns_name(configured_host_port) == normalize_dns_name(host_port)
        })
    })
}

fn public_url_rank(url: &str, hostname: Option<&str>, funnel: bool) -> (u8, u16) {
    let host_port = url.trim_start_matches("https://").split('/').next().unwrap_or_default();
    let (host, port) = host_and_port(host_port);
    let host_rank = match hostname {
        Some(expected) if normalize_dns_name(host) == normalize_dns_name(expected) => 0,
        Some(_) => 1,
        None => 0,
    };
    let port_rank = match (funnel, port) {
        (true, Some(443) | None) => 0,
        (true, Some(8443)) => 1,
        (false, Some(8443)) => 0,
        (false, Some(443) | None) => 1,
        (_, Some(port)) => 2_u16.saturating_add(port),
    };

    (host_rank, port_rank)
}

fn public_urls_from_status(
    status: &Value,
    hostname: Option<&str>,
    local_port: u16,
    funnel: bool,
) -> Vec<String> {
    let mut urls = Vec::new();
    let Some(web) = status.get("Web").and_then(Value::as_object) else {
        return urls;
    };

    for (host_port, service) in web {
        if let Some(expected_hostname) = hostname {
            if !host_matches(host_port, expected_hostname) {
                continue;
            }
        }

        let Some(handlers) = service.get("Handlers").and_then(Value::as_object) else {
            continue;
        };
        let points_to_local_port = handlers.values().any(|handler| {
            handler
                .get("Proxy")
                .and_then(Value::as_str)
                .is_some_and(|proxy| proxy_targets_local_port(proxy, local_port))
        });

        if !points_to_local_port {
            continue;
        }

        let funnel_enabled = host_port_has_funnel_enabled(status, host_port);
        if funnel && !funnel_enabled {
            continue;
        }
        if !funnel && funnel_enabled {
            continue;
        }

        if points_to_local_port {
            urls.push(host_port_to_https_url(host_port));
        }
    }

    urls.sort_by_key(|url| public_url_rank(url, hostname, funnel));
    urls.dedup();
    urls
}

fn fallback_public_url(hostname: &str, funnel: bool) -> String {
    if funnel {
        format!("https://{}", normalize_dns_name(hostname))
    } else {
        format!("https://{}:8443", normalize_dns_name(hostname))
    }
}

async fn tailscale_status_json(args: &[&str]) -> Result<Value> {
    let output = tokio_command("tailscale").args(args).output().await?;
    if !output.status.success() {
        bail!("tailscale {} failed: {}", args.join(" "), String::from_utf8_lossy(&output.stderr));
    }

    serde_json::from_slice(&output.stdout).context("failed to parse tailscale status JSON")
}

fn status_hostname(status: &Value) -> Result<String> {
    status
        .get("Self")
        .and_then(|value| value.get("DNSName"))
        .and_then(Value::as_str)
        .map(normalize_dns_name)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .context("tailscale status missing Self.DNSName")
}

/// Tailscale 隧道实现
///
/// 封装了 Tailscale serve/funnel 功能的进程管理，提供隧道生命周期管理。
///
/// # 字段说明
///
/// - `funnel`：是否使用 funnel 模式（公网暴露），`false` 则使用 serve 模式（仅 Tailscale 网络内）
/// - `hostname`：可选的自定义主机名，若为 `None` 则自动从 `tailscale status` 获取
/// - `proc`：共享的隧道进程句柄，用于管理底层 tailscale 子进程
///
/// # 线程安全
///
/// 内部使用 `SharedProcess`（基于 `Arc<Mutex<Option<TunnelProcess>>>`），
/// 支持多任务并发访问同一隧道实例。
pub struct TailscaleTunnel {
    /// 是否启用 funnel 模式（公网暴露）
    pub funnel: bool,
    /// 自定义主机名，为 None 时自动检测
    pub hostname: Option<String>,
    /// 当前激活的 Tailscale 状态（Tailscale CLI 为配置命令，不是常驻子进程）
    state: SharedState,
}

impl TailscaleTunnel {
    /// 创建新的 Tailscale 隧道实例
    ///
    /// # 参数
    ///
    /// - `funnel`：是否使用 funnel 模式。
    ///   - `true`：使用 `tailscale funnel`，服务可被公网访问
    ///   - `false`：使用 `tailscale serve`，仅 Tailscale 网络内可访问
    /// - `hostname`：可选的自定义主机名。若为 `None`，启动时会自动调用
    ///   `tailscale status --json` 获取当前节点的 DNS 名称
    ///
    /// # 返回值
    ///
    /// 返回一个未启动的隧道实例，后续需调用 [`Tunnel::start`] 启动
    ///
    /// # 示例
    ///
    /// ```ignore
    /// // funnel 模式 + 自动主机名
    /// let tunnel = TailscaleTunnel::new(true, None);
    ///
    /// // serve 模式 + 自定义主机名
    /// let tunnel = TailscaleTunnel::new(false, Some("my-server".to_string()));
    /// ```
    pub fn new(funnel: bool, hostname: Option<String>) -> Self {
        Self { funnel, hostname, state: new_shared_state() }
    }

    async fn resolve_hostname(&self) -> Result<String> {
        if let Some(ref hostname) = self.hostname {
            return Ok(normalize_dns_name(hostname).to_string());
        }

        let status = tailscale_status_json(&["status", "--json"]).await?;
        status_hostname(&status)
    }
}

#[async_trait::async_trait]
impl Tunnel for TailscaleTunnel {
    /// 返回隧道名称标识
    ///
    /// # 返回值
    ///
    /// 固定返回 `"tailscale"`，用于日志和调试标识
    fn name(&self) -> &str {
        "tailscale"
    }

    /// 启动 Tailscale 隧道
    ///
    /// 根据配置的 `funnel` 标志，执行 `tailscale serve` 或 `tailscale funnel` 命令，
    /// 将指定的本地端口暴露出去。
    ///
    /// # 参数
    ///
    /// - `_local_host`：本地主机地址（当前未使用，Tailscale 会绑定所有接口）
    /// - `local_port`：要暴露的本地端口号
    ///
    /// # 返回值
    ///
    /// 成功时返回 Tailscale 实际暴露的 HTTPS URL。
    ///
    /// # 错误
    ///
    /// - 若 `tailscale status` 命令执行失败，返回错误
    /// - 若 `tailscale serve/funnel` 未成功配置，返回错误
    ///
    /// # 流程
    ///
    /// 1. 确定子命令（`funnel` 或 `serve`）
    /// 2. 获取或检测主机名
    ///    - 若已配置 `hostname`，直接使用
    ///    - 否则执行 `tailscale status --json` 并解析 `Self.DNSName` 字段
    /// 3. 执行 tailscale 配置命令
    /// 4. 从 `tailscale {serve|funnel} status --json` 解析真实 HTTPS 入口
    /// 5. 记录当前隧道状态
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let url = tunnel.start("127.0.0.1", 3000).await?;
    /// // url 可能是 "https://my-node.tailnet.ts.net:8443"
    /// ```
    async fn start(&self, _local_host: &str, local_port: u16) -> Result<String> {
        let subcommand = if self.funnel { "funnel" } else { "serve" };
        let hostname = self.resolve_hostname().await?;

        let local_port_arg = local_port.to_string();
        let output = tokio_command("tailscale")
            .args([subcommand, "--bg", local_port_arg.as_str()])
            .output()
            .await?;
        let status = tailscale_status_json(&[subcommand, "status", "--json"]).await.ok();
        let public_urls = status
            .as_ref()
            .map(|status| public_urls_from_status(status, Some(&hostname), local_port, self.funnel))
            .unwrap_or_default();

        if self.funnel && public_urls.is_empty() {
            bail!(
                "tailscale funnel did not report an internet-facing URL; current status is still tailnet-only"
            );
        }

        let public_url = public_urls
            .into_iter()
            .next()
            .unwrap_or_else(|| fallback_public_url(&hostname, self.funnel));

        if !output.status.success()
            && status.as_ref().is_none_or(|status| {
                public_urls_from_status(status, Some(&hostname), local_port, self.funnel).is_empty()
            })
        {
            bail!("tailscale {subcommand} failed: {}", String::from_utf8_lossy(&output.stderr));
        }

        let mut guard = self.state.lock().await;
        *guard = Some(TailscaleState { local_port, public_url: public_url.clone() });

        Ok(public_url)
    }

    /// 停止 Tailscale 隧道
    ///
    /// 执行清理操作：
    /// 1. 调用 `tailscale {serve|funnel} reset` 重置配置
    /// 2. 终止底层子进程
    ///
    /// # 返回值
    ///
    /// 成功返回 `Ok(())`，即使 reset 命令失败也会继续尝试终止进程
    ///
    /// # 错误
    ///
    /// 若进程终止失败，返回相应错误
    async fn stop(&self) -> Result<()> {
        let subcommand = if self.funnel { "funnel" } else { "serve" };
        tokio_command("tailscale").args([subcommand, "reset"]).output().await.ok();

        let mut guard = self.state.lock().await;
        *guard = None;
        Ok(())
    }

    /// 检查隧道健康状态
    ///
    /// 通过检查子进程是否仍在运行来判断隧道是否可用。
    ///
    /// # 返回值
    ///
    /// - `true`：子进程存在且仍在运行
    /// - `false`：子进程不存在或已退出
    async fn health_check(&self) -> bool {
        let Some((local_port, public_url)) = self
            .state
            .lock()
            .await
            .as_ref()
            .map(|state| (state.local_port, state.public_url.clone()))
        else {
            return false;
        };

        let subcommand = if self.funnel { "funnel" } else { "serve" };
        tailscale_status_json(&[subcommand, "status", "--json"]).await.ok().is_some_and(|status| {
            public_urls_from_status(&status, None, local_port, self.funnel)
                .into_iter()
                .any(|url| url == public_url)
        })
    }

    /// 获取当前公网 URL
    ///
    /// 尝试获取隧道启动时生成的公网访问地址。
    ///
    /// # 返回值
    ///
    /// - `Some(url)`：隧道已启动，返回公网 URL
    /// - `None`：隧道未启动或锁获取失败
    ///
    /// # 注意
    ///
    /// 使用 `try_lock` 非阻塞获取锁，避免在锁被持有时阻塞
    fn public_url(&self) -> Option<String> {
        self.state
            .try_lock()
            .ok()
            .and_then(|guard| guard.as_ref().map(|state| state.public_url.clone()))
    }
}

#[cfg(test)]
mod tests;
