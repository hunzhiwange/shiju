//! 处理聊天输入区的局部消息。
//! 本模块将编辑器操作、文件检索和工具细节限制在输入面板边界内。

use super::super::{ChatMessage, ClipboardPastePayload};
use crate::app::Message;
#[cfg(not(target_arch = "wasm32"))]
use crate::app::message::project::helpers::persist_clipboard_image_attachment;
use iced::Task;

/// 模块内可见函数，执行 read_clipboard_for_input 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn read_clipboard_for_input() -> Task<Message> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        Task::perform(async move { read_native_clipboard_payload() }, resolved_clipboard_message)
    }

    #[cfg(target_arch = "wasm32")]
    {
        iced::clipboard::read()
            .map(|content| resolved_clipboard_message(clipboard_text_payload(content)))
    }
}

fn resolved_clipboard_message(payload: ClipboardPastePayload) -> Message {
    Message::Chat(ChatMessage::ClipboardPasteResolved(payload))
}

fn clipboard_text_payload(content: Option<String>) -> ClipboardPastePayload {
    match content {
        Some(content) if !content.is_empty() => {
            match vw_shared::pasted_text::cache_large_pasted_text(&content) {
                Ok(Some(path)) => {
                    ClipboardPastePayload::AttachmentPath(path.to_string_lossy().into_owned())
                }
                Ok(None) => ClipboardPastePayload::Text(content),
                Err(error) => ClipboardPastePayload::Error(format!("缓存粘贴文本失败：{error}")),
            }
        }
        _ => ClipboardPastePayload::Empty,
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn clipboard_image_dimensions(width: usize, height: usize) -> Result<(u32, u32), &'static str> {
    let width = u32::try_from(width).map_err(|_| "剪贴板图片宽度超出范围")?;
    let height = u32::try_from(height).map_err(|_| "剪贴板图片高度超出范围")?;
    Ok((width, height))
}

#[cfg(not(target_arch = "wasm32"))]
fn clipboard_attachment_payload(result: Result<String, String>) -> ClipboardPastePayload {
    match result {
        Ok(path) => ClipboardPastePayload::AttachmentPath(path),
        Err(error) => ClipboardPastePayload::Error(error),
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn read_native_clipboard_payload() -> ClipboardPastePayload {
    let mut clipboard = match arboard::Clipboard::new() {
        Ok(clipboard) => clipboard,
        Err(error) => {
            return ClipboardPastePayload::Error(format!("读取系统剪贴板失败：{}", error));
        }
    };

    if let Ok(image) = clipboard.get_image() {
        let (width, height) = match clipboard_image_dimensions(image.width, image.height) {
            Ok(dimensions) => dimensions,
            Err(error) => return ClipboardPastePayload::Error(error.to_string()),
        };

        return clipboard_attachment_payload(persist_clipboard_image_attachment(
            width,
            height,
            image.bytes.as_ref(),
        ));
    }

    clipboard_text_payload(clipboard.get_text().ok())
}
#[cfg(test)]
#[path = "clipboard_tests.rs"]
mod clipboard_tests;
