//! Discord 消息发送客户端模块
//!
//! 该模块提供与 Discord API 交互的底层客户端功能，主要用于发送消息到 Discord 频道。
//! 支持两种消息发送模式：
//! - 纯文本消息
//! - 带文件附件的消息
//!
//! # 功能特性
//!
//! - 使用 Discord Bot API v10 版本
//! - 支持异步操作
//! - 自动错误响应处理和敏感信息脱敏
//! - 支持多文件附件上传

use reqwest::multipart::{Form, Part};
use serde_json::json;
use std::path::PathBuf;

const DISCORD_API_BASE_URL: &str = "https://discord.com/api/v10";

pub(super) fn discord_messages_url(api_base: &str, recipient: &str) -> String {
    format!("{}/channels/{recipient}/messages", api_base.trim_end_matches('/'))
}

/// 通过 Discord API 发送纯文本消息到指定频道
///
/// 该函数使用 Discord Bot Token 进行身份验证，向指定频道发送纯文本消息。
/// 消息内容以 JSON 格式提交到 Discord API v10 端点。
///
/// # 参数
///
/// * `client` - reqwest HTTP 客户端实例，用于发送 HTTP 请求
/// * `bot_token` - Discord Bot 的认证令牌，用于 API 身份验证
/// * `recipient` - 目标频道 ID，消息将发送到此频道
/// * `content` - 要发送的消息文本内容
///
/// # 返回值
///
/// 成功时返回 `Ok(())`，失败时返回包含详细错误信息的 `anyhow::Error`
///
/// # 错误
///
/// 在以下情况会返回错误：
/// - HTTP 请求失败（网络错误、超时等）
/// - Discord API 返回非成功状态码（例如权限不足、频道不存在等）
/// - 响应体读取失败
///
/// # 示例
///
/// ```ignore
/// let client = reqwest::Client::new();
/// let result = send_discord_message_json(
///     &client,
///     "your_bot_token",
///     "1234567890",
///     "Hello, Discord!"
/// ).await;
/// ```
///
/// # 安全性
///
/// 该函数会自动对错误响应进行敏感信息脱敏处理，避免在错误日志中泄露敏感数据
pub(super) async fn send_discord_message_json(
    client: &reqwest::Client,
    bot_token: &str,
    recipient: &str,
    content: &str,
) -> anyhow::Result<()> {
    let url = discord_messages_url(DISCORD_API_BASE_URL, recipient);
    send_discord_message_json_to_url(client, bot_token, &url, content).await
}

pub(super) async fn send_discord_message_json_to_url(
    client: &reqwest::Client,
    bot_token: &str,
    url: &str,
    content: &str,
) -> anyhow::Result<()> {
    // 构造请求体，使用 JSON 格式包含消息内容
    let body = json!({ "content": content });

    // 发送 POST 请求到 Discord API
    // 使用 Bot Token 进行身份验证
    let resp = client
        .post(url)
        .header("Authorization", format!("Bot {bot_token}"))
        .json(&body)
        .send()
        .await?;

    // 检查响应状态，处理错误情况
    if !resp.status().is_success() {
        let status = resp.status();
        // 尝试读取错误响应体，如果读取失败则生成默认错误消息
        let err =
            resp.text().await.unwrap_or_else(|e| format!("<failed to read response body: {e}>"));
        // 对错误信息进行脱敏处理，避免泄露敏感数据
        let sanitized = crate::app::agent::providers::sanitize_api_error(&err);
        anyhow::bail!("Discord send message failed ({status}): {sanitized}");
    }

    Ok(())
}

/// 通过 Discord API 发送带文件附件的消息到指定频道
///
/// 该函数使用 Discord Bot Token 进行身份验证，向指定频道发送包含文本消息和文件附件的消息。
/// 文件以 multipart/form-data 格式上传到 Discord API v10 端点。
///
/// # 参数
///
/// * `client` - reqwest HTTP 客户端实例，用于发送 HTTP 请求
/// * `bot_token` - Discord Bot 的认证令牌，用于 API 身份验证
/// * `recipient` - 目标频道 ID，消息将发送到此频道
/// * `content` - 要发送的消息文本内容
/// * `files` - 要附加的文件路径列表，支持多个文件
///
/// # 返回值
///
/// 成功时返回 `Ok(())`，失败时返回包含详细错误信息的 `anyhow::Error`
///
/// # 错误
///
/// 在以下情况会返回错误：
/// - HTTP 请求失败（网络错误、超时等）
/// - Discord API 返回非成功状态码（例如权限不足、频道不存在等）
/// - 文件读取失败（文件不存在、权限不足等）
/// - 响应体读取失败
///
/// # 文件处理
///
/// - 文件会被异步读取并转换为字节流
/// - 保留原始文件名，如果无法获取文件名则使用 "attachment.bin" 作为默认名称
/// - 文件按索引编号（files[0], files[1], ...）上传到 Discord
/// - 支持任意类型的文件附件
///
/// # 示例
///
/// ```ignore
/// let client = reqwest::Client::new();
/// let files = vec![
///     PathBuf::from("/path/to/file1.txt"),
///     PathBuf::from("/path/to/file2.png"),
/// ];
/// let result = send_discord_message_with_files(
///     &client,
///     "your_bot_token",
///     "1234567890",
///     "Here are the files!",
///     &files
/// ).await;
/// ```
///
/// # 安全性
///
/// - 该函数会自动对错误响应进行敏感信息脱敏处理
/// - 文件路径错误会被明确报告，但不会泄露系统路径之外的信息
pub(super) async fn send_discord_message_with_files(
    client: &reqwest::Client,
    bot_token: &str,
    recipient: &str,
    content: &str,
    files: &[PathBuf],
) -> anyhow::Result<()> {
    let url = discord_messages_url(DISCORD_API_BASE_URL, recipient);
    send_discord_message_with_files_to_url(client, bot_token, &url, content, files).await
}

pub(super) async fn send_discord_message_with_files_to_url(
    client: &reqwest::Client,
    bot_token: &str,
    url: &str,
    content: &str,
    files: &[PathBuf],
) -> anyhow::Result<()> {
    // 初始化 multipart 表单，添加 JSON 格式的消息内容作为 payload
    let mut form = Form::new().text("payload_json", json!({ "content": content }).to_string());

    // 遍历所有文件路径，读取文件内容并添加到表单中
    for (idx, path) in files.iter().enumerate() {
        // 异步读取文件内容，如果失败则返回详细的错误信息
        let bytes = tokio::fs::read(path).await.map_err(|error| {
            anyhow::anyhow!("Discord attachment read failed for '{}': {error}", path.display())
        })?;

        // 提取文件名，如果无法提取则使用默认名称 "attachment.bin"
        let filename =
            path.file_name().and_then(|name| name.to_str()).unwrap_or("attachment.bin").to_string();

        // 将文件内容添加到表单中，使用索引编号作为字段名
        form = form.part(format!("files[{idx}]"), Part::bytes(bytes).file_name(filename));
    }

    // 发送 POST 请求到 Discord API，使用 multipart/form-data 格式
    let resp = client
        .post(url)
        .header("Authorization", format!("Bot {bot_token}"))
        .multipart(form)
        .send()
        .await?;

    // 检查响应状态，处理错误情况
    if !resp.status().is_success() {
        let status = resp.status();
        // 尝试读取错误响应体，如果读取失败则生成默认错误消息
        let err =
            resp.text().await.unwrap_or_else(|e| format!("<failed to read response body: {e}>"));
        // 对错误信息进行脱敏处理，避免泄露敏感数据
        let sanitized = crate::app::agent::providers::sanitize_api_error(&err);
        anyhow::bail!("Discord send message with files failed ({status}): {sanitized}");
    }

    Ok(())
}
