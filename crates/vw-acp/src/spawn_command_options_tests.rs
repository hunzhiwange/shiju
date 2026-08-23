//! 验证启动命令构造会将显式环境覆盖传递给子进程。

use super::*;
use std::collections::HashMap;

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
