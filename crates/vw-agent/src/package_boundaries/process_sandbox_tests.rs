use crate::security::traits::{NoopSandbox, Sandbox};
use std::ffi::OsStr;
use std::process::Command;

#[test]
fn process_sandbox_production_boundary() {
    let sandbox: process_sandbox::NoopSandbox = NoopSandbox;
    let old_entry: &dyn Sandbox = &sandbox;
    let new_entry: &dyn process_sandbox::Sandbox = old_entry;
    assert!(new_entry.is_available());

    // 只包装命令，不启动进程；空沙箱必须完整保留调用方设置。
    let workspace = tempfile::tempdir().expect("temporary workspace");
    let mut command = Command::new("sandbox-boundary-command");
    command.args(["--value", "含空格 参数"]).current_dir(workspace.path());
    command.env("PROCESS_SANDBOX_BOUNDARY", "preserved");
    old_entry.wrap_command(&mut command).expect("noop command wrapping");

    assert_eq!(command.get_program(), OsStr::new("sandbox-boundary-command"));
    assert_eq!(command.get_args().collect::<Vec<_>>(), ["--value", "含空格 参数"]);
    assert_eq!(command.get_current_dir(), Some(workspace.path()));
    assert!(command.get_envs().any(|(key, value)| {
        key == OsStr::new("PROCESS_SANDBOX_BOUNDARY") && value == Some(OsStr::new("preserved"))
    }));
}
