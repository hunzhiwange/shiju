//! 复制边界用例只替换系统后端与输出 writer，不访问用户剪贴板。

use super::write_osc52_to_writer;
use super::{CopyEnvironment, OSC52_MAX_RAW_BYTES, copy_to_clipboard_with, osc52_sequence};
use base64::Engine;
use std::cell::RefCell;
use std::io::{self, Write};

#[test]
fn terminal_req_10_unicode_code_roundtrips_with_and_without_tmux() {
    let text = "中文🙂\r\n```rust\nfn main() { println!(\"你好\"); }\n```\n";
    for tmux in [false, true] {
        let sequence = osc52_sequence(text, tmux).unwrap();
        let (prefix, suffix) =
            if tmux { ("\x1bPtmux;\x1b\x1b]52;c;", "\x07\x1b\\") } else { ("\x1b]52;c;", "\x07") };
        let encoded = sequence.strip_prefix(prefix).unwrap().strip_suffix(suffix).unwrap();
        let decoded = base64::engine::general_purpose::STANDARD.decode(encoded).unwrap();
        assert_eq!(decoded, text.as_bytes());
    }
}

#[test]
fn terminal_req_10_osc52_limit_counts_utf8_bytes() {
    let text = "🙂".repeat(OSC52_MAX_RAW_BYTES / 4);
    assert_eq!(text.len(), OSC52_MAX_RAW_BYTES);
    for tmux in [false, true] {
        assert!(osc52_sequence(&text, tmux).is_ok());
        let oversized = format!("{text}中");
        assert_eq!(
            osc52_sequence(&oversized, tmux),
            Err(format!(
                "OSC 52 payload too large ({} bytes; max {OSC52_MAX_RAW_BYTES})",
                oversized.len()
            ))
        );
    }
}

#[test]
fn terminal_req_10_unicode_is_preserved_across_environment_fallbacks() {
    let text = "中文🙂\n```rust\nfn main() {}\n```\n";
    // SSH must never write to the remote native clipboard, even inside WSL.
    for (environment, expected) in [
        (
            CopyEnvironment { ssh_session: true, wsl_session: true, tmux_session: false },
            vec!["osc"],
        ),
        (
            CopyEnvironment { ssh_session: true, wsl_session: true, tmux_session: true },
            vec!["tmux", "osc"],
        ),
        (
            CopyEnvironment { ssh_session: false, wsl_session: false, tmux_session: false },
            vec!["native", "osc"],
        ),
        (
            CopyEnvironment { ssh_session: false, wsl_session: true, tmux_session: true },
            vec!["native", "wsl", "tmux", "osc"],
        ),
    ] {
        let calls = RefCell::new(Vec::new());
        let result = copy_to_clipboard_with(
            text,
            environment,
            |received| {
                assert_eq!(received, text);
                calls.borrow_mut().push("tmux");
                Err("tmux unavailable".into())
            },
            |received| {
                assert_eq!(received, text);
                calls.borrow_mut().push("osc");
                Ok(())
            },
            |received| {
                assert_eq!(received, text);
                calls.borrow_mut().push("native");
                Err("native unavailable".into())
            },
            |received| {
                assert_eq!(received, text);
                calls.borrow_mut().push("wsl");
                Err("WSL unavailable".into())
            },
        );
        assert!(matches!(result, Ok(None)));
        assert_eq!(*calls.borrow(), expected);
    }
}

struct FailingWriter {
    fail_write: bool,
    bytes: Vec<u8>,
    flushed: bool,
}

impl Write for FailingWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self.fail_write {
            return Err(io::Error::new(io::ErrorKind::BrokenPipe, "closed"));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.flushed = true;
        Err(io::Error::new(io::ErrorKind::BrokenPipe, "closed"))
    }
}

#[test]
fn terminal_req_10_write_failure_does_not_flush() {
    let mut writer = FailingWriter { fail_write: true, bytes: Vec::new(), flushed: false };
    assert_eq!(
        write_osc52_to_writer(&mut writer, "sequence"),
        Err("failed to write OSC 52: closed".into())
    );
    assert!(writer.bytes.is_empty());
    assert!(!writer.flushed);
}

#[test]
fn terminal_req_10_flush_failure_is_reported_after_writing() {
    let mut writer = FailingWriter { fail_write: false, bytes: Vec::new(), flushed: false };
    assert_eq!(
        write_osc52_to_writer(&mut writer, "sequence"),
        Err("failed to flush OSC 52: closed".into())
    );
    assert_eq!(writer.bytes, b"sequence");
    assert!(writer.flushed);
}
