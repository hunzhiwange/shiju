//! 定义 `/status` 账户区域使用的只读展示模型。
//! 该模型只区分聊天账户与 API 密钥认证，不承担认证或凭据读取职责。

/// 已经脱敏并可直接交给状态界面渲染的账户信息。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum StatusAccountDisplay {
    /// ChatGPT 账户可选展示邮箱与套餐名称。
    ChatGpt {
        email: Option<String>,
        plan: Option<String>,
    },
    /// API 密钥模式仅展示认证类型，绝不携带原始密钥。
    ApiKey,
}
