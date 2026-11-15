//! 跨平台进程树终止。

use std::process::Child;
#[cfg(not(windows))]
use std::time::Duration;

#[cfg(not(windows))]
const SIGKILL_TIMEOUT_MS: u64 = 200;

/// 终止进程树
///
/// 递归终止指定进程及其所有子进程。这是一个跨平台的实现。
pub fn kill_tree(proc: &mut Child, exited: Option<&dyn Fn() -> bool>) {
    let pid = proc.id();
    if pid == 0 || exited.is_some_and(|f| f()) {
        return;
    }

    #[cfg(windows)]
    {
        let _ = std::process::Command::new("taskkill")
            .args(["/pid", &pid.to_string(), "/f", "/t"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
        return;
    }

    #[cfg(not(windows))]
    {
        let mut tried_group = false;
        if let Ok(status) =
            std::process::Command::new("kill").args(["-TERM", &format!("-{pid}")]).status()
        {
            tried_group = status.success();
        }

        if tried_group {
            std::thread::sleep(Duration::from_millis(SIGKILL_TIMEOUT_MS));
            if !exited.is_some_and(|f| f()) {
                let _ =
                    std::process::Command::new("kill").args(["-KILL", &format!("-{pid}")]).status();
            }
            return;
        }

        let _ = proc.kill();
        std::thread::sleep(Duration::from_millis(SIGKILL_TIMEOUT_MS));
        if !exited.is_some_and(|f| f()) {
            let _ = proc.kill();
        }
    }
}
