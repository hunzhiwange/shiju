//! 验证启动命令构造会将显式环境覆盖传递给子进程。
//!
//! 最小命令配置只检查环境覆盖是否进入构造结果，不启动子进程。

use super::*;
use std::collections::HashMap;

// 该用例固定显式环境变量覆盖的键和值均保持不变。
#[test]
fn build_spawn_command_applies_env_overrides() {
    let mut env = HashMap::new();
    env.insert("VW_ACP_TEST_ENV".to_string(), "ok".to_string());

    let command = build_spawn_command("echo", &env);
    let found = command
        .as_std()
        .get_envs()
        .any(|(key, value)| key == "VW_ACP_TEST_ENV" && value == Some(std::ffi::OsStr::new("ok")));

    assert!(found);
}
