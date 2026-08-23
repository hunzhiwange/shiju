//! Attachment metadata retained by prompt submissions.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PromptAttachmentKind {
    PastedText,
    Image,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PromptAttachment {
    pub(crate) id: usize,
    pub(crate) kind: PromptAttachmentKind,
    pub(crate) label: String,
    pub(crate) media_type: Option<String>,
    pub(crate) text: Option<String>,
    pub(crate) bytes: Vec<u8>,
}
