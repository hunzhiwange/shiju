use std::io;

use super::model::{PromptMode, PromptState};
use super::paste::{
    ClipboardImage, ClipboardImageSource, PasteAccumulator, PromptAttachmentKind,
    PromptContentBlock, apply_prompt_editor_result, attach_clipboard_image,
    create_prompt_editor_intent, delete_focused_attachment, move_attachment_focus,
    prompt_content_blocks,
};

struct MockClipboard(Result<Option<ClipboardImage>, String>);

impl ClipboardImageSource for MockClipboard {
    fn image(&mut self) -> Result<Option<ClipboardImage>, String> {
        self.0.clone()
    }
}

#[test]
fn tui_paste_attachments_vim_and_external_editor_match_reference() {
    let mut prompt = PromptState::default();
    let mut paste = PasteAccumulator::default();
    paste.push("一\r\n二\r");
    paste.push("三\n四\n五\n六\n七\n八");
    paste.finish(&mut prompt);
    assert_eq!(prompt.value, "[Pasted text #1]");
    assert_eq!(prompt.attachments[0].kind, PromptAttachmentKind::PastedText);
    assert_eq!(
        prompt.attachments[0].text.as_deref(),
        Some("一\n二\n三\n四\n五\n六\n七\n八")
    );

    let mut clipboard = MockClipboard(Ok(Some(ClipboardImage {
        media_type: "image/png".to_string(),
        bytes: vec![1, 2, 3],
    })));
    assert_eq!(attach_clipboard_image(&mut prompt, &mut clipboard), Ok(true));
    assert_eq!(prompt.value, "[Pasted text #1][Image #2]");
    move_attachment_focus(&mut prompt, -1);
    assert_eq!(prompt.focused_attachment, Some(0));
    assert!(delete_focused_attachment(&mut prompt));
    assert_eq!(prompt.value, "[Image #2]");
    assert!(matches!(
        prompt_content_blocks(&prompt).as_slice(),
        [PromptContentBlock::Text(_), PromptContentBlock::Image { .. }]
    ));

    prompt.set_value("!pwd");
    assert_eq!(prompt.mode, PromptMode::Bash);
    prompt.set_value(" !pwd");
    assert_eq!(prompt.mode, PromptMode::Compose);

    prompt.set_value("draft");
    let intent = create_prompt_editor_intent(&prompt).expect("editor temp file");
    assert_eq!(std::fs::read_to_string(&intent.path).unwrap(), "draft");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        assert_eq!(std::fs::metadata(&intent.path).unwrap().permissions().mode() & 0o777, 0o600);
    }
    prompt.set_value("mutated");
    let result = apply_prompt_editor_result(
        &mut prompt,
        &intent,
        Err(io::Error::other("editor failed")),
    );
    assert!(result.is_err());
    assert_eq!(prompt.value, "draft");
    std::fs::remove_file(intent.path).unwrap();
}

#[test]
fn clipboard_image_boundary_rejects_empty_unknown_and_oversized_images() {
    let mut prompt = PromptState::default();
    let mut empty = MockClipboard(Ok(None));
    assert_eq!(attach_clipboard_image(&mut prompt, &mut empty), Ok(false));

    let mut unknown = MockClipboard(Ok(Some(ClipboardImage {
        media_type: "image/tiff".to_string(),
        bytes: vec![1],
    })));
    assert!(attach_clipboard_image(&mut prompt, &mut unknown).is_err());

    let mut oversized = MockClipboard(Ok(Some(ClipboardImage {
        media_type: "image/png".to_string(),
        bytes: vec![0; 10 * 1024 * 1024 + 1],
    })));
    assert!(attach_clipboard_image(&mut prompt, &mut oversized).is_err());
    assert!(prompt.attachments.is_empty());
}
