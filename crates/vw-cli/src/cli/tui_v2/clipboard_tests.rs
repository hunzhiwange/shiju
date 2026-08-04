use std::io::{self, Write};

use super::clipboard::{osc52_sequence, write_osc52};
use super::editor::{EditorSelection, selected_text};
use super::model::PromptState;

struct FailingWriter;

impl Write for FailingWriter {
    fn write(&mut self, _buffer: &[u8]) -> io::Result<usize> {
        Err(io::Error::new(io::ErrorKind::BrokenPipe, "writer closed"))
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn osc52_encodes_unicode_newlines_and_reports_writer_failures() {
    let text = "中文🙂\nnext";
    let expected = "\u{1b}]52;c;5Lit5paH8J+ZggpuZXh0\u{7}";
    assert_eq!(osc52_sequence(text), expected);

    let mut output = Vec::new();
    write_osc52(&mut output, text).expect("memory writer");
    assert_eq!(output, expected.as_bytes());

    let error = write_osc52(&mut FailingWriter, text).expect_err("writer failure");
    assert_eq!(error.kind(), io::ErrorKind::BrokenPipe);
}

#[test]
fn selected_text_uses_grapheme_ranges_and_rejects_empty_or_invalid_ranges() {
    let mut prompt = PromptState::new("中e\u{301}🙂\n下一行");
    prompt.editor.selection = Some(EditorSelection { anchor: 6, active: 1 });
    assert_eq!(selected_text(&prompt).as_deref(), Some("e\u{301}🙂\n下一"));

    prompt.editor.selection = Some(EditorSelection { anchor: 2, active: 2 });
    assert_eq!(selected_text(&prompt), None);

    prompt.editor.selection = Some(EditorSelection { anchor: 1, active: usize::MAX });
    assert_eq!(selected_text(&prompt), None);
}
