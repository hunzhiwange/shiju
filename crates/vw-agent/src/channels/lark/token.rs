//! 飞书通道令牌管理模块
//!
//! 本模块负责飞书 API 令牌的生命周期管理，包括：
//! - 租户访问令牌（tenant_access_token）的获取、缓存与刷新
//! - 令牌有效性检测与自动重试机制
//! - 图片下载与 Base64 编码
//! - 消息反应（表情）添加
//! - 机器人 open_id 解析与缓存
//!
//! # 核心设计
//!
//! - **主动刷新策略**：令牌在过期前提前刷新，避免请求时令牌过期
//! - **透明重试**：API 返回 401 或无效令牌错误时自动刷新并重试
//! - **安全脱敏**：错误信息中的敏感内容会被净化处理

use super::LarkChannel;
use super::constants::{
    LARK_DEFAULT_TOKEN_TTL, LARK_INVALID_ACCESS_TOKEN_CODE, LARK_TOKEN_REFRESH_SKEW,
};
use base64::Engine;
use std::time::{Duration, Instant};

/// 缓存的租户令牌
///
/// 存储租户访问令牌及其刷新时间点。令牌会在实际过期前提前刷新，
/// 确保后续 API 调用始终使用有效的令牌。
#[derive(Debug, Clone)]
pub(crate) struct CachedTenantToken {
    /// 令牌字符串值
    pub(crate) value: String,
    /// 建议刷新时间点（早于实际过期时间）
    pub(crate) refresh_after: Instant,
}

/// 从飞书 API 响应中提取错误码
///
/// # 参数
///
/// - `body`: 飞书 API 返回的 JSON 响应体
///
/// # 返回值
///
/// 返回 `code` 字段的整数值，若不存在则返回 `None`
///
/// # 示例
///
/// ```ignore
/// let body = serde_json::json!({"code": 99991663, "msg": "invalid access token"});
/// let code = extract_lark_response_code(&body); // Some(99991663)
/// ```
pub(crate) fn extract_lark_response_code(body: &serde_json::Value) -> Option<i64> {
    body.get("code").and_then(|c| c.as_i64())
}

/// 判断响应是否表示访问令牌无效
///
/// # 参数
///
/// - `body`: 飞书 API 返回的 JSON 响应体
///
/// # 返回值
///
/// 若错误码为无效访问令牌错误码（99991663），返回 `true`
fn is_lark_invalid_access_token(body: &serde_json::Value) -> bool {
    extract_lark_response_code(body) == Some(LARK_INVALID_ACCESS_TOKEN_CODE)
}

/// 判断是否需要刷新租户令牌
///
/// 当满足以下任一条件时需要刷新：
/// - HTTP 状态码为 401（未授权）
/// - 响应体中的错误码表示访问令牌无效
///
/// # 参数
///
/// - `status`: HTTP 响应状态码
/// - `body`: 飞书 API 返回的 JSON 响应体
///
/// # 返回值
///
/// 需要刷新令牌时返回 `true`
pub(crate) fn should_refresh_lark_tenant_token(
    status: reqwest::StatusCode,
    body: &serde_json::Value,
) -> bool {
    status == reqwest::StatusCode::UNAUTHORIZED || is_lark_invalid_access_token(body)
}

/// 从令牌响应中提取有效期（秒）
///
/// 飞书 API 返回的令牌有效期可能使用 `expire` 或 `expires_in` 字段，
/// 且可能是无符号整数或有符号整数。本函数兼容所有这些情况。
///
/// # 参数
///
/// - `body`: 令牌 API 返回的 JSON 响应体
///
/// # 返回值
///
/// 返回令牌有效期（秒），最小值为 1。若无法解析则使用默认 TTL。
pub(crate) fn extract_lark_token_ttl_seconds(body: &serde_json::Value) -> u64 {
    let Some(value) = body.get("expire").or_else(|| body.get("expires_in")) else {
        return LARK_DEFAULT_TOKEN_TTL.as_secs();
    };

    if let Some(ttl) = value.as_i64() {
        return u64::try_from(ttl.max(1)).unwrap_or(1);
    }

    value.as_u64().unwrap_or(LARK_DEFAULT_TOKEN_TTL.as_secs()).max(1)
}

/// 计算下次令牌刷新的截止时间
///
/// 基于 TTL 提前一定时间刷新令牌，避免在实际过期时才刷新导致请求失败。
/// 提前量由 `LARK_TOKEN_REFRESH_SKEW` 常量定义。
///
/// # 参数
///
/// - `now`: 当前时间点
/// - `ttl_seconds`: 令牌有效期（秒）
///
/// # 返回值
///
/// 建议刷新令牌的时间点
pub(crate) fn next_token_refresh_deadline(now: Instant, ttl_seconds: u64) -> Instant {
    // 确保 TTL 至少为 1 秒
    let ttl = Duration::from_secs(ttl_seconds.max(1));
    // 计算刷新提前量，若 TTL 小于提前量则使用 1 秒
    let refresh_in = ttl.checked_sub(LARK_TOKEN_REFRESH_SKEW).unwrap_or(Duration::from_secs(1));
    now + refresh_in
}

/// 净化飞书响应体用于日志输出
///
/// 移除响应体中的敏感信息（如令牌、密钥等），确保日志安全。
///
/// # 参数
///
/// - `body`: 原始 JSON 响应体
///
/// # 返回值
///
/// 净化后的字符串，可安全用于日志记录
pub(crate) fn sanitize_lark_body(body: &serde_json::Value) -> String {
    crate::app::agent::providers::sanitize_api_error(&body.to_string())
}

/// 确保飞书消息发送成功
///
/// 检查 HTTP 状态码和响应体中的错误码，若任一表示失败则返回错误。
///
/// # 参数
///
/// - `status`: HTTP 响应状态码
/// - `body`: 飞书 API 返回的 JSON 响应体
/// - `context`: 错误上下文描述（用于错误消息）
///
/// # 返回值
///
/// - `Ok(())`: 发送成功
/// - `Err`: 发送失败，包含脱敏后的错误详情
///
/// # 错误
///
/// 当 HTTP 状态码非 2xx 或响应体中 `code` 非 0 时返回错误
pub(crate) fn ensure_lark_send_success(
    status: reqwest::StatusCode,
    body: &serde_json::Value,
    context: &str,
) -> anyhow::Result<()> {
    // 检查 HTTP 状态码
    if !status.is_success() {
        let sanitized = sanitize_lark_body(body);
        anyhow::bail!("Lark send failed {context}: status={status}, body={sanitized}");
    }

    // 检查飞书业务错误码（0 表示成功）
    let code = extract_lark_response_code(body).unwrap_or(0);
    if code != 0 {
        let sanitized = sanitize_lark_body(body);
        anyhow::bail!("Lark send failed {context}: code={code}, body={sanitized}");
    }

    Ok(())
}

impl LarkChannel {
    /// 获取已解析的机器人 open_id
    ///
    /// open_id 用于在群聊消息中识别机器人被 @ 的情况。
    /// 该值在首次需要时从飞书 API 获取并缓存。
    ///
    /// # 返回值
    ///
    /// 返回缓存的机器人 open_id，若未解析或读取失败则返回 `None`
    pub(crate) fn resolved_bot_open_id(&self) -> Option<String> {
        self.resolved_bot_open_id.read().ok().and_then(|guard| guard.clone())
    }

    /// 设置已解析的机器人 open_id
    ///
    /// # 参数
    ///
    /// - `open_id`: 从飞书 API 获取的机器人 open_id，或 `None` 表示解析失败
    pub(crate) fn set_resolved_bot_open_id(&self, open_id: Option<String>) {
        if let Ok(mut guard) = self.resolved_bot_open_id.write() {
            *guard = open_id;
        }
    }

    /// 获取图片并将其编码为内嵌数据标记
    ///
    /// 从飞书下载指定图片，转换为 Base64 编码的内嵌数据格式，
    /// 用于在消息中嵌入图片内容。
    ///
    /// # 参数
    ///
    /// - `image_key`: 飞书图片的唯一标识符
    ///
    /// # 返回值
    ///
    /// 返回格式为 `[IMAGE:data:{media_type};base64,{encoded_data}]` 的字符串
    ///
    /// # 错误
    ///
    /// - `image_key` 为空
    /// - 下载失败（网络错误、权限错误等）
    /// - 图片内容为空
    ///
    /// # 重试机制
    ///
    /// 若因令牌过期导致下载失败，会自动刷新令牌并重试一次
    pub(crate) async fn fetch_image_marker(&self, image_key: &str) -> anyhow::Result<String> {
        // 校验 image_key 非空
        if image_key.trim().is_empty() {
            anyhow::bail!("empty image_key");
        }

        let mut token = self.get_tenant_access_token().await?;
        let mut retried = false;
        let url = self.image_download_url(image_key);

        loop {
            // 发送图片下载请求
            let response = self
                .http_client()
                .get(&url)
                .header("Authorization", format!("Bearer {token}"))
                .send()
                .await?;

            let status = response.status();
            // 提取 Content-Type 头，用于确定图片类型
            let content_type = response
                .headers()
                .get(reqwest::header::CONTENT_TYPE)
                .and_then(|value| value.to_str().ok())
                .map(str::to_string);
            let body = response.bytes().await?;

            // 下载成功，进行 Base64 编码
            if status.is_success() {
                if body.is_empty() {
                    anyhow::bail!("image payload is empty");
                }
                // 解析媒体类型，默认为 image/png
                let media_type = content_type
                    .as_deref()
                    .and_then(|value| value.split(';').next())
                    .map(str::trim)
                    .filter(|value| value.starts_with("image/"))
                    .unwrap_or("image/png");
                let encoded = base64::engine::general_purpose::STANDARD.encode(body);
                return Ok(format!("[IMAGE:data:{media_type};base64,{encoded}]"));
            }

            // 下载失败，解析错误响应
            let parsed = serde_json::from_slice::<serde_json::Value>(&body)
                .unwrap_or(serde_json::Value::Null);
            // 若是令牌问题且尚未重试，则刷新令牌后重试
            if !retried && should_refresh_lark_tenant_token(status, &parsed) {
                self.invalidate_token().await;
                token = self.get_tenant_access_token().await?;
                retried = true;
                continue;
            }

            // 其他错误直接返回
            anyhow::bail!(
                "Lark image download failed: status={status}, body={}",
                crate::app::agent::providers::sanitize_api_error(&String::from_utf8_lossy(&body))
            );
        }
    }

    /// 使用指定令牌为消息添加表情反应
    ///
    /// # 参数
    ///
    /// - `message_id`: 目标消息 ID
    /// - `token`: 访问令牌
    /// - `emoji_type`: 表情类型（如 "THUMBSUP"）
    ///
    /// # 返回值
    ///
    /// 返回原始 HTTP 响应，由调用方处理响应内容
    async fn post_message_reaction_with_token(
        &self,
        message_id: &str,
        token: &str,
        emoji_type: &str,
    ) -> anyhow::Result<reqwest::Response> {
        let url = self.message_reaction_url(message_id);
        let body = serde_json::json!({
            "reaction_type": {
                "emoji_type": emoji_type
            }
        });

        let response = self
            .http_client()
            .post(&url)
            .header("Authorization", format!("Bearer {token}"))
            .header("Content-Type", "application/json; charset=utf-8")
            .json(&body)
            .send()
            .await?;

        Ok(response)
    }

    /// 尝试为消息添加确认表情反应
    ///
    /// 这是尽最大努力的"已接收"信号，用于在收到消息时给予用户反馈。
    /// 失败时仅记录日志，不会阻塞正常的消息处理流程。
    ///
    /// # 参数
    ///
    /// - `message_id`: 目标消息 ID
    /// - `emoji_type`: 要添加的表情类型
    ///
    /// # 重试机制
    ///
    /// 若因令牌过期（HTTP 401）导致失败，会自动刷新令牌并重试一次
    pub(crate) async fn try_add_ack_reaction(&self, message_id: &str, emoji_type: &str) {
        // 空消息 ID 直接跳过
        if message_id.is_empty() {
            return;
        }

        // 获取访问令牌，失败则记录警告并返回
        let mut token = match self.get_tenant_access_token().await {
            Ok(token) => token,
            Err(err) => {
                tracing::warn!("Lark: failed to fetch token for reaction: {err}");
                return;
            }
        };

        let mut retried = false;
        loop {
            // 发送表情反应请求
            let response =
                match self.post_message_reaction_with_token(message_id, &token, emoji_type).await {
                    Ok(resp) => resp,
                    Err(err) => {
                        tracing::warn!("Lark: failed to add reaction for {message_id}: {err}");
                        return;
                    }
                };

            // 401 错误时尝试刷新令牌并重试
            if response.status().as_u16() == 401 && !retried {
                self.invalidate_token().await;
                token = match self.get_tenant_access_token().await {
                    Ok(new_token) => new_token,
                    Err(err) => {
                        tracing::warn!(
                            "Lark: failed to refresh token for reaction on {message_id}: {err}"
                        );
                        return;
                    }
                };
                retried = true;
                continue;
            }

            // 非 2xx 状态码，记录警告并返回
            if !response.status().is_success() {
                let status = response.status();
                let err_body = response.text().await.unwrap_or_default();
                let sanitized = crate::app::agent::providers::sanitize_api_error(&err_body);
                tracing::warn!(
                    "Lark: add reaction failed for {message_id}: status={status}, body={sanitized}"
                );
                return;
            }

            // 解析响应体检查业务错误码
            let payload: serde_json::Value = match response.json().await {
                Ok(v) => v,
                Err(err) => {
                    tracing::warn!("Lark: add reaction decode failed for {message_id}: {err}");
                    return;
                }
            };

            // code 为 0 表示成功，否则记录警告
            let code = payload.get("code").and_then(|v| v.as_i64()).unwrap_or(-1);
            if code != 0 {
                let msg = payload.get("msg").and_then(|v| v.as_str()).unwrap_or("unknown error");
                tracing::warn!("Lark: add reaction returned code={code} for {message_id}: {msg}");
            }
            return;
        }
    }

    /// 获取或刷新租户访问令牌
    ///
    /// 首先检查缓存中是否有有效的令牌，若有则直接返回。
    /// 若缓存为空或令牌已过期，则向飞书 API 请求新令牌并缓存。
    ///
    /// # 返回值
    ///
    /// 返回有效的租户访问令牌字符串
    ///
    /// # 错误
    ///
    /// - HTTP 请求失败
    /// - 响应状态码非 2xx
    /// - 响应体中 `code` 非 0
    /// - 响应体中缺少 `tenant_access_token` 字段
    ///
    /// # 缓存策略
    ///
    /// 令牌会在实际过期前提前刷新（由 `LARK_TOKEN_REFRESH_SKEW` 控制），
    /// 确保后续 API 调用不会因令牌过期而失败
    pub(crate) async fn get_tenant_access_token(&self) -> anyhow::Result<String> {
        // 首先检查缓存
        {
            let cached = self.tenant_token.read().await;
            if let Some(ref token) = *cached {
                // 未到达刷新时间，直接返回缓存的令牌
                if Instant::now() < token.refresh_after {
                    return Ok(token.value.clone());
                }
            }
        }

        // 缓存无效，请求新令牌
        let url = self.tenant_access_token_url();
        let body = serde_json::json!({
            "app_id": self.app_id,
            "app_secret": self.app_secret,
        });

        let resp = self.http_client().post(&url).json(&body).send().await?;
        let status = resp.status();
        let data: serde_json::Value = resp.json().await?;

        // 检查 HTTP 状态码
        if !status.is_success() {
            let sanitized = sanitize_lark_body(&data);
            anyhow::bail!(
                "Lark tenant_access_token request failed: status={status}, body={sanitized}"
            );
        }

        // 检查业务错误码
        let code = data.get("code").and_then(|c| c.as_i64()).unwrap_or(-1);
        if code != 0 {
            let msg = data.get("msg").and_then(|m| m.as_str()).unwrap_or("unknown error");
            anyhow::bail!("Lark tenant_access_token failed: {msg}");
        }

        // 提取令牌字符串
        let token = data
            .get("tenant_access_token")
            .and_then(|t| t.as_str())
            .ok_or_else(|| anyhow::anyhow!("missing tenant_access_token in response"))?
            .to_string();

        // 计算刷新时间并缓存
        let ttl_seconds = extract_lark_token_ttl_seconds(&data);
        let refresh_after = next_token_refresh_deadline(Instant::now(), ttl_seconds);

        // 缓存令牌及刷新元数据
        {
            let mut cached = self.tenant_token.write().await;
            *cached = Some(CachedTenantToken { value: token.clone(), refresh_after });
        }

        Ok(token)
    }

    /// 使缓存的令牌失效
    ///
    /// 当 API 返回令牌过期错误时调用，清除缓存以便下次请求时获取新令牌。
    pub(crate) async fn invalidate_token(&self) {
        let mut cached = self.tenant_token.write().await;
        *cached = None;
    }

    /// 使用指定令牌获取机器人信息
    ///
    /// # 参数
    ///
    /// - `token`: 访问令牌
    ///
    /// # 返回值
    ///
    /// 返回元组：(HTTP 状态码, 响应 JSON 体)
    async fn fetch_bot_open_id_with_token(
        &self,
        token: &str,
    ) -> anyhow::Result<(reqwest::StatusCode, serde_json::Value)> {
        let resp = self
            .http_client()
            .get(self.bot_info_url())
            .header("Authorization", format!("Bearer {token}"))
            .send()
            .await?;
        let status = resp.status();
        // 解析失败时返回空对象而非报错，由调用方处理
        let body = resp.json::<serde_json::Value>().await.unwrap_or_else(|_| serde_json::json!({}));
        Ok((status, body))
    }

    /// 刷新机器人的 open_id
    ///
    /// 从飞书 API 获取机器人的 open_id 信息，用于识别群聊中被 @ 的消息。
    ///
    /// # 返回值
    ///
    /// - `Ok(Some(open_id))`: 成功获取到 open_id
    /// - `Ok(None)`: API 响应中不包含 open_id
    /// - `Err`: 请求失败
    ///
    /// # 重试机制
    ///
    /// 若因令牌问题导致失败，会自动刷新令牌并重试一次
    async fn refresh_bot_open_id(&self) -> anyhow::Result<Option<String>> {
        let token = self.get_tenant_access_token().await?;
        let (status, body) = self.fetch_bot_open_id_with_token(&token).await?;

        // 若因令牌问题失败，刷新令牌后重试
        let body = if should_refresh_lark_tenant_token(status, &body) {
            self.invalidate_token().await;
            let refreshed = self.get_tenant_access_token().await?;
            let (retry_status, retry_body) = self.fetch_bot_open_id_with_token(&refreshed).await?;
            if !retry_status.is_success() {
                let sanitized = sanitize_lark_body(&retry_body);
                anyhow::bail!(
                    "Lark bot info request failed after token refresh: status={retry_status}, body={sanitized}"
                );
            }
            retry_body
        } else {
            if !status.is_success() {
                let sanitized = sanitize_lark_body(&body);
                anyhow::bail!("Lark bot info request failed: status={status}, body={sanitized}");
            }
            body
        };

        // 检查业务错误码
        let code = body.get("code").and_then(|c| c.as_i64()).unwrap_or(-1);
        if code != 0 {
            let sanitized = sanitize_lark_body(&body);
            anyhow::bail!("Lark bot info failed: code={code}, body={sanitized}");
        }

        // 从响应中提取 open_id，支持两种路径格式
        let bot_open_id = body
            .pointer("/bot/open_id")
            .or_else(|| body.pointer("/data/bot/open_id"))
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(str::to_owned);

        // 缓存解析结果
        self.set_resolved_bot_open_id(bot_open_id.clone());
        Ok(bot_open_id)
    }

    /// 确保已解析机器人的 open_id
    ///
    /// 当启用 `mention_only` 模式且尚未解析 open_id 时，
    /// 从飞书 API 获取机器人的 open_id 以便过滤群聊消息。
    ///
    /// 若解析失败仅记录警告，不会阻止消息处理
    pub(crate) async fn ensure_bot_open_id(&self) {
        // 若未启用 mention_only 或已有 open_id，则无需处理
        if !self.mention_only || self.resolved_bot_open_id().is_some() {
            return;
        }

        // 尝试刷新 open_id
        match self.refresh_bot_open_id().await {
            Ok(Some(open_id)) => {
                tracing::info!("Lark: resolved bot open_id: {open_id}");
            }
            Ok(None) => {
                tracing::warn!(
                    "Lark: bot open_id missing from /bot/v3/info response; mention_only group messages will be ignored"
                );
            }
            Err(err) => {
                tracing::warn!(
                    "Lark: failed to resolve bot open_id: {err}; mention_only group messages will be ignored"
                );
            }
        }
    }

    /// 发送单次文本消息
    ///
    /// 使用指定的 URL、令牌和请求体发送消息，返回原始响应。
    ///
    /// # 参数
    ///
    /// - `url`: 消息发送 API 的完整 URL
    /// - `token`: 访问令牌
    /// - `body`: 请求体 JSON
    ///
    /// # 返回值
    ///
    /// 返回元组：(HTTP 状态码, 响应 JSON 体)
    ///
    /// # 说明
    ///
    /// 若响应体无法解析为 JSON，则将其包装为 `{"raw": "<原始文本>"}` 格式
    pub(crate) async fn send_text_once(
        &self,
        url: &str,
        token: &str,
        body: &serde_json::Value,
    ) -> anyhow::Result<(reqwest::StatusCode, serde_json::Value)> {
        let resp = self
            .http_client()
            .post(url)
            .header("Authorization", format!("Bearer {token}"))
            .header("Content-Type", "application/json; charset=utf-8")
            .json(body)
            .send()
            .await?;
        let status = resp.status();
        // 获取原始文本响应
        let raw = resp.text().await.unwrap_or_default();
        // 尝试解析为 JSON，失败则包装为 raw 字段
        let parsed = serde_json::from_str::<serde_json::Value>(&raw)
            .unwrap_or_else(|_| serde_json::json!({ "raw": raw }));
        Ok((status, parsed))
    }
}

#[cfg(test)]
#[path = "token_tests.rs"]
mod token_tests;
