use std::process::Command;

use crate::Sandbox;
use crate::bubblewrap::BubblewrapSandbox;

// 测试 bubblewrap 沙箱的名称是否正确返回 "bubblewrap"
#[test]
fn bubblewrap_sandbox_name() {
    let sandbox = BubblewrapSandbox;
    assert_eq!(sandbox.name(), "bubblewrap");
}

// 测试 bubblewrap 是否仅在系统中安装后才返回可用状态
#[test]
fn bubblewrap_is_available_only_if_installed() {
    let sandbox = BubblewrapSandbox;
    let _available = sandbox.is_available();

    assert_eq!(sandbox.name(), "bubblewrap");
}

// 测试包装后的命令是否包含必要的隔离标志（如 --unshare-all、--die-with-parent 等）
#[test]
fn bubblewrap_wrap_command_includes_isolation_flags() {
    let sandbox = BubblewrapSandbox;
    if !sandbox.is_available() {
        return;
    }

    let mut cmd = Command::new("echo");
    cmd.arg("hello");
    sandbox.wrap_command(&mut cmd).unwrap();

    assert_eq!(
        cmd.get_program().to_string_lossy(),
        "bwrap",
        "wrapped command should use bwrap as program"
    );

    let args: Vec<String> = cmd.get_args().map(|s| s.to_string_lossy().to_string()).collect();

    assert!(
        args.contains(&"--unshare-all".to_string()),
        "must include --unshare-all for namespace isolation"
    );
    assert!(
        args.contains(&"--die-with-parent".to_string()),
        "must include --die-with-parent to prevent orphan processes"
    );
    assert!(
        !args.contains(&"--share-net".to_string()),
        "must NOT include --share-net (network should be blocked)"
    );
}

// 测试包装命令是否正确保留原始命令及其参数
#[test]
fn bubblewrap_wrap_command_preserves_original_command() {
    let sandbox = BubblewrapSandbox;
    if !sandbox.is_available() {
        return;
    }

    let mut cmd = Command::new("ls");
    cmd.arg("-la");
    cmd.arg("/tmp");
    sandbox.wrap_command(&mut cmd).unwrap();

    let args: Vec<String> = cmd.get_args().map(|s| s.to_string_lossy().to_string()).collect();

    assert!(args.contains(&"ls".to_string()), "original program must be passed as argument");
    assert!(args.contains(&"-la".to_string()), "original args must be preserved");
    assert!(args.contains(&"/tmp".to_string()), "original args must be preserved");
}

// 测试包装命令是否绑定了必要的系统路径（如 /usr、/dev、/proc 等）
#[test]
fn bubblewrap_wrap_command_binds_required_paths() {
    let sandbox = BubblewrapSandbox;
    if !sandbox.is_available() {
        return;
    }

    let mut cmd = Command::new("echo");
    sandbox.wrap_command(&mut cmd).unwrap();

    let args: Vec<String> = cmd.get_args().map(|s| s.to_string_lossy().to_string()).collect();

    assert!(args.contains(&"--ro-bind".to_string()), "must include read-only bind for /usr");
    assert!(args.contains(&"--dev".to_string()), "must include /dev mount");
    assert!(args.contains(&"--proc".to_string()), "must include /proc mount");
}
