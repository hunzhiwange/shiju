//! MCP 引导请求的表单、URL 确认与响应模型。
//!
//! 响应构造在此完成最小安全校验：必填字段不得为空，URL 仅接受 HTTP(S)
//! 且必须经过用户显式确认。

use std::collections::BTreeMap;

/// 表单引导中的一个可编辑字段。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ElicitationField {
    pub(crate) name: String,
    pub(crate) label: String,
    pub(crate) required: bool,
    pub(crate) value: String,
}

/// 引导请求的交互形式。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ElicitationKind {
    Form { fields: Vec<ElicitationField> },
    Url { url: String, confirmed: bool },
}

/// 一条待回复的服务器引导请求。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ElicitationRequest {
    pub(crate) id: String,
    pub(crate) server_id: String,
    pub(crate) kind: ElicitationKind,
}

/// 发送给引导服务器的用户决定。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ElicitationResponse {
    Accept { values: BTreeMap<String, String> },
    Reject,
}

impl ElicitationRequest {
    /// 根据用户决定构造响应；接受路径会验证必填值与 URL 安全边界。
    pub(crate) fn response(&self, accept: bool) -> Result<ElicitationResponse, String> {
        if !accept {
            return Ok(ElicitationResponse::Reject);
        }
        match &self.kind {
            ElicitationKind::Form { fields } => {
                let mut values = BTreeMap::new();
                for field in fields {
                    if field.required && field.value.trim().is_empty() {
                        return Err(format!("required elicitation field is empty: {}", field.name));
                    }
                    values.insert(field.name.clone(), field.value.clone());
                }
                Ok(ElicitationResponse::Accept { values })
            }
            ElicitationKind::Url { url, confirmed } => {
                // 先限制协议，再检查显式确认，避免将任意 URI 当作可安全打开的链接。
                if !is_safe_url(url) {
                    return Err("elicitation URL must use http or https".to_string());
                }
                if !confirmed {
                    return Err("elicitation URL requires explicit user confirmation".to_string());
                }
                Ok(ElicitationResponse::Accept { values: BTreeMap::new() })
            }
        }
    }
}

fn is_safe_url(url: &str) -> bool {
    url.starts_with("https://") || url.starts_with("http://")
}
