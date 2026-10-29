//! Windows CLI replacement preserves a loaded executable until its users exit.

use std::fs;
use std::path::Path;
use std::time::Duration;

// Six attempts, with at most 2.5 seconds of backoff per file operation.
const FILE_RETRY_DELAYS: [Duration; 5] = [
    Duration::from_millis(100),
    Duration::from_millis(200),
    Duration::from_millis(400),
    Duration::from_millis(800),
    Duration::from_millis(1000),
];

fn retry_windows_file_operation(
    mut operation: impl FnMut() -> std::io::Result<()>,
) -> std::io::Result<()> {
    for (attempt, delay) in FILE_RETRY_DELAYS.into_iter().enumerate() {
        match operation() {
            Ok(()) => return Ok(()),
            // ERROR_ACCESS_DENIED, ERROR_SHARING_VIOLATION and ERROR_LOCK_VIOLATION
            // can persist briefly while a loaded image or antivirus releases a file.
            Err(error) if matches!(error.raw_os_error(), Some(5 | 32 | 33)) => {
                tracing::debug!(target: "vw_desktop", attempt = attempt + 1, %error,
                    "CLI file operation is blocked; retrying after backoff");
                std::thread::sleep(delay);
            }
            Err(error) => return Err(error),
        }
    }
    operation()
}

pub(super) fn replace_staged_cli(staged: &Path, target: &Path) -> Result<(), String> {
    match fs::symlink_metadata(target) {
        Ok(metadata) if !metadata.is_file() => {
            return Err(format!("CLI 安装目标不是普通文件: {}", target.display()));
        }
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(format!("读取旧 CLI 文件状态失败: {error}")),
    }
    let parent = target.parent().ok_or_else(|| "CLI 安装目录无效".to_string())?;
    let backup_dir = tempfile::Builder::new()
        .prefix(".shiju-cli-backup-")
        .tempdir_in(parent)
        .map_err(|error| format!("创建 CLI 备份目录失败: {error}"))?;
    let backup = backup_dir.path().join("shiju.exe");
    // A loaded image can be renamed on Windows, but cannot be deleted or overwritten.
    let backed_up = match retry_windows_file_operation(|| fs::rename(target, &backup)) {
        Ok(()) => true,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
        Err(error) => {
            return Err(format!("备份旧 CLI 可执行文件失败 {}: {error}", target.display()));
        }
    };

    if let Err(error) = retry_windows_file_operation(|| fs::rename(staged, target)) {
        if backed_up
            && let Err(restore_error) = retry_windows_file_operation(|| fs::rename(&backup, target))
        {
            let retained = backup_dir.keep();
            return Err(format!(
                "替换 CLI 失败: {error}；恢复旧 CLI 失败: {restore_error}；旧文件保留于 {}",
                retained.join("shiju.exe").display()
            ));
        }
        return Err(format!("替换 CLI 可执行文件失败 {}: {error}", target.display()));
    }

    if backed_up && let Err(error) = retry_windows_file_operation(|| fs::remove_file(&backup)) {
        let retained = backup_dir.keep();
        tracing::warn!(target: "vw_desktop", %error, path = %retained.display(),
            "old CLI is still in use; retained its backup after successful replacement");
    }
    Ok(())
}

#[cfg(windows)]
pub(super) fn stop_installed_daemon(
    cli: &Path,
    endpoint: &super::GatewayRuntimeEndpoint,
) -> Result<(), String> {
    let Some(health) = super::gateway_health(endpoint) else { return Ok(()) };
    let pid = health
        .pointer("/runtime/pid")
        .and_then(serde_json::Value::as_u64)
        .and_then(|pid| u32::try_from(pid).ok())
        .filter(|pid| *pid != 0 && *pid != std::process::id())
        .ok_or_else(|| "无法确认旧 Gateway 的进程，请手动退出旧 shiju daemon 后重试".to_string())?;
    stop_cli_process(pid, cli)
}

#[cfg(windows)]
fn stop_cli_process(pid: u32, cli: &Path) -> Result<(), String> {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
    use windows_sys::Win32::Foundation::{ERROR_INVALID_PARAMETER, WAIT_OBJECT_0, WAIT_TIMEOUT};
    use windows_sys::Win32::System::Threading::{
        OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE, PROCESS_TERMINATE,
        QueryFullProcessImageNameW, TerminateProcess, WaitForSingleObject,
    };

    if pid == 0 || pid == std::process::id() {
        return Err("拒绝停止无效的旧 CLI 进程".to_string());
    }
    // Keep one handle for identity verification, termination and waiting to avoid PID reuse.
    let raw = unsafe {
        OpenProcess(
            PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_TERMINATE | PROCESS_SYNCHRONIZE,
            0,
            pid,
        )
    };
    if raw.is_null() {
        let error = std::io::Error::last_os_error();
        if error.raw_os_error() == Some(ERROR_INVALID_PARAMETER as i32) {
            return Ok(());
        }
        return Err(format!("打开旧 CLI 进程 {pid} 失败: {error}"));
    }
    // SAFETY: OpenProcess returned a valid owned handle, closed when this guard is dropped.
    let process = unsafe { OwnedHandle::from_raw_handle(raw) };
    let handle = process.as_raw_handle();
    if unsafe { WaitForSingleObject(handle, 0) } == WAIT_OBJECT_0 {
        return Ok(());
    }
    let mut image = vec![0u16; 32768];
    let mut length = image.len() as u32;
    if unsafe { QueryFullProcessImageNameW(handle, 0, image.as_mut_ptr(), &mut length) } == 0 {
        return Err(format!("读取旧 CLI 进程路径失败: {}", std::io::Error::last_os_error()));
    }
    let image = std::path::PathBuf::from(OsString::from_wide(&image[..length as usize]));
    let expected =
        fs::canonicalize(cli).map_err(|error| format!("读取 CLI 安装路径失败: {error}"))?;
    let actual =
        fs::canonicalize(&image).map_err(|error| format!("读取旧 CLI 进程路径失败: {error}"))?;
    if actual != expected {
        return Err(format!("旧 Gateway 进程 {pid} 不属于 {}，请手动停止后重试", cli.display()));
    }
    if unsafe { TerminateProcess(handle, 0) } == 0 {
        let error = std::io::Error::last_os_error();
        if unsafe { WaitForSingleObject(handle, 0) } != WAIT_OBJECT_0 {
            return Err(format!("停止旧 CLI 进程 {pid} 失败: {error}"));
        }
    }
    match unsafe { WaitForSingleObject(handle, 10_000) } {
        WAIT_OBJECT_0 => Ok(()),
        WAIT_TIMEOUT => {
            Err(format!("等待旧 CLI 进程 {pid} 退出超时，请退出旧 shiju daemon 后重试"))
        }
        _ => Err(format!("等待旧 CLI 进程退出失败: {}", std::io::Error::last_os_error())),
    }
}

#[cfg(test)]
#[path = "config_cli_install_windows_tests.rs"]
mod tests;
