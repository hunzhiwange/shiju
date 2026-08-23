//! 网关隧道提供方的统一抽象与工厂装配。
//!
//! 具体实现负责外部进程生命周期，调用方通过 `Tunnel` 统一启动、停止并获取公网地址。

mod cloudflare;
mod custom;
mod ngrok;
mod none;
mod tailscale;

pub use cloudflare::CloudflareTunnel;
pub use custom::CustomTunnel;
pub use ngrok::NgrokTunnel;
#[allow(unused_imports)]
pub use none::NoneTunnel;
pub use tailscale::TailscaleTunnel;

use crate::app::agent::config::{TailscaleTunnelConfig, TunnelConfig};
use anyhow::{Result, bail};
use std::sync::Arc;
use tokio::sync::Mutex;

// ── Tunnel trait ─────────────────────────────────────────────────

/// 与具体提供方无关的网关隧道抽象。
///
/// 实现可包装外部隧道程序或自定义命令；网关绑定本地端口后启动，关闭时停止。
#[async_trait::async_trait]
pub trait Tunnel: Send + Sync {
    /// 返回面向用户的稳定提供方名称。
    fn name(&self) -> &str;

    /// 启动隧道并暴露本地地址，成功时返回公网 URL。
    async fn start(&self, local_host: &str, local_port: u16) -> Result<String>;

    /// 尽力平滑停止隧道进程。
    async fn stop(&self) -> Result<()>;

    /// 检查隧道进程是否仍存活。
    async fn health_check(&self) -> bool;

    /// 隧道运行时返回当前公网 URL。
    fn public_url(&self) -> Option<String>;
}

// ── Shared child-process handle ──────────────────────────────────

/// 封装可由隧道实现共享的已启动子进程及公网地址。
pub(crate) struct TunnelProcess {
    pub child: tokio::process::Child,
    pub public_url: String,
}

/// 可在启动、健康检查与停止路径之间共享的进程槽。
pub(crate) type SharedProcess = Arc<Mutex<Option<TunnelProcess>>>;

/// 创建初始为空的共享进程槽。
pub(crate) fn new_shared_process() -> SharedProcess {
    Arc::new(Mutex::new(None))
}

/// 取出并终止当前共享隧道进程；没有进程时保持幂等成功。
pub(crate) async fn kill_shared(proc: &SharedProcess) -> Result<()> {
    let mut guard = proc.lock().await;
    if let Some(ref mut tp) = *guard {
        tp.child.kill().await.ok();
        tp.child.wait().await.ok();
    }
    *guard = None;
    Ok(())
}

// ── Factory ──────────────────────────────────────────────────────

/// 根据配置创建隧道实现；提供方为 `none` 或空值时返回 `None`。
pub fn create_tunnel(config: &TunnelConfig) -> Result<Option<Box<dyn Tunnel>>> {
    match config.provider.as_str() {
        "none" | "" => Ok(None),

        "cloudflare" => {
            let cf = config.cloudflare.as_ref().ok_or_else(|| {
                anyhow::anyhow!(
                    "tunnel.provider = \"cloudflare\" but [tunnel.cloudflare] section is missing"
                )
            })?;
            Ok(Some(Box::new(CloudflareTunnel::new(cf.token.clone()))))
        }

        "tailscale" => {
            let ts = config
                .tailscale
                .as_ref()
                .unwrap_or(&TailscaleTunnelConfig { funnel: false, hostname: None });
            Ok(Some(Box::new(TailscaleTunnel::new(ts.funnel, ts.hostname.clone()))))
        }

        "ngrok" => {
            let ng = config.ngrok.as_ref().ok_or_else(|| {
                anyhow::anyhow!("tunnel.provider = \"ngrok\" but [tunnel.ngrok] section is missing")
            })?;
            Ok(Some(Box::new(NgrokTunnel::new(ng.auth_token.clone(), ng.domain.clone()))))
        }

        "custom" => {
            let cu = config.custom.as_ref().ok_or_else(|| {
                anyhow::anyhow!(
                    "tunnel.provider = \"custom\" but [tunnel.custom] section is missing"
                )
            })?;
            let has_start_command = !cu.start_command.trim().is_empty();
            let has_url = cu.url.as_deref().is_some_and(|value| !value.trim().is_empty());
            let has_auth_token =
                cu.auth_token.as_deref().is_some_and(|value| !value.trim().is_empty());
            let has_health_url =
                cu.health_url.as_deref().is_some_and(|value| !value.trim().is_empty());
            let has_url_pattern =
                cu.url_pattern.as_deref().is_some_and(|value| !value.trim().is_empty());

            if !has_start_command
                && !has_url
                && !has_auth_token
                && !has_health_url
                && !has_url_pattern
            {
                return Ok(None);
            }

            Ok(Some(Box::new(CustomTunnel::new(
                cu.start_command.clone(),
                cu.health_url.clone(),
                cu.url_pattern.clone(),
            ))))
        }

        other => bail!(
            "Unknown tunnel provider: \"{other}\". Valid: none, cloudflare, tailscale, ngrok, custom"
        ),
    }
}

#[cfg(test)]
mod tests;
