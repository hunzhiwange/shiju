//! Ngrok 隧道模块
//!
//! 本模块提供基于 Ngrok 服务的隧道实现，用于将本地服务暴露到公网。
//! Ngrok 是一个流行的反向代理工具，可以快速创建公网可访问的 URL。
//!
//! # 主要功能
//!
//! - 配置并启动 Ngrok 进程
//! - 解析 Ngrok 输出以获取公网 URL
//! - 支持自定义域名绑定
//! - 提供进程生命周期管理（启动、停止、健康检查）
//!
//! # 使用示例
//!
//! ```ignore
//! use crate::app::agent::tunnel::ngrok::NgrokTunnel;
//!
//! let tunnel = NgrokTunnel::new("your-auth-token".to_string(), Some("custom.domain".to_string()));
//! let public_url = tunnel.start("localhost", 8080).await?;
//! println!("公网地址: {}", public_url);
//! tunnel.stop().await?;
//! ```

use super::{SharedProcess, Tunnel, TunnelProcess, kill_shared, new_shared_process};
use crate::app::agent::shell::tokio_command;
use anyhow::{Result, bail};
use tokio::io::AsyncBufReadExt;

/// Ngrok 隧道实现
///
/// 封装 Ngrok 进程的生命周期管理，提供认证、域名配置和进程控制能力。
/// 该结构体实现了 [`Tunnel`] trait，可与其他隧道实现互换使用。
///
/// # 字段说明
///
/// - `auth_token`: Ngrok 账户的认证令牌，用于身份验证
/// - `domain`: 可选的自定义域名，如未指定则使用 Ngrok 提供的随机域名
/// - `proc`: 共享进程句柄，用于存储和管理 Ngrok 子进程
pub struct NgrokTunnel {
    /// Ngrok 认证令牌
    auth_token: String,
    /// 自定义域名（可选）
    pub domain: Option<String>,
    /// 共享进程句柄，存储 Ngrok 子进程和公网 URL
    proc: SharedProcess,
}

impl NgrokTunnel {
    /// 创建新的 Ngrok 隧道实例
    ///
    /// # 参数
    ///
    /// - `auth_token`: Ngrok 账户的认证令牌，从 Ngrok 控制台获取
    /// - `domain`: 可选的自定义域名，需要先在 Ngrok 控制台中注册
    ///
    /// # 返回值
    ///
    /// 返回配置好的 `NgrokTunnel` 实例，尚未启动
    ///
    /// # 示例
    ///
    /// ```ignore
    /// // 使用随机域名
    /// let tunnel = NgrokTunnel::new("token".to_string(), None);
    ///
    /// // 使用自定义域名
    /// let tunnel = NgrokTunnel::new("token".to_string(), Some("my-app.ngrok.io".to_string()));
    /// ```
    pub fn new(auth_token: String, domain: Option<String>) -> Self {
        Self { auth_token, domain, proc: new_shared_process() }
    }
}

#[async_trait::async_trait]
impl Tunnel for NgrokTunnel {
    /// 返回隧道名称标识
    ///
    /// # 返回值
    ///
    /// 返回固定字符串 `"ngrok"`，用于标识隧道类型
    fn name(&self) -> &str {
        "ngrok"
    }

    /// 启动 Ngrok 隧道并获取公网 URL
    ///
    /// 该方法执行以下步骤：
    /// 1. 配置 Ngrok 认证令牌
    /// 2. 启动 Ngrok HTTP 隧道进程
    /// 3. 解析标准输出日志以提取公网 URL
    /// 4. 将进程信息存储到共享句柄中
    ///
    /// # 参数
    ///
    /// - `_local_host`: 本地主机地址（当前未使用，Ngrok 默认连接 localhost）
    /// - `local_port`: 本地服务端口号，Ngrok 将转发流量到此端口
    ///
    /// # 返回值
    ///
    /// - `Ok(String)`: 成功时返回公网可访问的 URL（如 `https://abc123.ngrok.io`）
    /// - `Err`: 启动失败或超时时返回错误
    ///
    /// # 错误情况
    ///
    /// - Ngrok 可执行文件未安装或不在 PATH 中
    /// - 认证令牌无效或已过期
    /// - 自定义域名未注册或不可用
    /// - 15 秒内未检测到公网 URL（超时）
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let url = tunnel.start("localhost", 8080).await?;
    /// println!("服务已暴露到: {}", url);
    /// ```
    async fn start(&self, _local_host: &str, local_port: u16) -> Result<String> {
        // 第一步：配置 Ngrok 认证令牌
        // 通过运行 `ngrok config add-authtoken` 命令将令牌写入配置文件
        tokio_command(ngrok_command_name())
            .args(["config", "add-authtoken", &self.auth_token])
            .output()
            .await?;

        // 第二步：构建 Ngrok 启动参数
        // 基础参数：http 模式 + 本地端口
        let mut args = vec!["http".to_string(), local_port.to_string()];

        // 如果指定了自定义域名，添加 --domain 参数
        if let Some(ref domain) = self.domain {
            args.push("--domain".into());
            args.push(domain.clone());
        }

        // 配置日志输出到标准输出，使用 logfmt 格式便于解析
        args.push("--log".into());
        args.push("stdout".into());
        args.push("--log-format".into());
        args.push("logfmt".into());

        // 第三步：启动 Ngrok 子进程
        // - 捕获标准输出和标准错误
        // - 设置 kill_on_drop 确保进程在句柄丢弃时被终止
        let mut child = tokio_command(ngrok_command_name())
            .args(&args)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true)
            .spawn()?;

        // 获取标准输出流的句柄
        let stdout =
            child.stdout.take().ok_or_else(|| anyhow::anyhow!("Failed to capture ngrok stdout"))?;

        // 创建带缓冲的行读取器
        let mut reader = tokio::io::BufReader::new(stdout).lines();
        let mut public_url = String::new();

        // 第四步：解析日志输出以提取公网 URL
        // 设置 15 秒超时，每 3 秒检查一次是否有新日志行
        let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(15);
        while tokio::time::Instant::now() < deadline {
            // 使用超时机制读取下一行，避免无限阻塞
            let line =
                tokio::time::timeout(tokio::time::Duration::from_secs(3), reader.next_line()).await;

            match line {
                Ok(Ok(Some(l))) => {
                    // 记录日志行用于调试
                    tracing::debug!("ngrok: {l}");

                    // 在日志行中查找 URL 标识
                    // Ngrok logfmt 格式中包含类似 "url=https://xxx.ngrok.io" 的字段
                    if let Some(url) = extract_ngrok_public_url(&l) {
                        public_url = url;
                        break; // 成功获取 URL，退出循环
                    }
                }
                Ok(Ok(None)) => break, // 流已结束
                Ok(Err(e)) => bail!("Error reading ngrok output: {e}"), // 读取错误
                Err(_) => {}           // 超时，继续等待
            }
        }

        // 第五步：验证是否成功获取公网 URL
        // 如果 15 秒内未获取到 URL，终止进程并返回错误
        if public_url.is_empty() {
            child.kill().await.ok();
            bail!("ngrok did not produce a public URL within 15s. Is the auth token valid?");
        }

        // 第六步：将进程信息和公网 URL 存储到共享句柄
        let mut guard = self.proc.lock().await;
        *guard = Some(TunnelProcess { child, public_url: public_url.clone() });

        Ok(public_url)
    }

    /// 停止 Ngrok 隧道进程
    ///
    /// 终止正在运行的 Ngrok 子进程并释放相关资源。
    /// 调用后公网 URL 将不再可用。
    ///
    /// # 返回值
    ///
    /// - `Ok(())`: 成功停止进程
    /// - `Err`: 停止过程中发生错误（如进程已终止）
    async fn stop(&self) -> Result<()> {
        kill_shared(&self.proc).await
    }

    /// 检查 Ngrok 隧道健康状态
    ///
    /// 通过检查子进程是否仍在运行来判断隧道是否健康。
    ///
    /// # 返回值
    ///
    /// - `true`: 进程正在运行，隧道健康
    /// - `false`: 进程已终止或未启动
    async fn health_check(&self) -> bool {
        let guard = self.proc.lock().await;
        guard.as_ref().is_some_and(|tp| tp.child.id().is_some())
    }

    /// 获取当前公网 URL
    ///
    /// 尝试非阻塞地获取已分配的公网 URL。
    ///
    /// # 返回值
    ///
    /// - `Some(String)`: 隧道已启动并分配了公网 URL
    /// - `None`: 隧道未启动或无法获取锁
    ///
    /// # 注意
    ///
    /// 该方法使用 `try_lock`，不会阻塞。如果锁被占用则返回 `None`。
    fn public_url(&self) -> Option<String> {
        self.proc.try_lock().ok().and_then(|g| g.as_ref().map(|tp| tp.public_url.clone()))
    }
}

fn extract_ngrok_public_url(line: &str) -> Option<String> {
    let idx = line.find("url=https://")?;
    let url_part = &line[idx + 4..];
    let end = url_part.find(|c: char| c.is_whitespace()).unwrap_or(url_part.len());
    Some(url_part[..end].to_string())
}

fn ngrok_command_name() -> &'static str {
    #[cfg(test)]
    {
        "vibewindow-ngrok-missing-for-unit-tests"
    }
    #[cfg(not(test))]
    {
        "ngrok"
    }
}

#[cfg(test)]
mod tests;
