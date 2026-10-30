//! ClawdTalk 语音通道模块 - 通过 Telnyx SIP 基础设施实现实时语音通话
//!
//! 本模块实现了与 ClawdTalk (https://clawdtalk.com) 的集成，
//! 提供基于 AI 的语音对话功能。ClawdTalk 使用 Telnyx 的全球 SIP 网络，
//! 确保低延迟和高质量的通话体验。
//!
//! # 主要功能
//!
//! - 发起和接听语音电话
//! - 文本转语音（TTS）播放
//! - AI 驱动的智能对话
//! - 通话状态管理和挂断
//! - Webhook 事件处理
//!
//! # 架构
//!
//! 本模块实现了 `Channel` trait，作为 VibeWindow 代理系统的语音通道之一。
//! 通过 Telnyx Call Control API 进行电话控制，支持双向语音交互。

pub use vw_config_types::channels::clawdtalk::ClawdTalkConfig;

use crate::app::agent::config::traits::ChannelConfig;

use super::traits::{Channel, ChannelMessage, SendMessage};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

/// ClawdTalk 语音通道
///
/// 管理与 Telnyx API 的交互，提供语音通话、TTS 播放和 AI 对话功能。
/// 该结构体封装了所有必要的配置信息和 HTTP 客户端，
/// 用于与 Telnyx 的 Call Control API 进行通信。
pub struct ClawdTalkChannel {
    /// Telnyx API 密钥，用于身份验证
    api_key: String,
    /// Telnyx 连接 ID（SIP 连接标识符）
    connection_id: String,
    /// 主叫号码或 SIP URI，必须是 E.164 格式
    from_number: String,
    /// 允许拨打的号码或模式列表（支持前缀匹配和通配符）
    allowed_destinations: Vec<String>,
    /// 用于调用 Telnyx API 的 HTTP 客户端
    client: Client,
    #[cfg(test)]
    telnyx_api_url: String,
    /// Webhook 密钥，用于验证传入通话的签名
    webhook_secret: Option<String>,
}

impl ChannelConfig for ClawdTalkConfig {
    fn name() -> &'static str {
        "ClawdTalk"
    }
    fn desc() -> &'static str {
        "ClawdTalk Channel"
    }
}

impl ClawdTalkChannel {
    /// 创建新的 ClawdTalk 通道实例
    ///
    /// # 参数
    ///
    /// - `config`: ClawdTalk 配置对象，包含 API 密钥、连接 ID 等信息
    ///
    /// # 返回值
    ///
    /// 返回初始化后的 `ClawdTalkChannel` 实例
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let config = ClawdTalkConfig {
    ///     api_key: "your_api_key".to_string(),
    ///     connection_id: "connection_id".to_string(),
    ///     from_number: "+12345678900".to_string(),
    ///     allowed_destinations: vec!["+1".to_string()],
    ///     webhook_secret: None,
    /// };
    /// let channel = ClawdTalkChannel::new(config);
    /// ```
    pub fn new(config: ClawdTalkConfig) -> Self {
        Self {
            api_key: config.api_key,
            connection_id: config.connection_id,
            from_number: config.from_number,
            allowed_destinations: config.allowed_destinations,
            client: crate::channels::http::client_builder()
                .build()
                .expect("ClawdTalk HTTP client should build"),
            #[cfg(test)]
            telnyx_api_url: Self::TELNYX_API_URL.to_string(),
            webhook_secret: config.webhook_secret,
        }
    }

    /// Telnyx API 基础 URL
    ///
    /// 所有 Telnyx Call Control API 请求都基于此 URL
    const TELNYX_API_URL: &'static str = "https://api.telnyx.com/v2";

    fn telnyx_api_url(&self) -> &str {
        #[cfg(test)]
        {
            &self.telnyx_api_url
        }
        #[cfg(not(test))]
        {
            Self::TELNYX_API_URL
        }
    }

    #[cfg(test)]
    fn with_telnyx_api_url(mut self, telnyx_api_url: String) -> Self {
        self.telnyx_api_url = telnyx_api_url;
        self
    }

    /// 检查目标号码是否在允许列表中
    ///
    /// # 参数
    ///
    /// - `destination`: 要检查的目标号码
    ///
    /// # 返回值
    ///
    /// 如果目标号码被允许则返回 `true`，否则返回 `false`
    ///
    /// # 匹配规则
    ///
    /// - 如果允许列表为空，则默认允许所有号码
    /// - 支持完整号码精确匹配
    /// - 支持前缀匹配（例如 "+1" 匹配所有美国/加拿大号码）
    /// - 支持通配符 "*" 匹配所有号码
    fn is_destination_allowed(&self, destination: &str) -> bool {
        // 如果未配置允许列表，默认允许所有号码
        if self.allowed_destinations.is_empty() {
            return true;
        }
        // 检查是否匹配任一模式：通配符、前缀或精确匹配
        self.allowed_destinations.iter().any(|pattern| {
            pattern == "*" || destination.starts_with(pattern) || pattern == destination
        })
    }

    /// 通过 Telnyx 发起呼出电话
    ///
    /// # 参数
    ///
    /// - `to`: 目标电话号码，必须是 E.164 格式（如 +12345678900）
    /// - `_prompt`: 可选的初始提示（当前未使用，保留供未来扩展）
    ///
    /// # 返回值
    ///
    /// 成功时返回 `CallSession` 包含通话控制信息，失败时返回错误
    ///
    /// # 错误
    ///
    /// - 如果目标号码不在允许列表中，返回错误
    /// - 如果 Telnyx API 调用失败，返回错误
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let session = channel.initiate_call("+12345678900", None).await?;
    /// println!("Call session ID: {}", session.call_session_id);
    /// ```
    pub async fn initiate_call(
        &self,
        to: &str,
        _prompt: Option<&str>,
    ) -> anyhow::Result<CallSession> {
        // 检查目标号码是否在允许列表中
        if !self.is_destination_allowed(to) {
            anyhow::bail!("Destination {} is not in allowed list", to);
        }

        // 构建呼叫请求，启用高级语音信箱检测
        let request = CallRequest {
            connection_id: self.connection_id.clone(),
            to: to.to_string(),
            from: self.from_number.clone(),
            answering_machine_detection: Some(AnsweringMachineDetection {
                mode: "premium".to_string(),
            }),
            webhook_url: None,
            command_id: None,
        };

        // 向 Telnyx API 发起呼叫请求
        let response = self
            .client
            .post(format!("{}/calls", self.telnyx_api_url()))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        // 检查 API 响应状态
        if !response.status().is_success() {
            let error = response.text().await?;
            anyhow::bail!("Failed to initiate call: {}", error);
        }

        // 解析响应并返回通话会话信息
        let call_response: CallResponse = response.json().await?;

        Ok(CallSession {
            call_control_id: call_response.call_control_id,
            call_leg_id: call_response.call_leg_id,
            call_session_id: call_response.call_session_id,
        })
    }

    /// 向活动通话发送文本转语音（TTS）音频
    ///
    /// # 参数
    ///
    /// - `call_control_id`: 通话控制 ID，由 `initiate_call` 返回
    /// - `text`: 要转换为语音的文本内容
    ///
    /// # 返回值
    ///
    /// 成功时返回 `Ok(())`，失败时返回错误
    ///
    /// # 语音设置
    ///
    /// - 使用高级（premium）服务质量
    /// - 默认女声、美式英语
    ///
    /// # 示例
    ///
    /// ```ignore
    /// channel.speak(&session.call_control_id, "Hello, this is a test.").await?;
    /// ```
    pub async fn speak(&self, call_control_id: &str, text: &str) -> anyhow::Result<()> {
        // 构建 TTS 请求
        let request = SpeakRequest {
            payload: text.to_string(),
            payload_type: "text".to_string(),
            service_level: "premium".to_string(),
            voice: "female".to_string(),
            language: "en-US".to_string(),
        };

        // 调用 Telnyx speak API 端点
        let response = self
            .client
            .post(format!("{}/calls/{}/actions/speak", self.telnyx_api_url(), call_control_id))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        // 检查 API 响应状态
        if !response.status().is_success() {
            let error = response.text().await?;
            anyhow::bail!("Failed to speak: {}", error);
        }

        Ok(())
    }

    /// 挂断活动通话
    ///
    /// # 参数
    ///
    /// - `call_control_id`: 通话控制 ID，由 `initiate_call` 返回
    ///
    /// # 返回值
    ///
    /// 始终返回 `Ok(())`，即使挂断失败也只记录警告日志
    ///
    /// # 注意
    ///
    /// 此方法采用"尽力而为"策略，即使挂断失败也不会返回错误，
    /// 因为通话可能已经自然结束
    pub async fn hangup(&self, call_control_id: &str) -> anyhow::Result<()> {
        // 调用 Telnyx hangup API 端点
        let response = self
            .client
            .post(format!("{}/calls/{}/actions/hangup", self.telnyx_api_url(), call_control_id))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await?;

        // 挂断失败只记录警告，不返回错误
        if !response.status().is_success() {
            let error = response.text().await?;
            tracing::warn!("Failed to hangup call: {}", error);
        }

        Ok(())
    }

    /// 启动 AI 驱动的智能对话
    ///
    /// 使用 Telnyx 的 AI 推理功能，为通话提供智能对话能力。
    /// AI 会根据系统提示自动响应用户的语音输入。
    ///
    /// # 参数
    ///
    /// - `call_control_id`: 通话控制 ID，由 `initiate_call` 返回
    /// - `system_prompt`: 系统提示，定义 AI 的角色和行为
    /// - `model`: 使用的 AI 模型名称
    ///
    /// # 返回值
    ///
    /// 成功时返回 `Ok(())`，失败时返回错误
    ///
    /// # 示例
    ///
    /// ```ignore
    /// channel.start_ai_conversation(
    ///     &session.call_control_id,
    ///     "You are a helpful customer service assistant.",
    ///     "gpt-4"
    /// ).await?;
    /// ```
    pub async fn start_ai_conversation(
        &self,
        call_control_id: &str,
        system_prompt: &str,
        model: &str,
    ) -> anyhow::Result<()> {
        // 构建 AI 对话请求，包含系统提示和语音设置
        let request = AiConversationRequest {
            system_prompt: system_prompt.to_string(),
            model: model.to_string(),
            voice_settings: VoiceSettings { voice: "alloy".to_string(), speed: 1.0 },
        };

        // 调用 Telnyx AI conversation API 端点
        let response = self
            .client
            .post(format!(
                "{}/calls/{}/actions/ai_conversation",
                self.telnyx_api_url(),
                call_control_id
            ))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        // 检查 API 响应状态
        if !response.status().is_success() {
            let error = response.text().await?;
            anyhow::bail!("Failed to start AI conversation: {}", error);
        }

        Ok(())
    }
}

/// 活动通话会话信息
///
/// 包含 Telnyx 返回的通话控制信息，用于后续的通话操作。
#[derive(Debug, Clone)]
pub struct CallSession {
    /// 通话控制 ID，用于执行通话操作（如挂断、播放音频等）
    pub call_control_id: String,
    /// 通话腿 ID，标识特定的通话端点
    pub call_leg_id: String,
    /// 通话会话 ID，标识整个通话会话
    pub call_session_id: String,
}

/// Telnyx 呼叫发起请求结构
///
/// 用于向 Telnyx API 发送呼叫请求的内部数据结构。
#[derive(Debug, Serialize)]
struct CallRequest {
    /// SIP 连接 ID
    connection_id: String,
    /// 目标号码（E.164 格式）
    to: String,
    /// 主叫号码（E.164 格式）
    from: String,
    /// 语音信箱检测配置
    #[serde(skip_serializing_if = "Option::is_none")]
    answering_machine_detection: Option<AnsweringMachineDetection>,
    /// Webhook 回调 URL（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    webhook_url: Option<String>,
    /// 命令 ID（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    command_id: Option<String>,
}

/// 语音信箱检测配置
///
/// 配置 Telnyx 的语音信箱检测功能。
#[derive(Debug, Serialize)]
struct AnsweringMachineDetection {
    /// 检测模式："premium" 表示使用高级检测算法
    mode: String,
}

/// Telnyx 呼叫响应结构
///
/// 从 Telnyx API 接收的呼叫响应数据。
#[derive(Debug, Deserialize)]
struct CallResponse {
    /// 通话控制 ID
    call_control_id: String,
    /// 通话腿 ID
    call_leg_id: String,
    /// 通话会话 ID
    call_session_id: String,
}

/// TTS 语音合成请求结构
///
/// 用于向 Telnyx API 发送文本转语音请求。
#[derive(Debug, Serialize)]
struct SpeakRequest {
    /// 要播放的文本内容
    payload: String,
    /// 载荷类型："text" 表示文本
    payload_type: String,
    /// 服务级别："premium" 表示高质量服务
    service_level: String,
    /// 语音类型："female" 或 "male"
    voice: String,
    /// 语言代码（如 "en-US"）
    language: String,
}

/// AI 对话请求结构
///
/// 用于启动 Telnyx AI 驱动的智能对话。
#[derive(Debug, Serialize)]
struct AiConversationRequest {
    /// 系统提示，定义 AI 的角色和行为
    system_prompt: String,
    /// AI 模型名称
    model: String,
    /// 语音设置
    voice_settings: VoiceSettings,
}

/// 语音设置配置
///
/// 配置 AI 对话的语音输出参数。
#[derive(Debug, Serialize)]
struct VoiceSettings {
    /// 语音类型（如 "alloy"）
    voice: String,
    /// 语速倍率（1.0 为正常速度）
    speed: f32,
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Channel for ClawdTalkChannel {
    /// 返回通道名称
    fn name(&self) -> &str {
        "ClawdTalk"
    }

    /// 发送消息（发起语音通话并播放消息）
    ///
    /// 对于 ClawdTalk，"发送"消息意味着：
    /// 1. 发起呼出电话到消息接收者
    /// 2. 等待电话接通
    /// 3. 使用 TTS 播放消息内容
    /// 4. 等待 TTS 完成后挂断电话
    ///
    /// # 参数
    ///
    /// - `message`: 要发送的消息，recipient 为目标号码，content 为要播放的文本
    ///
    /// # 返回值
    ///
    /// 成功时返回 `Ok(())`，失败时返回错误
    async fn send(&self, message: &SendMessage) -> anyhow::Result<()> {
        // 发起呼出电话
        let session = self.initiate_call(&message.recipient, None).await?;

        // 等待电话接通（2 秒）
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;

        // 使用 TTS 播放消息内容
        self.speak(&session.call_control_id, &message.content).await?;

        // 等待 TTS 播放完成（1 秒）
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        // 挂断电话
        self.hangup(&session.call_control_id).await?;

        Ok(())
    }

    /// 监听传入的消息（来电）
    ///
    /// ClawdTalk 通过 Webhook 接收来电通知，实际的来电处理由 gateway 模块完成。
    /// 此方法保持通道活跃，定期检查通道状态。
    ///
    /// # 参数
    ///
    /// - `tx`: 消息发送通道，用于向代理系统传递接收到的消息
    ///
    /// # 返回值
    ///
    /// 当通道关闭时返回 `Ok(())`
    async fn listen(&self, tx: mpsc::Sender<ChannelMessage>) -> anyhow::Result<()> {
        // 记录通道已开始监听
        tracing::info!("ClawdTalk channel listening for incoming calls");

        // 保持监听器活跃，定期检查通道状态
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;

            // 检查通道是否仍然开启
            if tx.is_closed() {
                break;
            }
        }

        Ok(())
    }

    /// 执行健康检查
    ///
    /// 通过查询 Telnyx 电话号码配置来验证 API 密钥的有效性。
    ///
    /// # 返回值
    ///
    /// 如果 API 密钥有效且能成功访问 Telnyx API，返回 `true`，否则返回 `false`
    async fn health_check(&self) -> bool {
        // 查询 Telnyx 电话号码列表以验证 API 密钥
        let response = self
            .client
            .get(format!("{}/phone_numbers", self.telnyx_api_url()))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await;

        match response {
            Ok(resp) => resp.status().is_success(),
            Err(e) => {
                tracing::warn!("ClawdTalk health check failed: {}", e);
                false
            }
        }
    }
}

/// Telnyx Webhook 事件结构
///
/// 接收来自 Telnyx 的来电 Webhook 回调。
/// 这些事件用于处理传入的电话呼叫。
#[derive(Debug, Deserialize)]
pub struct TelnyxWebhookEvent {
    /// 事件数据载荷
    pub data: TelnyxWebhookData,
}

/// Telnyx Webhook 数据载荷
#[derive(Debug, Deserialize)]
pub struct TelnyxWebhookData {
    /// 事件类型（如 "call.initiated", "call.answered" 等）
    pub event_type: String,
    /// 通话详细信息
    pub payload: TelnyxCallPayload,
}

/// Telnyx 通话载荷详细信息
///
/// 包含通话的各种状态和标识信息。
#[derive(Debug, Deserialize)]
pub struct TelnyxCallPayload {
    /// 通话控制 ID（可选）
    pub call_control_id: Option<String>,
    /// 通话腿 ID（可选）
    pub call_leg_id: Option<String>,
    /// 通话会话 ID（可选）
    pub call_session_id: Option<String>,
    /// 通话方向："inbound"（呼入）或 "outbound"（呼出）
    pub direction: Option<String>,
    /// 主叫号码
    pub from: Option<String>,
    /// 被叫号码
    pub to: Option<String>,
    /// 通话状态（如 "ringing", "answered", "hangup" 等）
    pub state: Option<String>,
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
