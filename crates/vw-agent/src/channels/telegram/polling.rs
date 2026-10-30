//! Telegram 长轮询监听模块。
//!
//! 本模块负责通过 Telegram Bot API 的 `getUpdates` 拉取入站更新，
//! 将可处理的更新解析为通道消息并送入代理运行时。它还处理启动探测、
//! 409 冲突退避、bot 用户名缓存刷新、确认反应和健康检查等网络边界逻辑。

use super::TelegramChannel;
use crate::app::agent::channels::traits::ChannelMessage;
use std::time::Duration;

impl TelegramChannel {
    /// 启动 Telegram 长轮询并把解析后的消息发送到运行时队列。
    ///
    /// # 参数
    /// - `tx`: 通道消息发送端，监听到的消息会通过它交给上层运行时。
    ///
    /// # 返回值
    /// 队列关闭时返回 `Ok(())`；当前实现会在可恢复网络/API 错误上持续重试。
    ///
    /// # 错误处理
    /// 网络、JSON 解析和 Telegram API 临时错误会记录脱敏日志并退避重试。
    /// 当上层接收端关闭时结束监听并返回成功。
    ///
    /// # 安全说明
    /// 日志中的 Telegram 错误会经过脱敏，避免 bot token 或敏感响应内容泄露。
    pub(super) async fn listen_impl(
        &self,
        tx: tokio::sync::mpsc::Sender<ChannelMessage>,
    ) -> anyhow::Result<()> {
        let mut offset: i64 = 0;

        if self.mention_only {
            // mention-only 需要 bot 用户名才能判断群组消息是否显式点名，启动时先尽量预热缓存。
            let _ = self.get_bot_username().await;
        }

        tracing::info!("Telegram channel listening for messages...");

        loop {
            let url = self.api_url("getUpdates");
            let probe = serde_json::json!({
                "offset": offset,
                "timeout": 0,
                "allowed_updates": ["message", "callback_query"]
            });
            match self.http_client().post(&url).json(&probe).send().await {
                Err(e) => {
                    let sanitized = Self::sanitize_telegram_error(&e.to_string());
                    tracing::warn!("Telegram startup probe error: {sanitized}; retrying in 5s");
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                }
                Ok(resp) => match resp.json::<serde_json::Value>().await {
                    Err(e) => {
                        let sanitized = Self::sanitize_telegram_error(&e.to_string());
                        tracing::warn!(
                            "Telegram startup probe parse error: {sanitized}; retrying in 5s"
                        );
                        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    }
                    Ok(data) => {
                        let ok =
                            data.get("ok").and_then(serde_json::Value::as_bool).unwrap_or(false);
                        if ok {
                            if let Some(results) =
                                data.get("result").and_then(serde_json::Value::as_array)
                            {
                                for update in results {
                                    if let Some(uid) =
                                        update.get("update_id").and_then(serde_json::Value::as_i64)
                                    {
                                        // 启动探测阶段跳过已有更新，避免进程重启后重复处理旧消息。
                                        offset = uid + 1;
                                    }
                                }
                            }
                            break;
                        }

                        let error_code = data
                            .get("error_code")
                            .and_then(serde_json::Value::as_i64)
                            .unwrap_or_default();
                        if error_code == 409 {
                            // 409 表示同一 bot token 已被其他 getUpdates 消费者占用，退避等待槽位释放。
                            tracing::debug!("Startup probe: slot busy (409), retrying in 5s");
                        } else {
                            let desc = data
                                .get("description")
                                .and_then(serde_json::Value::as_str)
                                .unwrap_or("unknown");
                            tracing::warn!(
                                "Startup probe: API error {error_code}: {desc}; retrying in 5s"
                            );
                        }
                        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    }
                },
            }
        }

        tracing::debug!("Startup probe succeeded; entering main long-poll loop.");

        loop {
            if self.mention_only {
                let missing_username = self.bot_username.lock().is_none();
                if missing_username {
                    // 用户名可能因启动探测失败而缺失，主循环中补拉以恢复群组提及过滤。
                    let _ = self.get_bot_username().await;
                }
            }

            let url = self.api_url("getUpdates");
            let body = serde_json::json!({
                "offset": offset,
                "timeout": 30,
                "allowed_updates": ["message", "callback_query"]
            });

            // 服务端长轮询等待 30 秒，HTTP 期限须为响应传输留出余量。
            let resp = match self
                .http_client()
                .post(&url)
                .timeout(Duration::from_secs(60))
                .json(&body)
                .send()
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    let sanitized = Self::sanitize_telegram_error(&e.to_string());
                    tracing::warn!("Telegram poll error: {sanitized}");
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    continue;
                }
            };

            let data: serde_json::Value = match resp.json().await {
                Ok(d) => d,
                Err(e) => {
                    let sanitized = Self::sanitize_telegram_error(&e.to_string());
                    tracing::warn!("Telegram parse error: {sanitized}");
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    continue;
                }
            };

            let ok = data.get("ok").and_then(serde_json::Value::as_bool).unwrap_or(true);
            if !ok {
                let error_code =
                    data.get("error_code").and_then(serde_json::Value::as_i64).unwrap_or_default();
                let description = data
                    .get("description")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("unknown Telegram API error");

                if error_code == 409 {
                    tracing::warn!(
                        "Telegram polling conflict (409): {description}. \
Ensure only one `vibewindow` process is using this bot token."
                    );
                    // 冲突通常不是瞬时网络抖动，使用更长退避降低对 Telegram API 的压力。
                    tokio::time::sleep(std::time::Duration::from_secs(35)).await;
                } else {
                    tracing::warn!(
                        "Telegram getUpdates API error (code={}): {description}",
                        error_code
                    );
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                }
                continue;
            }

            if let Some(results) = data.get("result").and_then(serde_json::Value::as_array) {
                for update in results {
                    if let Some(uid) = update.get("update_id").and_then(serde_json::Value::as_i64) {
                        offset = uid + 1;
                    }

                    let msg = if let Some(m) = self.parse_update_message(update) {
                        m
                    } else if let Some(m) = self.try_parse_approval_callback_query(update) {
                        m
                    } else if let Some(m) = self.try_parse_attachment_message(update).await {
                        m
                    } else {
                        self.handle_unauthorized_message(update).await;
                        continue;
                    };

                    if let Some((reaction_chat_id, reaction_message_id)) =
                        Self::extract_update_message_target(update)
                    {
                        // 确认反应是用户体验优化，异步非阻塞执行，避免拖慢消息入队。
                        self.try_add_ack_reaction_nonblocking(
                            reaction_chat_id,
                            reaction_message_id,
                        );
                    }

                    let typing_body = serde_json::json!({
                        "chat_id": &msg.reply_target,
                        "action": "typing"
                    });
                    let _ = self
                        .http_client()
                        .post(self.api_url("sendChatAction"))
                        .json(&typing_body)
                        .send()
                        .await;

                    if tx.send(msg).await.is_err() {
                        return Ok(());
                    }
                }
            }
        }
    }

    /// 检查 Telegram Bot API 是否可用。
    ///
    /// # 返回值
    /// `getMe` 在 5 秒内返回成功状态码时返回 `true`，否则返回 `false`。
    ///
    /// # 错误处理
    /// 请求失败和超时只记录调试日志，不向外传播错误，便于健康检查调用方做布尔判断。
    pub(super) async fn health_check_impl(&self) -> bool {
        let timeout_duration = Duration::from_secs(5);

        match tokio::time::timeout(
            timeout_duration,
            self.http_client().get(self.api_url("getMe")).send(),
        )
        .await
        {
            Ok(Ok(resp)) => resp.status().is_success(),
            Ok(Err(e)) => {
                let sanitized = Self::sanitize_telegram_error(&e.to_string());
                tracing::debug!("Telegram health check failed: {sanitized}");
                false
            }
            Err(_) => {
                tracing::debug!("Telegram health check timed out after 5s");
                false
            }
        }
    }
}
