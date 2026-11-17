//! 自定义隧道模块
//!
//! 本模块提供用户自定义隧道实现，允许通过任意命令行工具创建网络隧道。
//! 用户可以指定启动命令、健康检查 URL 和公网地址匹配模式，
//! 从而灵活地集成任何第三方隧道服务（如 ngrok、frp、cloudflared 等）。
//!
//! # 核心功能
//!
//! - **自定义启动命令**：支持 `{port}` 和 `{host}` 占位符，自动替换为本地监听地址
//! - **公网地址提取**：从命令输出中解析公网访问地址
//! - **健康检查**：可选的健康检查 URL，用于验证隧道可用性
//!
//! # 示例
//!
//! ```ignore
//! use agent_tunnel::CustomTunnel;
//!
//! // 创建一个使用 ngrok 的自定义隧道
//! let tunnel = CustomTunnel::new(
//!     "ngrok http {port}".to_string(),
//!     Some("http://localhost:4040/api/tunnels".to_string()),
//!     Some("https://".to_string()),
//! );
//! ```

use super::{SharedProcess, Tunnel, TunnelProcess, kill_shared, new_shared_process};
use anyhow::{Result, bail};
use tokio::io::AsyncBufReadExt;
use tokio::process::Command;

/// 自定义隧道实现
///
/// 通过执行用户指定的命令来启动隧道服务。该结构体封装了隧道的启动、
/// 停止、健康检查以及公网地址获取等功能。
///
/// # 字段说明
///
/// - `start_command`：启动隧道的命令，支持 `{port}` 和 `{host}` 占位符
/// - `health_url`：可选的健康检查 URL，用于验证隧道是否正常运行
/// - `url_pattern`：可选的 URL 匹配模式，用于从命令输出中提取公网地址
/// - `proc`：共享的隧道进程句柄，用于管理子进程生命周期
pub struct CustomTunnel {
    /// 启动隧道的命令字符串
    /// 占位符 `{port}` 会被替换为本地端口，`{host}` 会被替换为本地主机地址
    start_command: String,
    /// 健康检查 URL（可选）
    /// 如果设置，将通过 HTTP GET 请求验证隧道的可用性
    health_url: Option<String>,
    /// 公网地址匹配模式（可选）
    /// 用于从隧道命令的标准输出中识别包含公网地址的行
    url_pattern: Option<String>,
    /// 共享的隧道进程句柄
    /// 使用 Arc<Mutex> 包装，支持跨异步任务共享和并发访问
    proc: SharedProcess,
}

impl CustomTunnel {
    /// 创建一个新的自定义隧道实例
    ///
    /// # 参数
    ///
    /// - `start_command`：启动隧道的命令，例如 `"ngrok http {port}"`
    ///   - `{port}` 会被替换为实际的本地端口号
    ///   - `{host}` 会被替换为本地主机地址
    /// - `health_url`：可选的健康检查 URL，用于验证隧道服务是否可用
    /// - `url_pattern`：可选的匹配模式，用于从命令输出中提取公网地址
    ///
    /// # 返回值
    ///
    /// 返回初始化后的 `CustomTunnel` 实例，此时隧道尚未启动
    ///
    /// # 示例
    ///
    /// ```ignore
    /// // 创建使用 cloudflared 的隧道
    /// let tunnel = CustomTunnel::new(
    ///     "cloudflared tunnel --url http://localhost:{port}".to_string(),
    ///     None,
    ///     Some("https://".to_string()),
    /// );
    /// ```
    pub fn new(
        start_command: String,
        health_url: Option<String>,
        url_pattern: Option<String>,
    ) -> Self {
        Self { start_command, health_url, url_pattern, proc: new_shared_process() }
    }
}

/// 为 CustomTunnel 实现 Tunnel trait
///
/// 提供隧道的标准生命周期管理接口，包括启动、停止、健康检查和公网地址获取。
#[async_trait::async_trait]
impl Tunnel for CustomTunnel {
    /// 获取隧道类型的名称标识
    ///
    /// # 返回值
    ///
    /// 返回固定字符串 `"custom"`，用于标识这是一个用户自定义的隧道实现
    fn name(&self) -> &str {
        "custom"
    }

    /// 启动自定义隧道
    ///
    /// 执行用户配置的启动命令，并尝试从命令输出中提取公网访问地址。
    /// 该方法会替换命令中的 `{port}` 和 `{host}` 占位符，然后以子进程方式执行命令。
    ///
    /// # 参数
    ///
    /// - `local_host`：本地主机地址，用于替换命令中的 `{host}` 占位符
    /// - `local_port`：本地端口号，用于替换命令中的 `{port}` 占位符
    ///
    /// # 返回值
    ///
    /// - `Ok(String)`：成功启动后返回公网访问地址
    /// - `Err`：启动失败时返回错误信息，可能的原因包括：
    ///   - 启动命令为空
    ///   - 子进程启动失败
    ///
    /// # 启动流程
    ///
    /// 1. 替换命令中的占位符 `{port}` 和 `{host}`
    /// 2. 解析命令字符串为程序名和参数列表
    /// 3. 启动子进程，捕获标准输出和标准错误
    /// 4. 如果配置了 `url_pattern`，从标准输出中解析公网地址
    /// 5. 将子进程句柄和公网地址保存到共享状态中
    ///
    /// # 超时机制
    ///
    /// URL 解析阶段设置 15 秒总超时，每次读取行设置 3 秒超时，
    /// 避免因隧道命令输出异常而无限等待。
    async fn start(&self, local_host: &str, local_port: u16) -> Result<String> {
        // 替换命令中的占位符：{port} -> 实际端口号，{host} -> 本地主机地址
        let cmd = self
            .start_command
            .replace("{port}", &local_port.to_string())
            .replace("{host}", local_host);

        // 将命令字符串按空白字符分割为程序名和参数列表
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        if parts.is_empty() {
            bail!("Custom tunnel start_command is empty");
        }

        // 启动子进程执行隧道命令
        // 配置：捕获 stdout/stderr、进程被丢弃时自动终止
        let mut child = Command::new(parts[0])
            .args(&parts[1..])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true)
            .spawn()?;

        // 默认公网地址为本地地址（当无法从输出中解析时使用）
        let mut public_url = format!("http://{local_host}:{local_port}");

        // 如果配置了 URL 匹配模式，尝试从子进程标准输出中提取公网地址
        if let Some(ref pattern) = self.url_pattern
            && let Some(stdout) = child.stdout.take()
        {
            let mut reader = tokio::io::BufReader::new(stdout).lines();
            // 设置 15 秒的总超时时间
            let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(15);

            // 在超时时间内持续读取输出行，寻找公网地址
            while tokio::time::Instant::now() < deadline {
                // 每次读取行设置 3 秒超时，防止单次读取阻塞过久
                let line =
                    tokio::time::timeout(tokio::time::Duration::from_secs(3), reader.next_line())
                        .await;

                match line {
                    Ok(Ok(Some(l))) => {
                        tracing::debug!("custom-tunnel: {l}");
                        // 检查当前行是否包含匹配模式或 HTTP(S) URL 标识
                        if l.contains(pattern) || l.contains("https://") || l.contains("http://") {
                            // 尝试提取 HTTPS URL
                            if let Some(idx) = l.find("https://") {
                                let url_part = &l[idx..];
                                // 找到 URL 结束位置（以空白字符为界）
                                let end = url_part
                                    .find(|c: char| c.is_whitespace())
                                    .unwrap_or(url_part.len());
                                public_url = url_part[..end].to_string();
                                break;
                            // 尝试提取 HTTP URL
                            } else if let Some(idx) = l.find("http://") {
                                let url_part = &l[idx..];
                                let end = url_part
                                    .find(|c: char| c.is_whitespace())
                                    .unwrap_or(url_part.len());
                                public_url = url_part[..end].to_string();
                                break;
                            }
                        }
                    }
                    // EOF 或读取错误，退出循环
                    Ok(Ok(None) | Err(_)) => break,
                    // 超时继续尝试
                    Err(_) => {}
                }
            }
        }

        // 将子进程句柄和公网地址保存到共享状态
        let mut guard = self.proc.lock().await;
        *guard = Some(TunnelProcess { child, public_url: public_url.clone() });

        Ok(public_url)
    }

    /// 停止自定义隧道
    ///
    /// 终止隧道子进程并清理相关资源。
    ///
    /// # 返回值
    ///
    /// - `Ok(())`：成功停止隧道
    /// - `Err`：停止过程中发生错误
    async fn stop(&self) -> Result<()> {
        kill_shared(&self.proc).await
    }

    /// 执行隧道健康检查
    ///
    /// 根据配置选择不同的健康检查策略：
    /// - 如果配置了 `health_url`，发送 HTTP GET 请求验证服务可用性
    /// - 否则，检查隧道子进程是否仍在运行
    ///
    /// # 返回值
    ///
    /// - `true`：隧道健康（HTTP 请求成功或子进程运行中）
    /// - `false`：隧道不可用（HTTP 请求失败或子进程已退出）
    ///
    /// # 超时设置
    ///
    /// HTTP 健康检查设置 5 秒超时，避免因网络问题长时间阻塞。
    async fn health_check(&self) -> bool {
        // 如果配置了健康检查 URL，通过 HTTP 请求验证
        if let Some(ref url) = self.health_url {
            return agent_config::schema::build_runtime_proxy_client("tunnel.custom")
                .get(url)
                .timeout(std::time::Duration::from_secs(5))
                .send()
                .await
                .is_ok_and(|response| response.status().is_success());
        }

        // 未配置健康检查 URL 时，检查子进程是否仍在运行
        let guard = self.proc.lock().await;
        guard.as_ref().is_some_and(|tp| tp.child.id().is_some())
    }

    /// 获取隧道的公网访问地址
    ///
    /// 尝试获取当前隧道的公网 URL。此方法使用非阻塞锁，
    /// 如果锁正被占用则立即返回 `None`。
    ///
    /// # 返回值
    ///
    /// - `Some(String)`：成功获取到公网地址
    /// - `None`：隧道未启动或锁被占用
    ///
    /// # 注意事项
    ///
    /// 使用 `try_lock` 而非阻塞锁，避免在健康检查等场景中造成死锁。
    fn public_url(&self) -> Option<String> {
        self.proc.try_lock().ok().and_then(|g| g.as_ref().map(|tp| tp.public_url.clone()))
    }
}

/// 单元测试模块
#[cfg(test)]
mod tests;
