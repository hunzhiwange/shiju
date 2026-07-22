//! 验证 ACP 子进程启动命令的环境变量与 shell 路径处理。
//!
//! 该文件只在 Unix 平台启用，因为测试依赖 shell、可执行权限和用户 profile
//! 中的 PATH 扩展行为。

#[cfg(unix)]
use std::collections::HashMap;
#[cfg(unix)]
use std::ffi::OsStr;
#[cfg(unix)]
use std::fs;
#[cfg(unix)]
use std::path::Path;
#[cfg(unix)]
use std::process::Stdio;
#[cfg(unix)]
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(unix)]
use tokio::io::AsyncReadExt;
#[cfg(unix)]
use vw_acp::build_spawn_command;

/// 在测试期间临时覆盖单个环境变量。
///
/// 创建时保存原值，析构时恢复；这样可以验证 HOME/PATH 等进程级状态，
/// 同时避免污染同一测试进程中的后续用例。
#[cfg(unix)]
struct EnvGuard {
    key: &'static str,
    original: Option<std::ffi::OsString>,
}

#[cfg(unix)]
impl EnvGuard {
    /// 设置环境变量并返回恢复守卫。
    ///
    /// 参数 `key` 是要覆盖的变量名，`value` 是测试期值；返回值在 drop 时恢复
    /// 原始状态。Rust 2024 将环境变量修改标记为 unsafe，因为它会影响整个进程。
    fn set_os(key: &'static str, value: &OsStr) -> Self {
        let original = std::env::var_os(key);
        unsafe { std::env::set_var(key, value) };
        Self { key, original }
    }
}

#[cfg(unix)]
impl Drop for EnvGuard {
    fn drop(&mut self) {
        // 环境变量是进程全局状态，必须在测试结束时恢复，避免并发/后续测试读取到
        // 本用例注入的 HOME 或 PATH。
        match &self.original {
            Some(value) => unsafe { std::env::set_var(self.key, value) },
            None => unsafe { std::env::remove_var(self.key) },
        }
    }
}

#[cfg(unix)]
/// 创建可执行脚本文件。
///
/// 参数 `path` 指向脚本路径，`contents` 是脚本文本；函数会创建父目录并设置
/// Unix 可执行位，便于后续通过 PATH 查找执行。
fn make_executable(path: &Path, contents: &str) {
    fs::create_dir_all(path.parent().expect("script parent should exist"))
        .expect("script parent should be created");
    fs::write(path, contents).expect("script should be written");
    use std::os::unix::fs::PermissionsExt;
    let mut perms = fs::metadata(path).expect("script metadata should exist").permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms).expect("script permissions should be updated");
}

#[cfg(unix)]
/// 生成用于 PATH/profile 测试的唯一临时目录。
///
/// 返回值只构造路径，不自动清理；调用方负责在测试末尾删除目录。
fn unique_temp_dir() -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after unix epoch")
        .as_nanos();
    std::env::temp_dir()
        .join(format!("vw-acp-spawn-command-options-{nanos}-{}", std::process::id()))
}

#[cfg(unix)]
#[tokio::test]
async fn build_spawn_command_uses_augmented_shell_path() {
    let home = unique_temp_dir();
    fs::create_dir_all(&home).expect("temp home should be created");
    let profile_bin = home.join("profile-bin");
    let script = profile_bin.join("vw-acp-path-script");

    make_executable(&script, "#!/bin/sh\nprintf profile-path-ok\n");
    fs::write(home.join(".profile"), format!("export PATH={}:$PATH\n", profile_bin.display()))
        .expect("profile should be written");

    // 强制 HOME/PATH 指向测试夹，确保命令解析依赖本用例创建的 profile，而不是
    // 开发者机器上的真实 shell 配置。
    let _home_guard = EnvGuard::set_os("HOME", home.as_os_str());
    let _path_guard = EnvGuard::set_os("PATH", OsStr::new("/usr/bin:/bin"));

    let mut command = build_spawn_command("vw-acp-path-script", &HashMap::new());
    command.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null());

    let mut child = command.spawn().expect("command should spawn via profile PATH");
    let mut stdout = String::new();
    child
        .stdout
        .take()
        .expect("stdout should be piped")
        .read_to_string(&mut stdout)
        .await
        .expect("stdout should be readable");
    let status = child.wait().await.expect("child should exit");

    assert!(status.success());
    assert_eq!(stdout, "profile-path-ok");

    let _ = fs::remove_dir_all(home);
}
