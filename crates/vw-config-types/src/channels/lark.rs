//! 定义 Lark 渠道配置类型。
//! 配置结构保持可序列化且边界清晰，避免渠道实现直接暴露到通用配置层。

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::types::{
    GroupReplyConfig, GroupReplyMode, clone_group_reply_allowed_sender_ids,
    resolve_group_reply_mode,
};

/// LarkReceiveMode 描述该模块对外暴露的离散状态。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum LarkReceiveMode {
    #[default]
    Websocket,
    Webhook,
}

/// 执行 default_lark_draft_update_interval_ms 对应的模块功能。
///
/// 参数含义保持与函数签名一致；返回值用于调用方继续组合处理，错误由返回类型显式表达。
pub fn default_lark_draft_update_interval_ms() -> u64 {
    3000
}

/// 执行 default_lark_max_draft_edits 对应的模块功能。
///
/// 参数含义保持与函数签名一致；返回值用于调用方继续组合处理，错误由返回类型显式表达。
pub fn default_lark_max_draft_edits() -> u32 {
    20
}

/// LarkConfig 表示该模块对外暴露的结构化状态。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LarkConfig {
    /// app_id 字段保存该结构体对外暴露的同名状态。
    pub app_id: String,
    /// app_secret 字段保存该结构体对外暴露的同名状态。
    pub app_secret: String,
    /// encrypt_key 字段保存该结构体对外暴露的同名状态。
    #[serde(default)]
    pub encrypt_key: Option<String>,
    /// verification_token 字段保存该结构体对外暴露的同名状态。
    #[serde(default)]
    pub verification_token: Option<String>,
    /// allowed_users 字段保存该结构体对外暴露的同名状态。
    #[serde(default)]
    pub allowed_users: Vec<String>,
    /// mention_only 字段保存该结构体对外暴露的同名状态。
    #[serde(default)]
    pub mention_only: bool,
    /// group_reply 字段保存该结构体对外暴露的同名状态。
    #[serde(default)]
    pub group_reply: Option<GroupReplyConfig>,
    /// use_feishu 字段保存该结构体对外暴露的同名状态。
    #[serde(default)]
    pub use_feishu: bool,
    /// receive_mode 字段保存该结构体对外暴露的同名状态。
    #[serde(default)]
    pub receive_mode: LarkReceiveMode,
    /// port 字段保存该结构体对外暴露的同名状态。
    #[serde(default)]
    pub port: Option<u16>,
    /// draft_update_interval_ms 字段保存该结构体对外暴露的同名状态。
    #[serde(default = "default_lark_draft_update_interval_ms")]
    pub draft_update_interval_ms: u64,
    /// max_draft_edits 字段保存该结构体对外暴露的同名状态。
    #[serde(default = "default_lark_max_draft_edits")]
    pub max_draft_edits: u32,
}

impl LarkConfig {
    #[must_use]
    pub fn effective_group_reply_mode(&self) -> GroupReplyMode {
        resolve_group_reply_mode(
            self.group_reply.as_ref(),
            Some(self.mention_only),
            GroupReplyMode::AllMessages,
        )
    }

    #[must_use]
    pub fn group_reply_allowed_sender_ids(&self) -> Vec<String> {
        clone_group_reply_allowed_sender_ids(self.group_reply.as_ref())
    }
}

/// FeishuConfig 表示该模块对外暴露的结构化状态。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FeishuConfig {
    /// app_id 字段保存该结构体对外暴露的同名状态。
    pub app_id: String,
    /// app_secret 字段保存该结构体对外暴露的同名状态。
    pub app_secret: String,
    /// encrypt_key 字段保存该结构体对外暴露的同名状态。
    #[serde(default)]
    pub encrypt_key: Option<String>,
    /// verification_token 字段保存该结构体对外暴露的同名状态。
    #[serde(default)]
    pub verification_token: Option<String>,
    /// allowed_users 字段保存该结构体对外暴露的同名状态。
    #[serde(default)]
    pub allowed_users: Vec<String>,
    /// group_reply 字段保存该结构体对外暴露的同名状态。
    #[serde(default)]
    pub group_reply: Option<GroupReplyConfig>,
    /// receive_mode 字段保存该结构体对外暴露的同名状态。
    #[serde(default)]
    pub receive_mode: LarkReceiveMode,
    /// port 字段保存该结构体对外暴露的同名状态。
    #[serde(default)]
    pub port: Option<u16>,
    /// draft_update_interval_ms 字段保存该结构体对外暴露的同名状态。
    #[serde(default = "default_lark_draft_update_interval_ms")]
    pub draft_update_interval_ms: u64,
    /// max_draft_edits 字段保存该结构体对外暴露的同名状态。
    #[serde(default = "default_lark_max_draft_edits")]
    pub max_draft_edits: u32,
}

/// 命名 Feishu 通道实例配置。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FeishuInstanceConfig {
    /// 稳定实例 ID，用于员工绑定和 profile ref。
    pub id: String,
    /// Feishu 通道原有配置字段。
    #[serde(flatten)]
    pub config: FeishuConfig,
}

impl FeishuConfig {
    #[must_use]
    pub fn effective_group_reply_mode(&self) -> GroupReplyMode {
        resolve_group_reply_mode(self.group_reply.as_ref(), None, GroupReplyMode::AllMessages)
    }

    #[must_use]
    pub fn group_reply_allowed_sender_ids(&self) -> Vec<String> {
        clone_group_reply_allowed_sender_ids(self.group_reply.as_ref())
    }
}
#[cfg(test)]
#[path = "lark_tests.rs"]
mod lark_tests;
