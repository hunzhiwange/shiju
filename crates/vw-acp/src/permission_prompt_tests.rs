use super::*;

#[cfg(unix)]
use std::os::fd::{FromRawFd, RawFd};

#[test]
fn prompt_for_permission_with_io_accepts_yes_and_writes_prompt_parts() {
    let mut reader = std::io::Cursor::new(b"  YeS  \n".to_vec());
    let mut output = Vec::new();

    let allowed = prompt_for_permission_with_io(
        &PermissionPromptOptions {
            prompt: "Allow terminal child?".to_string(),
            header: Some("Permission header".to_string()),
            details: Some("Permission details".to_string()),
        },
        &mut reader,
        &mut output,
    )
    .expect("prompt should read answer");

    let output = String::from_utf8(output).expect("prompt output should be utf8");
    assert!(allowed);
    assert!(output.contains("Permission header"));
    assert!(output.contains("Permission details"));
    assert!(output.contains("Allow terminal child?"));
}

#[test]
fn prompt_for_permission_with_io_rejects_non_yes_and_skips_blank_details() {
    let mut reader = std::io::Cursor::new(b"no\n".to_vec());
    let mut output = Vec::new();

    let allowed = prompt_for_permission_with_io(
        &PermissionPromptOptions {
            prompt: "Allow terminal child?".to_string(),
            header: None,
            details: Some("   \t  ".to_string()),
        },
        &mut reader,
        &mut output,
    )
    .expect("prompt should read answer");

    let output = String::from_utf8(output).expect("prompt output should be utf8");
    assert!(!allowed);
    assert!(output.contains("Allow terminal child?"));
    assert!(!output.contains("   \t  \n"));
}

#[test]
fn prompt_for_permission_with_io_accepts_short_y_without_optional_parts() {
    let mut reader = std::io::Cursor::new(b"y\n".to_vec());
    let mut output = Vec::new();

    let allowed = prompt_for_permission_with_io(
        &PermissionPromptOptions { prompt: "Allow?".to_string(), header: None, details: None },
        &mut reader,
        &mut output,
    )
    .expect("prompt should read answer");

    assert!(allowed);
    assert_eq!(String::from_utf8(output).expect("prompt output should be utf8"), "Allow?");
}

#[test]
fn prompt_for_permission_with_io_writes_expected_header_and_details_layout() {
    let mut reader = std::io::Cursor::new(b"yes\n".to_vec());
    let mut output = Vec::new();

    let allowed = prompt_for_permission_with_io(
        &PermissionPromptOptions {
            prompt: "Allow?".to_string(),
            header: Some("Header".to_string()),
            details: Some("Details".to_string()),
        },
        &mut reader,
        &mut output,
    )
    .expect("prompt should read answer");

    assert!(allowed);
    assert_eq!(
        String::from_utf8(output).expect("prompt output should be utf8"),
        "\nHeader\nDetails\nAllow?"
    );
}

#[test]
fn prompt_for_permission_with_io_rejects_empty_answer() {
    let mut reader = std::io::Cursor::new(Vec::new());
    let mut output = Vec::new();

    let allowed = prompt_for_permission_with_io(
        &PermissionPromptOptions { prompt: "Allow?".to_string(), header: None, details: None },
        &mut reader,
        &mut output,
    )
    .expect("prompt should handle eof");

    assert!(!allowed);
}

#[test]
fn prompt_for_permission_with_io_returns_reader_error() {
    let mut reader = FailingReader;
    let mut output = Vec::new();

    let err = prompt_for_permission_with_io(
        &PermissionPromptOptions { prompt: "Allow?".to_string(), header: None, details: None },
        &mut reader,
        &mut output,
    )
    .expect_err("reader error should be returned");

    assert_eq!(err.kind(), std::io::ErrorKind::Other);
}

#[test]
fn prompt_for_permission_with_io_returns_writer_error() {
    let mut reader = std::io::Cursor::new(b"yes\n".to_vec());
    let mut output = FailingWriter;

    let err = prompt_for_permission_with_io(
        &PermissionPromptOptions {
            prompt: "Allow?".to_string(),
            header: Some("Header".to_string()),
            details: None,
        },
        &mut reader,
        &mut output,
    )
    .expect_err("writer error should be returned");

    assert_eq!(err.kind(), std::io::ErrorKind::Other);
}

#[test]
fn prompt_for_permission_with_io_returns_flush_error() {
    let mut reader = std::io::Cursor::new(b"yes\n".to_vec());
    let mut output = FlushFailingWriter::default();

    let err = prompt_for_permission_with_io(
        &PermissionPromptOptions { prompt: "Allow?".to_string(), header: None, details: None },
        &mut reader,
        &mut output,
    )
    .expect_err("flush error should be returned");

    assert_eq!(err.kind(), std::io::ErrorKind::Other);
    assert_eq!(output.written, b"Allow?");
}

#[test]
fn prompt_for_permission_with_io_returns_prompt_write_error_after_optional_content() {
    let mut reader = std::io::Cursor::new(b"yes\n".to_vec());
    let mut output = PromptFailingWriter::default();

    let err = prompt_for_permission_with_io(
        &PermissionPromptOptions {
            prompt: "Allow?".to_string(),
            header: Some("Header".to_string()),
            details: Some("Details".to_string()),
        },
        &mut reader,
        &mut output,
    )
    .expect_err("prompt write error should be returned");

    assert_eq!(err.kind(), std::io::ErrorKind::Other);
    assert_eq!(
        String::from_utf8(output.written).expect("output should be utf8"),
        "\nHeader\nDetails\n"
    );
}

#[test]
fn permission_prompt_options_preserve_prompt_header_and_details() {
    let options = PermissionPromptOptions {
        prompt: "Allow?".to_string(),
        header: Some("Header".to_string()),
        details: Some("Details".to_string()),
    };

    assert_eq!(options.prompt, "Allow?");
    assert_eq!(options.header.as_deref(), Some("Header"));
    assert_eq!(options.details.as_deref(), Some("Details"));
}

#[test]
fn prompt_for_permission_defaults_to_false_when_terminal_is_unavailable() {
    if can_prompt_for_permission() {
        return;
    }

    let allowed = prompt_for_permission(&PermissionPromptOptions {
        prompt: "Allow?".to_string(),
        header: None,
        details: None,
    })
    .expect("non-terminal prompt should be handled");
    assert!(!allowed);
}

#[cfg(unix)]
#[test]
fn prompt_for_permission_accepts_yes_with_terminal_stdio() {
    let pty = TestPty::open().expect("pty should open");
    let stdin = pty.stdio_from_slave().expect("stdin should duplicate");
    let stderr = pty.stdio_from_slave().expect("stderr should duplicate");
    let current_exe = std::env::current_exe().expect("test binary path should resolve");
    let mut child = std::process::Command::new(current_exe)
        .arg("permission_prompt_terminal_child_accepts_yes")
        .env("VW_ACP_PERMISSION_PROMPT_TERMINAL_CHILD", "1")
        .stdin(stdin)
        .stderr(stderr)
        .stdout(std::process::Stdio::null())
        .spawn()
        .expect("terminal child should spawn");

    pty.write_all(b"yes\n").expect("answer should write");

    let status = child.wait().expect("terminal child should exit");
    assert!(status.success());
}

#[cfg(unix)]
#[test]
fn can_prompt_for_permission_requires_terminal_stderr() {
    let pty = TestPty::open().expect("pty should open");
    let stdin = pty.stdio_from_slave().expect("stdin should duplicate");
    let current_exe = std::env::current_exe().expect("test binary path should resolve");
    let status = std::process::Command::new(current_exe)
        .arg("permission_prompt_terminal_child_requires_terminal_stderr")
        .env("VW_ACP_PERMISSION_PROMPT_STDIN_ONLY_CHILD", "1")
        .stdin(stdin)
        .stderr(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .status()
        .expect("terminal child should spawn");

    assert!(status.success());
}

#[cfg(unix)]
#[test]
fn permission_prompt_terminal_child_requires_terminal_stderr() {
    if std::env::var_os("VW_ACP_PERMISSION_PROMPT_STDIN_ONLY_CHILD").is_none() {
        return;
    }

    assert!(!can_prompt_for_permission());
}

#[cfg(unix)]
#[test]
fn permission_prompt_terminal_child_accepts_yes() {
    if std::env::var_os("VW_ACP_PERMISSION_PROMPT_TERMINAL_CHILD").is_none() {
        return;
    }

    let allowed = prompt_for_permission(&PermissionPromptOptions {
        prompt: "Allow?".to_string(),
        header: Some("Header".to_string()),
        details: Some("Details".to_string()),
    })
    .expect("terminal prompt should read answer");

    assert!(allowed);
}

struct FailingReader;

impl std::io::Read for FailingReader {
    fn read(&mut self, _buf: &mut [u8]) -> std::io::Result<usize> {
        Err(std::io::Error::other("read failed"))
    }
}

impl std::io::BufRead for FailingReader {
    fn fill_buf(&mut self) -> std::io::Result<&[u8]> {
        Err(std::io::Error::other("read failed"))
    }

    fn consume(&mut self, _amt: usize) {}
}

struct FailingWriter;

impl std::io::Write for FailingWriter {
    fn write(&mut self, _buf: &[u8]) -> std::io::Result<usize> {
        Err(std::io::Error::other("write failed"))
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[derive(Default)]
struct FlushFailingWriter {
    written: Vec<u8>,
}

impl std::io::Write for FlushFailingWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.written.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Err(std::io::Error::other("flush failed"))
    }
}

#[derive(Default)]
struct PromptFailingWriter {
    written: Vec<u8>,
}

impl std::io::Write for PromptFailingWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if buf == b"Allow?" {
            return Err(std::io::Error::other("prompt write failed"));
        }
        self.written.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[cfg(unix)]
struct TestPty {
    master: RawFd,
    slave: RawFd,
}

#[cfg(unix)]
impl TestPty {
    fn open() -> std::io::Result<Self> {
        let mut master = -1;
        let mut slave = -1;
        let result = unsafe {
            libc::openpty(
                &mut master,
                &mut slave,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };
        if result < 0 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(Self { master, slave })
    }

    fn stdio_from_slave(&self) -> std::io::Result<std::process::Stdio> {
        let fd = unsafe { libc::dup(self.slave) };
        if fd < 0 {
            return Err(std::io::Error::last_os_error());
        }
        let file = unsafe { std::fs::File::from_raw_fd(fd) };
        Ok(std::process::Stdio::from(file))
    }

    fn write_all(&self, bytes: &[u8]) -> std::io::Result<()> {
        let written = unsafe { libc::write(self.master, bytes.as_ptr().cast(), bytes.len()) };
        if written < 0 {
            return Err(std::io::Error::last_os_error());
        }
        if written as usize != bytes.len() {
            return Err(std::io::Error::new(std::io::ErrorKind::WriteZero, "short pty write"));
        }
        Ok(())
    }
}

#[cfg(unix)]
impl Drop for TestPty {
    fn drop(&mut self) {
        unsafe {
            libc::close(self.master);
            libc::close(self.slave);
        }
    }
}
