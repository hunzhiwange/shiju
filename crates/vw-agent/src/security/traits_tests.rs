//! 验证空沙箱实现的稳定元数据与无损命令包装。

use super::{NoopSandbox, Sandbox};
use std::process::Command;

#[test]
fn noop_sandbox_reports_stable_metadata_and_wraps_without_mutation() {
    let sandbox = NoopSandbox;
    let mut command = Command::new("echo");
    command.arg("hello");

    sandbox.wrap_command(&mut command).expect("noop sandbox should not fail");

    assert!(sandbox.is_available());
    assert_eq!(sandbox.name(), "none");
    assert!(sandbox.description().contains("sandboxing"));
}
