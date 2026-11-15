//! Telegram 文件下载辅助模块。
//!
//! 本模块封装 Telegram Bot API 的 `getFile` 查询和文件二进制下载流程。
//! 它只负责从 Telegram 服务端取得文件路径与内容，不决定文件的落盘位置，
//! 从而让附件、语音等上层模块各自执行自己的安全校验和存储策略。

use super::TelegramChannel;
use anyhow::Context;

impl TelegramChannel {
    /// 通过 Telegram `getFile` API 查询文件路径。
    ///
    /// # 参数
    /// - `file_id`: Telegram 消息附件中的文件标识符。
    ///
    /// # 返回值
    /// 成功时返回 Telegram 服务端提供的 `file_path`。
    ///
    /// # 错误
    /// 当 API 调用失败、响应不是有效 JSON，或响应缺少 `result.file_path` 时返回错误。
    pub(super) async fn get_file_path(&self, file_id: &str) -> anyhow::Result<String> {
        let url = self.api_url("getFile");
        let resp = self
            .http_client()
            .get(&url)
            .query(&[("file_id", file_id)])
            .send()
            .await
            .context("Failed to call Telegram getFile")?;

        let data: serde_json::Value = resp.json().await?;
        data.get("result")
            .and_then(|r| r.get("file_path"))
            .and_then(serde_json::Value::as_str)
            .map(String::from)
            .context("Telegram getFile: missing file_path in response")
    }

    /// 从 Telegram 文件接口下载二进制内容。
    ///
    /// # 参数
    /// - `file_path`: `get_file_path` 返回的 Telegram 文件路径。
    ///
    /// # 返回值
    /// 成功时返回文件字节。
    ///
    /// # 错误
    /// 网络请求失败、HTTP 状态非成功，或读取响应体失败时返回错误。
    ///
    /// # 安全说明
    /// URL 中包含 bot token，因此调用方和日志路径不得记录完整下载 URL。
    /// 本函数仅在内存中构造 URL 并不打印它。
    pub(super) async fn download_file(&self, file_path: &str) -> anyhow::Result<Vec<u8>> {
        let url = format!("{}/file/bot{}/{file_path}", self.api_base, self.bot_token);
        let resp = self
            .http_client()
            .get(&url)
            .timeout(std::time::Duration::from_secs(120))
            .send()
            .await
            .context("Failed to download Telegram file")?;

        if !resp.status().is_success() {
            // 这里只暴露状态码，避免把包含 token 的请求上下文写入错误消息。
            anyhow::bail!("Telegram file download failed: {}", resp.status());
        }

        Ok(resp.bytes().await?.to_vec())
    }
}
