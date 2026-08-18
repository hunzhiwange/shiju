use std::io;

use super::model::{PromptMode, PromptState};
use super::paste::{
    ClipboardImage, ClipboardImageSource, PasteAccumulator, PromptContentBlock,
    apply_prompt_editor_result, apply_terminal_paste, attach_clipboard_image,
    create_prompt_editor_intent, delete_focused_attachment, prompt_content_blocks,
};

struct MockClipboard(Result<Option<ClipboardImage>, String>);

impl ClipboardImageSource for MockClipboard {
    fn image(&mut self) -> Result<Option<ClipboardImage>, String> {
        self.0.clone()
    }
}

#[test]
fn tui_paste_file_references_vim_and_external_editor_match_reference() {
    let mut prompt = PromptState::default();
    let mut paste = PasteAccumulator::default();
    paste.push("短文本");
    paste.finish(&mut prompt);
    assert_eq!(prompt.value, "短文本");

    let large_text = "一".repeat(501);
    apply_terminal_paste(&mut prompt, &large_text);
    let path = prompt
        .value
        .strip_prefix("短文本[DOCUMENT:")
        .and_then(|value| value.strip_suffix(']'))
        .expect("large paste should insert a document reference");
    assert_eq!(std::fs::read_to_string(path).unwrap(), large_text);
    std::fs::remove_file(path).unwrap();

    let mut clipboard = MockClipboard(Ok(Some(ClipboardImage {
        media_type: "image/png".to_string(),
        bytes: vec![1, 2, 3],
    })));
    assert_eq!(attach_clipboard_image(&mut prompt, &mut clipboard), Ok(true));
    assert!(prompt.value.ends_with("[Image #1]"));
    assert_eq!(prompt.focused_attachment, Some(0));
    assert!(delete_focused_attachment(&mut prompt));
    assert!(!prompt.value.contains("[Image #1]"));
    assert!(matches!(prompt_content_blocks(&prompt).as_slice(), [PromptContentBlock::Text(_)]));

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
    let result =
        apply_prompt_editor_result(&mut prompt, &intent, Err(io::Error::other("editor failed")));
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
