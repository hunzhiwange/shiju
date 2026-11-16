//! Export domain messages, never terminal buffers or attachment payloads.
use std::path::{Path, PathBuf};
use cli_interaction_model::UiMessage;
use cli_interaction_state::TuiState;
use cli_gateway_workspace::workspace_files::normalize_workspace_relative_path;

pub(crate) fn markdown(state: &TuiState) -> Result<String, String> {
    let mut output = String::from("# 会话导出\n\n> 仅包含当前已加载的会话消息；未加载历史未导出，不保证完整。\n\n");
    for message in &state.messages {
        if message.base().session_id.is_some()
            && message.base().session_id.as_deref() != state.session.session_id.as_deref()
        {
            return Err("消息不属于当前会话，无法导出".into());
        }
        let (role, text): (&str, &str) = match message {
            UiMessage::User(m) => ("User", &m.text),
            UiMessage::Assistant(m) => ("Assistant", &m.text),
            UiMessage::System(m) => ("System", &m.text),
            UiMessage::Thinking(m) => ("Thinking", &m.content),
            UiMessage::ToolCall(m) => ("Tool call", m.arguments.as_deref().or(m.summary.as_deref()).unwrap_or_default()),
            UiMessage::ToolResult(m) => ("Tool result", &m.content),
            UiMessage::Error(m) => ("Error", &m.message),
            // Even names/previews/URIs may contain credentials or encoded data.
            UiMessage::Attachment(_) => ("Attachment", "[附件已省略；不导出名称、路径、预览或二进制内容]"),
            UiMessage::CompactBoundary(m) => ("Compact boundary", &m.summary),
            UiMessage::Progress(m) => ("Progress", &m.label),
            UiMessage::Step(_) => ("Step", "[步骤状态记录]"),
            UiMessage::Unknown(_) => ("Unknown", "[未识别消息载荷已省略]"),
        };
        output.push_str(&format!("## {role}\n\n"));
        output.push_str(text);
        output.push_str("\n\n");
    }
    Ok(output)
}

pub(crate) fn export(state: &TuiState, root: &Path, path: &str) -> Result<PathBuf, String> {
    let path = normalize_workspace_relative_path(path)?;
    if path == "." { return Err("导出需要文件路径".into()); }
    let content = markdown(state)?;
    let root = root.canonicalize().map_err(|e| format!("工作区不可用：{e}"))?;
    write_new(&root, Path::new(&path), content.as_bytes())
        .map_err(|e| format!("导出失败（不会覆盖已有文件）：{e}"))?;
    Ok(root.join(path))
}

/// Descriptor-relative traversal prevents parent symlink substitution. Publish only
/// after the complete private file has been synced; linkat never replaces a name.
#[cfg(unix)]
fn write_new(root: &Path, relative: &Path, content: &[u8]) -> std::io::Result<()> {
    use std::{ffi::CString, fs::File, io::Write, os::{fd::{AsRawFd, FromRawFd}, unix::ffi::OsStrExt}, sync::atomic::{AtomicU64, Ordering}};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let open_dir = |parent: &File, name: &std::ffi::OsStr| -> std::io::Result<File> {
        let name = CString::new(name.as_bytes())?;
        // SAFETY: parent descriptor and name remain live; returned fd is owned.
        let fd = unsafe { libc::openat(parent.as_raw_fd(), name.as_ptr(), libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC) };
        if fd < 0 { return Err(std::io::Error::last_os_error()); }
        Ok(unsafe { File::from_raw_fd(fd) })
    };
    let mut directory = File::open("/")?;
    for component in root.components().chain(relative.parent().unwrap_or(Path::new("")).components()) {
        if let std::path::Component::Normal(name) = component {
            directory = open_dir(&directory, name)?;
        }
    }
    let destination = CString::new(relative.file_name().ok_or(std::io::ErrorKind::InvalidInput)?.as_bytes())?;
    let temporary = CString::new(format!(".shiju-export-{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)))?;
    // SAFETY: valid directory descriptor and NUL-terminated name, with mode for O_CREAT.
    let fd = unsafe { libc::openat(directory.as_raw_fd(), temporary.as_ptr(), libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC, 0o600) };
    if fd < 0 { return Err(std::io::Error::last_os_error()); }
    let mut file = unsafe { File::from_raw_fd(fd) };
    let result = (|| {
        file.write_all(content)?;
        file.sync_all()?;
        // SAFETY: names and directory remain live. No flags: do not follow links.
        if unsafe { libc::linkat(directory.as_raw_fd(), temporary.as_ptr(), directory.as_raw_fd(), destination.as_ptr(), 0) } < 0 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(())
    })();
    // SAFETY: remove only the temporary name created by this invocation.
    unsafe { libc::unlinkat(directory.as_raw_fd(), temporary.as_ptr(), 0); }
    result
}

#[cfg(not(unix))]
fn write_new(_: &Path, _: &Path, _: &[u8]) -> std::io::Result<()> {
    Err(std::io::Error::new(std::io::ErrorKind::Unsupported, "此平台暂不支持安全的会话导出"))
}
