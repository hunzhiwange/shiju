use std::fs::OpenOptions;
use std::io::Write as _;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use super::editor::{EditorCommand, apply_editor_command};
use super::model::PromptState;
use vw_shared::pasted_text::{cache_large_pasted_text, document_reference};

const MAX_IMAGE_BYTES: usize = 10 * 1024 * 1024;

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PromptContentBlock {
    Text(String),
    Image { media_type: String, bytes: Vec<u8> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PromptEditorIntent {
    pub(crate) path: PathBuf,
    pub(crate) original_draft: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct PasteAccumulator {
    chunks: Vec<String>,
}

impl PasteAccumulator {
    pub(crate) fn push(&mut self, chunk: impl Into<String>) {
        self.chunks.push(chunk.into());
    }

    pub(crate) fn finish(self, prompt: &mut PromptState) {
        apply_terminal_paste(prompt, self.chunks.concat().as_str());
    }
}

pub(crate) trait ClipboardImageSource {
    fn image(&mut self) -> Result<Option<ClipboardImage>, String>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ClipboardImage {
    pub(crate) media_type: String,
    pub(crate) bytes: Vec<u8>,
}

pub(crate) fn apply_terminal_paste(prompt: &mut PromptState, text: &str) {
    let text = text.replace("\r\n", "\n").replace('\r', "\n");
    if let Ok(Some(path)) = cache_large_pasted_text(&text) {
        apply_editor_command(prompt, EditorCommand::Insert(document_reference(&path)));
    } else {
        apply_editor_command(prompt, EditorCommand::PasteStart);
        apply_editor_command(prompt, EditorCommand::Insert(text));
        apply_editor_command(prompt, EditorCommand::PasteEnd);
    }
}

pub(crate) fn attach_clipboard_image(
    prompt: &mut PromptState,
    source: &mut dyn ClipboardImageSource,
) -> Result<bool, String> {
    let Some(image) = source.image()? else {
        return Ok(false);
    };
    if image.bytes.is_empty() {
        return Ok(false);
    }
    if image.bytes.len() > MAX_IMAGE_BYTES {
        return Err("clipboard image exceeds 10 MiB".to_string());
    }
    if !matches!(image.media_type.as_str(), "image/png" | "image/jpeg" | "image/gif" | "image/webp")
    {
        return Err(format!("unsupported clipboard image type: {}", image.media_type));
    }
    let id = next_attachment_id(prompt);
    let label = format!("[Image #{id}]");
    prompt.attachments.push(PromptAttachment {
        id,
        kind: PromptAttachmentKind::Image,
        label: label.clone(),
        media_type: Some(image.media_type),
        text: None,
        bytes: image.bytes,
    });
    apply_editor_command(prompt, EditorCommand::Insert(label));
    prompt.focused_attachment = Some(prompt.attachments.len().saturating_sub(1));
    Ok(true)
}

pub(crate) fn move_attachment_focus(prompt: &mut PromptState, delta: isize) {
    if prompt.attachments.is_empty() {
        prompt.focused_attachment = None;
        return;
    }
    let current = prompt.focused_attachment.unwrap_or_default();
    prompt.focused_attachment = Some(if delta < 0 {
        current.saturating_sub(delta.unsigned_abs())
    } else {
        current.saturating_add(delta.cast_unsigned()).min(prompt.attachments.len() - 1)
    });
}

pub(crate) fn delete_focused_attachment(prompt: &mut PromptState) -> bool {
    let Some(index) = prompt.focused_attachment else {
        return false;
    };
    if index >= prompt.attachments.len() {
        prompt.focused_attachment = None;
        return false;
    }
    let attachment = prompt.attachments.remove(index);
    prompt.value = prompt.value.replacen(attachment.label.as_str(), "", 1);
    prompt.cursor = super::model::prompt::PromptCursor::at_end(&prompt.value);
    prompt.focused_attachment = if prompt.attachments.is_empty() {
        None
    } else {
        Some(index.min(prompt.attachments.len() - 1))
    };
    true
}

pub(crate) fn prompt_content_blocks(prompt: &PromptState) -> Vec<PromptContentBlock> {
    let mut blocks = vec![PromptContentBlock::Text(prompt.value.clone())];
    blocks.extend(prompt.attachments.iter().filter_map(|attachment| match attachment.kind {
        PromptAttachmentKind::PastedText => attachment.text.clone().map(PromptContentBlock::Text),
        PromptAttachmentKind::Image => Some(PromptContentBlock::Image {
            media_type: attachment.media_type.clone().unwrap_or_else(|| "image/png".to_string()),
            bytes: attachment.bytes.clone(),
        }),
    }));
    blocks
}

pub(crate) fn create_prompt_editor_intent(
    prompt: &PromptState,
) -> std::io::Result<PromptEditorIntent> {
    let nonce = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
    let path =
        std::env::temp_dir().join(format!("vibewindow-prompt-{}-{nonce}.txt", std::process::id()));
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.mode(0o600);
    }
    let mut file = options.open(&path)?;
    file.write_all(prompt.value.as_bytes())?;
    Ok(PromptEditorIntent { path, original_draft: prompt.value.clone() })
}

pub(crate) fn apply_prompt_editor_result(
    prompt: &mut PromptState,
    intent: &PromptEditorIntent,
    edited: std::io::Result<String>,
) -> std::io::Result<()> {
    match edited {
        Ok(value) => {
            prompt.set_value(value);
            Ok(())
        }
        Err(error) => {
            prompt.set_value(intent.original_draft.clone());
            Err(error)
        }
    }
}

fn next_attachment_id(prompt: &mut PromptState) -> usize {
    prompt.next_attachment_id = prompt.next_attachment_id.saturating_add(1).max(1);
    prompt.next_attachment_id
}
