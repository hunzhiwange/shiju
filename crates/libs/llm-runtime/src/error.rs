use message_types as message;
#[derive(Debug)]
pub enum Error {
    /// 提供商未找到错误，包含未找到的提供商标识符
    ProviderNotFound(String),
    /// HTTP 请求错误，封装 reqwest 错误
    Http(reqwest::Error),
    /// 请求被中断（用户取消或超时）
    Aborted,
    /// API 错误，包含助手级别的错误详情
    Api(message::AssistantError),
}

/// 实现 Display trait 以提供人类可读的错误信息
impl std::fmt::Display for Error {
    /// 格式化错误信息
    ///
    /// # 参数
    /// - `f` - 格式化器
    ///
    /// # 返回值
    /// 格式化结果，成功返回 Ok(())
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            // 提供商未找到：显示提供商标识符
            Error::ProviderNotFound(id) => write!(f, "provider not found: {}", id),
            // HTTP 错误：直接显示原始错误信息
            Error::Http(e) => write!(f, "{}", e),
            // 中断错误：显示简短的标识
            Error::Aborted => write!(f, "aborted"),
            // API 错误：尝试序列化为 JSON，失败则显示占位符
            Error::Api(message::AssistantError::Unknown { message }) => {
                write!(f, "{}", message.strip_prefix("acp: ").unwrap_or(message))
            }
            Error::Api(e) => {
                write!(f, "{}", serde_json::to_string(e).unwrap_or_else(|_| "?".to_string()))
            }
        }
    }
}

/// 实现 Error trait 以支持标准错误处理
impl std::error::Error for Error {}

/// 实现 From trait 以自动转换 reqwest::Error
impl From<reqwest::Error> for Error {
    /// 将 reqwest 错误转换为 LLM 错误
    ///
    /// # 参数
    /// - `value` - reqwest 错误实例
    ///
    /// # 返回值
    /// 转换后的 Error::Http 变体
    fn from(value: reqwest::Error) -> Self {
        Error::Http(value)
    }
}
