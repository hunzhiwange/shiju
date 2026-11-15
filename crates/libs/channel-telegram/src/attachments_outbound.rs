//! Telegram 通道出站附件处理模块
//!
//! 本模块负责处理 Telegram 通道中的出站附件操作，包括：
//! - 从 Telegram 服务器下载图片文件
//! - 调整图片尺寸以优化传输
//! - 将图片转换为 data URI 格式
//!
//! 主要用于将 Telegram 消息中的图片附件转换为可在内部消息流中使用的格式。

use super::TelegramChannel;
use base64::Engine as _;

impl TelegramChannel {
    /// 解析 Telegram 图片文件并转换为 data URI 格式
    ///
    /// 此方法从 Telegram 服务器下载指定 file_id 的图片文件，
    /// 自动调整尺寸（限制最大边长为 512 像素），并将其编码为
    /// base64 格式的 data URI 字符串。
    ///
    /// # 参数
    ///
    /// * `file_id` - Telegram 文件标识符，用于从 Telegram API 获取文件
    ///
    /// # 返回值
    ///
    /// 返回格式为 `data:image/jpeg;base64,<base64编码的数据>` 的字符串
    ///
    /// # 错误
    ///
    /// 此方法可能返回以下错误：
    /// - HTTP 请求失败（网络错误、API 错误等）
    /// - JSON 解析失败（响应格式不正确）
    /// - 文件路径不存在于响应中
    /// - 图片加载/处理失败（格式不支持、数据损坏等）
    /// - Base64 编码失败
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let channel = TelegramChannel::new(config);
    /// let data_uri = channel.resolve_photo_data_uri("AgACAgUAAxkBAA...").await?;
    /// // data_uri = "data:image/jpeg;base64,/9j/4AAQSkZJRgABAQ..."
    /// ```
    ///
    /// # 性能说明
    ///
    /// 图片处理（调整尺寸）在阻塞线程池中执行，以避免阻塞异步运行时。
    /// 这对于大图片处理特别重要。
    pub(super) async fn resolve_photo_data_uri(&self, file_id: &str) -> anyhow::Result<String> {
        // 步骤 1：通过 Telegram getFile API 获取文件路径信息
        let get_file_url = self.api_url(&format!("getFile?file_id={}", file_id));
        let resp = self.http_client().get(&get_file_url).send().await?;
        let json: serde_json::Value = resp.json().await?;

        // 从 JSON 响应中提取 file_path 字段
        // Telegram API 返回格式：{"ok": true, "result": {"file_path": "photos/file.jpg", ...}}
        let file_path = json
            .get("result")
            .and_then(|r| r.get("file_path"))
            .and_then(|p| p.as_str())
            .ok_or_else(|| anyhow::anyhow!("getFile: no file_path in response"))?
            .to_string();

        // 步骤 2：构建文件下载 URL 并下载图片二进制数据
        // Telegram 文件下载 URL 格式：https://api.telegram.org/file/bot<token>/<file_path>
        let download_url = format!("{}/file/bot{}/{}", self.api_base, self.bot_token, file_path);
        let img_resp = self
            .http_client()
            .get(&download_url)
            .timeout(std::time::Duration::from_secs(120))
            .send()
            .await?;
        let bytes = img_resp.bytes().await?;

        // 步骤 3：在阻塞线程池中处理图片（调整尺寸并转换为 JPEG）
        // 使用 spawn_blocking 避免阻塞异步运行时
        let resized_bytes = tokio::task::spawn_blocking(move || -> anyhow::Result<Vec<u8>> {
            // 从内存加载图片
            let img = image::load_from_memory(&bytes)?;
            let (w, h) = (img.width(), img.height());

            // 定义最大尺寸限制（512 像素）
            // 保持宽高比的同时确保图片不会过大
            let max_dim = 512u32;

            // 仅当图片尺寸超过限制时才进行调整
            // thumbnail 方法会保持宽高比
            let resized =
                if w > max_dim || h > max_dim { img.thumbnail(max_dim, max_dim) } else { img };

            // 将调整后的图片编码为 JPEG 格式
            let mut buf = Vec::new();
            resized.write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Jpeg)?;
            Ok(buf)
        })
        .await??;

        // 步骤 4：将图片数据编码为 base64 并构建 data URI
        let b64 = base64::engine::general_purpose::STANDARD.encode(&resized_bytes);
        Ok(format!("data:image/jpeg;base64,{}", b64))
    }
}
