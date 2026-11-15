//! prompt 提交保留的附件元数据。
//!
//! 该模型只承载粘贴文本或图片内容，不负责读取剪贴板、解码媒体或上传数据。

/// prompt 附件的内容类别。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptAttachmentKind {
    File,
    PastedText,
    Image,
    Resource,
}

/// 一条待随 prompt 提交的附件快照，文本与原始字节按类别取用。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptAttachment {
    pub id: usize,
    pub kind: PromptAttachmentKind,
    pub label: String,
    pub media_type: Option<String>,
    pub uri: Option<String>,
    pub text: Option<String>,
    pub bytes: Vec<u8>,
}
