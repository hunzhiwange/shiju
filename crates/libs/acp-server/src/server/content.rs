//! ACP 提示内容到协议中立后端内容的安全转换。

use agent_client_protocol::{ContentBlock, Error, Result};

/// 后端可消费的基础提示片段。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromptContent {
    /// 原始文本片段。
    Text(String),
    /// 仅传递引用元数据，不读取 URI。
    ResourceLink {
        /// 展示名称。
        name: String,
        /// 未解析、未访问的资源 URI。
        uri: String,
        /// 可选 MIME 类型。
        mime_type: Option<String>,
    },
}

/// 按输入顺序转换 ACP v1 基础内容，并显式拒绝未声明能力。
pub fn prompt_content(blocks: Vec<ContentBlock>) -> Result<Vec<PromptContent>> {
    blocks
        .into_iter()
        .map(|block| match block {
            ContentBlock::Text(text) => Ok(PromptContent::Text(text.text)),
            ContentBlock::ResourceLink(link) => Ok(PromptContent::ResourceLink {
                name: link.name,
                uri: link.uri,
                mime_type: link.mime_type,
            }),
            ContentBlock::Image(_) => Err(unsupported_content("image")),
            ContentBlock::Audio(_) => Err(unsupported_content("audio")),
            ContentBlock::Resource(_) => Err(unsupported_content("embedded resource")),
            _ => Err(unsupported_content("unknown")),
        })
        .collect()
}

fn unsupported_content(kind: &str) -> Error {
    Error::invalid_params().data(serde_json::json!({
        "reason": format!("{kind} prompt content is not advertised by this Agent")
    }))
}
