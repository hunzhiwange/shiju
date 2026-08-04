//! 消息附件视图模型。

use super::UiMessageBase;

/// 终端中可见的附件类别。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UiAttachmentKind {
    Image,
    File,
    Link,
    PastedText,
    Resource,
}

/// 一条独立附件消息。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UiAttachment {
    pub(crate) base: UiMessageBase,
    pub(crate) kind: UiAttachmentKind,
    pub(crate) name: String,
    pub(crate) media_type: Option<String>,
    pub(crate) uri: String,
    pub(crate) alt_text: Option<String>,
    pub(crate) width_px: Option<u32>,
    pub(crate) height_px: Option<u32>,
    pub(crate) size_bytes: Option<u64>,
    pub(crate) source: Option<String>,
    pub(crate) preview: Option<String>,
}

impl UiAttachment {
    pub(crate) fn new(
        base: UiMessageBase,
        kind: UiAttachmentKind,
        name: impl Into<String>,
        uri: impl Into<String>,
    ) -> Self {
        Self {
            base,
            kind,
            name: name.into(),
            media_type: None,
            uri: uri.into(),
            alt_text: None,
            width_px: None,
            height_px: None,
            size_bytes: None,
            source: None,
            preview: None,
        }
    }

    pub(crate) fn with_media_type(mut self, media_type: impl Into<String>) -> Self {
        self.media_type = Some(media_type.into());
        self
    }

    pub(crate) fn with_dimensions(mut self, width_px: u32, height_px: u32) -> Self {
        self.width_px = Some(width_px);
        self.height_px = Some(height_px);
        self
    }

    pub(crate) fn with_size_bytes(mut self, size_bytes: u64) -> Self {
        self.size_bytes = Some(size_bytes);
        self
    }

    pub(crate) fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }

    pub(crate) fn with_preview(mut self, preview: impl Into<String>) -> Self {
        self.preview = Some(preview.into());
        self
    }
}
