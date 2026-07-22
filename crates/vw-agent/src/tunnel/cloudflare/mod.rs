//! Cloudflare 隧道模块
//!
//! 本模块提供基于 Cloudflare Tunnel (cloudflared) 的隧道实现。
//! 通过 Cloudflare 的隧道服务，可以将本地服务安全地暴露到公网，
//! 无需公网 IP 或端口转发配置。
//!
//! # 主要功能
//!
//! - 使用 cloudflared 命令行工具建立隧道连接
//! - 基于 token 的认证方式，简化配置流程
//! - 自动解析和提取公网访问 URL
//! - 支持健康检查和进程管理
//!
//! # 使用示例
//!
//! ```rust,no_run
//! use vibe_agent::tunnel::cloudflare::CloudflareTunnel;
//!
//! // 创建 Cloudflare 隧道实例
//! let tunnel = CloudflareTunnel::new("your-cloudflare-token".to_string());
//!
//! // 启动隧道并获取公网 URL
//! let public_url = tunnel.start("localhost", 8080).await?;
//! println!("公网访问地址: {}", public_url);
//! ```

use super::{SharedProcess, Tunnel, TunnelProcess, kill_shared, new_shared_process};
use crate::app::agent::shell::tokio_command;
use anyhow::{Result, bail};
use tokio::io::AsyncBufReadExt;

/// Cloudflare 隧道实现
///
/// 该结构体封装了 cloudflared 进程的管理逻辑，提供与 Cloudflare
/// 隧道服务的集成能力。通过持有认证 token 和进程句柄，
/// 实现隧道的生命周期管理。
///
/// # 字段说明
///
/// - `token`: Cloudflare 隧道的认证令牌，从 Cloudflare Zero Trust 仪表板获取
/// - `proc`: 共享的隧道进程句柄，用于管理 cloudflared 子进程
pub struct CloudflareTunnel {
    /// Cloudflare 隧道认证令牌
    pub token: String,
    /// 隧道进程句柄（共享状态）
    proc: SharedProcess,
}

impl CloudflareTunnel {
    /// 创建新的 Cloudflare 隧道实例
    ///
    /// # 参数
    ///
    /// - `token`: Cloudflare 隧道的认证令牌，格式通常为一长串 Base64 编码的字符串
    ///
    /// # 返回值
    ///
    /// 返回配置好但尚未启动的 CloudflareTunnel 实例
    ///
    /// # 示例
    ///
    /// ```rust
    /// use vibe_agent::tunnel::cloudflare::CloudflareTunnel;
    ///
    /// let tunnel = CloudflareTunnel::new("eyJhIjoyfQ==".to_string());
    /// ```
    pub fn new(token: String) -> Self {
        Self { token, proc: new_shared_process() }
    }
}

#[async_trait::async_trait]
impl Tunnel for CloudflareTunnel {
    /// 获取隧道名称标识
    ///
    /// 返回固定字符串 "cloudflare"，用于在日志和监控中标识隧道类型
    fn name(&self) -> &str {
        "cloudflare"
    }

    /// 启动 Cloudflare 隧道并建立公网连接
    ///
    /// 该方法会启动 cloudflared 子进程，建立与 Cloudflare 边缘网络的连接，
    /// 并从进程输出中解析出公网可访问的 URL。
    ///
    /// # 参数
    ///
    /// - `_local_host`: 本地主机地址（当前未使用，保留用于未来扩展）
    /// - `local_port`: 本地服务监听端口，隧道将转发流量到此端口
    ///
    /// # 返回值
    ///
    /// - `Ok(String)`: 成功时返回公网可访问的 HTTPS URL
    /// - `Err`: 启动失败或超时时返回错误信息
    ///
    /// # 错误情况
    ///
    /// - cloudflared 命令不存在或无法执行
    /// - 30 秒内未能解析出公网 URL（通常是 token 无效）
    /// - 读取进程输出时发生 I/O 错误
    ///
    /// # 超时机制
    ///
    /// 方法设置了 30 秒的总体超时限制，每 5 秒检查一次进程输出。
    /// 如果超时前未能获取到 URL，会自动终止子进程并返回错误。
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// let tunnel = CloudflareTunnel::new("token".to_string());
    /// match tunnel.start("localhost", 3000).await {
    ///     Ok(url) => println!("隧道已启动: {}", url),
    ///     Err(e) => eprintln!("启动失败: {}", e),
    /// }
    /// ```
    async fn start(&self, _local_host: &str, local_port: u16) -> Result<String> {
        // 启动 cloudflared 进程，配置参数：
        // - tunnel: 使用隧道模式
        // - --no-autoupdate: 禁用自动更新，避免运行时更新导致的不稳定
        // - run: 运行隧道
        // - --token: 使用 token 认证
        // - --url: 指定本地服务的转发地址
        let mut child = tokio_command(cloudflared_command_name())
            .args([
                "tunnel",
                "--no-autoupdate",
                "run",
                "--token",
                &self.token,
                "--url",
                &format!("http://localhost:{local_port}"),
            ])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true) // 当 Child 对象被 drop 时自动终止进程
            .spawn()?;

        // 获取 stderr 读取器，cloudflared 会在 stderr 中输出连接信息和公网 URL
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| anyhow::anyhow!("Failed to capture cloudflared stderr"))?;

        // 创建带缓冲的行读取器，逐行解析输出
        let mut reader = tokio::io::BufReader::new(stderr).lines();
        let mut public_url = String::new();

        // 设置 30 秒超时限制
        let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(30);

        // 在超时前持续读取进程输出，寻找公网 URL
        while tokio::time::Instant::now() < deadline {
            // 每次读取操作设置 5 秒超时
            let line =
                tokio::time::timeout(tokio::time::Duration::from_secs(5), reader.next_line()).await;

            match line {
                Ok(Ok(Some(l))) => {
                    // 记录所有输出行，便于调试
                    tracing::debug!("cloudflared: {l}");

                    // 在输出行中查找 "https://" 标记，定位公网 URL
                    if let Some(url) = extract_cloudflare_public_url(&l) {
                        public_url = url;
                        break; // 成功获取 URL，退出循环
                    }
                }
                Ok(Ok(None)) => break, // EOF，进程输出结束
                Ok(Err(e)) => bail!("Error reading cloudflared output: {e}"), // 读取错误
                Err(_) => {}           // 单次读取超时，继续尝试
            }
        }

        // 如果未能获取到公网 URL，终止进程并返回错误
        if public_url.is_empty() {
            child.kill().await.ok();
            bail!("cloudflared did not produce a public URL within 30s. Is the token valid?");
        }

        // 保存进程句柄和公网 URL 到共享状态
        let mut guard = self.proc.lock().await;
        *guard = Some(TunnelProcess { child, public_url: public_url.clone() });

        Ok(public_url)
    }

    /// 停止 Cloudflare 隧道
    ///
    /// 终止 cloudflared 子进程并释放相关资源。
    /// 该方法会安全地关闭进程连接并清理状态。
    ///
    /// # 返回值
    ///
    /// - `Ok(())`: 成功停止隧道
    /// - `Err`: 停止过程中发生错误
    async fn stop(&self) -> Result<()> {
        kill_shared(&self.proc).await
    }

    /// 检查隧道健康状态
    ///
    /// 通过检查 cloudflared 子进程是否仍在运行来判断隧道的健康状态。
    /// 如果进程已退出或不存在，则认为隧道不健康。
    ///
    /// # 返回值
    ///
    /// - `true`: 隧道进程正在运行，连接正常
    /// - `false`: 隧道进程已退出或未启动
    async fn health_check(&self) -> bool {
        let guard = self.proc.lock().await;
        // 检查进程是否存在且有有效的进程 ID
        guard.as_ref().is_some_and(|tp| tp.child.id().is_some())
    }

    /// 获取隧道的公网访问 URL
    ///
    /// 返回当前隧道对应的公网 HTTPS URL，该 URL 可用于从公网访问本地服务。
    ///
    /// # 返回值
    ///
    /// - `Some(String)`: 隧道已启动，返回公网 URL
    /// - `None`: 隧道未启动或无法获取锁
    ///
    /// # 注意事项
    ///
    /// 该方法使用 `try_lock()` 非阻塞方式获取锁，如果锁被占用则返回 None。
    /// 适用于需要快速查询而不希望阻塞的场景。
    fn public_url(&self) -> Option<String> {
        self.proc.try_lock().ok().and_then(|g| g.as_ref().map(|tp| tp.public_url.clone()))
    }
}

fn extract_cloudflare_public_url(line: &str) -> Option<String> {
    let idx = line.find("https://")?;
    let url_part = &line[idx..];
    let end = url_part.find(|c: char| c.is_whitespace()).unwrap_or(url_part.len());
    Some(url_part[..end].to_string())
}

fn cloudflared_command_name() -> &'static str {
    #[cfg(test)]
    {
        "vibewindow-cloudflared-missing-for-unit-tests"
    }
    #[cfg(not(test))]
    {
        "cloudflared"
    }
}

#[cfg(test)]
mod tests;
