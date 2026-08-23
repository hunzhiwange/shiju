use std::env;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::Stdio;

#[cfg(windows)]
use codex_protocol::permissions::FileSystemPath;
use codex_protocol::permissions::FileSystemSandboxPolicy;
#[cfg(windows)]
use codex_protocol::permissions::FileSystemSpecialPath;
use color_eyre::eyre::Report;
use color_eyre::eyre::Result;
use tempfile::Builder;
use thiserror::Error;
use tokio::process::Command;

#[derive(Debug, Error)]
pub(crate) enum EditorError {
    /// 环境变量、命令解析和空命令分别保留可操作的失败原因。
    #[error("neither VISUAL nor EDITOR is set")]
    MissingEditor,
    #[cfg(not(windows))]
    #[error("failed to parse editor command")]
    ParseFailed,
    #[error("editor command is empty")]
    EmptyCommand,
}

/// 按 `PATH` 和 `PATHEXT` 尝试解析 Windows 程序的完整路径。
/// 解析失败时回退到原始程序名。
/// Tries to resolve the full path to a Windows program, respecting PATH + PATHEXT.
/// Falls back to the original program name if resolution fails.
#[cfg(windows)]
fn resolve_windows_program(program: &str) -> std::path::PathBuf {
    // Windows 上 `Command::new("code")` 不会解析 PATH 中的 `code.cmd` 包装脚本，
    // 因此通过 `which` 同时遵守 PATH 与 PATHEXT。
    // On Windows, `Command::new("code")` will not resolve `code.cmd` shims on PATH.
    // Use `which` so we respect PATH + PATHEXT (e.g., `code` -> `code.cmd`).
    which::which(program).unwrap_or_else(|_| std::path::PathBuf::from(program))
}

/// 从环境变量解析编辑器命令，并优先使用 `VISUAL` 而非 `EDITOR`。
/// Resolve the editor command from environment variables.
/// Prefers `VISUAL` over `EDITOR`.
pub(crate) fn resolve_editor_command() -> std::result::Result<Vec<String>, EditorError> {
    let raw = env::var("VISUAL")
        .or_else(|_| env::var("EDITOR"))
        .map_err(|_| EditorError::MissingEditor)?;
    let parts = {
        #[cfg(windows)]
        {
            winsplit::split(&raw)
        }
        #[cfg(not(windows))]
        {
            shlex::split(&raw).ok_or(EditorError::ParseFailed)?
        }
    };
    if parts.is_empty() {
        return Err(EditorError::EmptyCommand);
    }
    Ok(parts)
}

pub(super) fn editor_directory(
    candidate_homes: &[&Path],
    file_system_policy: &FileSystemSandboxPolicy,
    cwd: &Path,
) -> Result<PathBuf> {
    // 编辑器临时目录必须位于沙箱不可写的受信父目录，防止代理篡改待执行内容。
    let writable_roots = file_system_policy.get_writable_roots_with_cwd(cwd);
    #[cfg(windows)]
    let windows_temporary_roots = if !file_system_policy.has_full_disk_write_access()
        && file_system_policy.entries.iter().any(|entry| {
            matches!(
                &entry.path,
                FileSystemPath::Special {
                    value: FileSystemSpecialPath::Tmpdir
                }
            ) && entry.access.can_write()
        }) {
        ["TEMP", "TMP"]
            .into_iter()
            .filter_map(env::var_os)
            .map(PathBuf::from)
            .filter(|path| path.is_absolute())
            .flat_map(|path| {
                let canonical_path = dunce::canonicalize(&path).ok();
                std::iter::once(path).chain(canonical_path)
            })
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    let mut error = Report::msg("editor directory must not be writable");
    let mut rejected_writable = false;

    for candidate_home in candidate_homes {
        let canonical_home = match dunce::canonicalize(candidate_home) {
            Ok(path) => path,
            Err(canonicalize_error)
                if canonicalize_error.kind() == std::io::ErrorKind::NotFound =>
            {
                let Some(parent) = candidate_home.parent() else {
                    error = Report::msg("editor directory has no parent");
                    continue;
                };
                let Some(name) = candidate_home.file_name() else {
                    error = Report::msg("editor directory has no parent");
                    continue;
                };
                match dunce::canonicalize(parent) {
                    Ok(parent) => parent.join(name),
                    Err(canonicalize_error) => {
                        error = canonicalize_error.into();
                        continue;
                    }
                }
            }
            Err(canonicalize_error) => {
                error = canonicalize_error.into();
                continue;
            }
        };
        let editor_directory = canonical_home.join("editor");
        let logical_editor_directory = candidate_home.join("editor");

        // 同时检查逻辑路径和规范路径，避免符号链接或父子根关系绕过写权限判断。
        if !file_system_policy.has_full_disk_write_access()
            && [&logical_editor_directory, &editor_directory]
                .into_iter()
                .any(|directory| {
                    let Some(parent) = directory.parent() else {
                        return true;
                    };
                    let is_writable = file_system_policy.can_write_path_with_cwd(directory, cwd)
                        || file_system_policy.can_write_path_with_cwd(parent, cwd)
                        || writable_roots.iter().any(|root| {
                            root.is_path_writable(directory)
                                || root.root.as_path().starts_with(directory)
                        });
                    #[cfg(windows)]
                    let is_writable = is_writable
                        || windows_temporary_roots.iter().any(|root| {
                            directory.starts_with(root)
                                || parent.starts_with(root)
                                || root.starts_with(directory)
                        });
                    is_writable
                })
        {
            error = Report::msg("editor directory must not be writable");
            rejected_writable = true;
            continue;
        }

        if let Err(create_error) = fs::create_dir_all(&editor_directory) {
            error = create_error.into();
            continue;
        }
        match dunce::canonicalize(&editor_directory) {
            Ok(path) if path == editor_directory => return Ok(editor_directory),
            Ok(_) => {
                error = Report::msg("editor directory must not contain symbolic links");
            }
            Err(canonicalize_error) => {
                error = canonicalize_error.into();
            }
        }
    }

    if rejected_writable {
        Err(Report::msg("editor directory must not be writable"))
    } else {
        Err(error)
    }
}

/// 把 `seed` 写入临时文件，启动编辑器命令，并返回编辑后的内容。
/// Write `seed` to a temp file, launch the editor command, and return the updated content.
pub(crate) async fn run_editor(
    seed: &str,
    editor_cmd: &[String],
    codex_home: &Path,
    file_system_policy: &FileSystemSandboxPolicy,
    cwd: &Path,
) -> Result<String> {
    if editor_cmd.is_empty() {
        return Err(Report::msg("editor command is empty"));
    }

    let default_codex_home = dirs::home_dir().map(|home| home.join(".codex"));
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    let project_codex_home = cwd.join(".codex");
    let mut candidate_homes = vec![codex_home];
    if let Some(default_codex_home) = default_codex_home.as_deref() {
        candidate_homes.push(default_codex_home);
    }
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    candidate_homes.push(&project_codex_home);
    let editor_directory = editor_directory(&candidate_homes, file_system_policy, cwd)?;
    // 立即转换为 `TempPath`，避免 Windows 上仍有文件句柄占用。
    // Convert to TempPath immediately so no file handle stays open on Windows.
    let temp_path = Builder::new()
        .suffix(".md")
        .tempfile_in(editor_directory)?
        .into_temp_path();
    fs::write(&temp_path, seed)?;

    let mut cmd = {
        #[cfg(windows)]
        {
            // 兼容 `.cmd` 和 `.bat` 包装脚本。
            // handles .cmd/.bat shims
            Command::new(resolve_windows_program(&editor_cmd[0]))
        }
        #[cfg(not(windows))]
        {
            Command::new(&editor_cmd[0])
        }
    };
    if editor_cmd.len() > 1 {
        cmd.args(&editor_cmd[1..]);
    }
    let status = cmd
        .arg(&temp_path)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .await?;

    if !status.success() {
        return Err(Report::msg(format!("editor exited with status {status}")));
    }

    let contents = fs::read_to_string(&temp_path)?;
    Ok(contents)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use serial_test::serial;
    #[cfg(unix)]
    use tempfile::tempdir;

    struct EnvGuard {
        // 串行测试结束时恢复进程级编辑器环境变量，避免污染其他测试。
        visual: Option<String>,
        editor: Option<String>,
    }

    impl EnvGuard {
        fn new() -> Self {
            Self {
                visual: env::var("VISUAL").ok(),
                editor: env::var("EDITOR").ok(),
            }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            restore_env("VISUAL", self.visual.take());
            restore_env("EDITOR", self.editor.take());
        }
    }

    fn restore_env(key: &str, value: Option<String>) {
        match value {
            Some(val) => unsafe { env::set_var(key, val) },
            None => unsafe { env::remove_var(key) },
        }
    }

    #[test]
    #[serial]
    // 同时设置两个变量时必须优先解析 `VISUAL`。
    fn resolve_editor_prefers_visual() {
        let _guard = EnvGuard::new();
        unsafe {
            env::set_var("VISUAL", "vis");
            env::set_var("EDITOR", "ed");
        }
        let cmd = resolve_editor_command().unwrap();
        assert_eq!(cmd, vec!["vis".to_string()]);
    }

    #[test]
    #[serial]
    // 两个环境变量都缺失时应返回明确的缺少编辑器错误。
    fn resolve_editor_errors_when_unset() {
        let _guard = EnvGuard::new();
        unsafe {
            env::remove_var("VISUAL");
            env::remove_var("EDITOR");
        }
        assert!(matches!(
            resolve_editor_command(),
            Err(EditorError::MissingEditor)
        ));
    }

    #[tokio::test]
    #[cfg(unix)]
    // 用临时脚本模拟编辑器，验证子进程写回内容会被完整读取。
    async fn run_editor_returns_updated_content() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempdir().unwrap();
        let script_path = dir.path().join("edit.sh");
        fs::write(&script_path, "#!/bin/sh\nprintf \"edited\" > \"$1\"\n").unwrap();
        let mut perms = fs::metadata(&script_path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&script_path, perms).unwrap();

        let cmd = vec![script_path.to_string_lossy().to_string()];
        let policy = FileSystemSandboxPolicy::read_only();
        let result = run_editor("seed", &cmd, dir.path(), &policy, dir.path())
            .await
            .unwrap();
        assert_eq!(result, "edited".to_string());
    }
}

#[cfg(test)]
#[path = "external_editor_tests.rs"]
mod buffer_tests;
//! 在文件系统沙箱边界内选择外部编辑器、安全创建临时文件并读取编辑结果。
