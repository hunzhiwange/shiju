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

#[cfg(target_os = "linux")]
#[tokio::test]
async fn acp_platform_spawn_uses_child_path_and_preserves_arguments() {
    use std::os::unix::fs::PermissionsExt;

    let cwd = tempfile::Builder::new().prefix("ACP 工作区 ").tempdir().unwrap();
    let shim = cwd.path().join("sh");
    std::fs::write(
        &shim,
        "#!/bin/sh\nprintf '%s\\0' \"$PWD\" \"$TEST_VALUE\" \"$@\"\n",
    )
    .unwrap();
    std::fs::set_permissions(&shim, std::fs::Permissions::from_mode(0o700)).unwrap();

    let env = HashMap::from([
        ("PATH".into(), cwd.path().to_str().unwrap().into()),
        ("TEST_VALUE".into(), "环境 空格".into()),
    ]);
    let args = ["", "参数 空格", "$(literal);*"];
    let output = build_spawn_command("sh", &env)
        .current_dir(cwd.path())
        .args(args)
        .output()
        .await
        .unwrap();
    assert!(output.status.success());
    let expected = [cwd.path().to_str().unwrap(), "环境 空格", args[0], args[1], args[2]];
    assert_eq!(output.stdout, format!("{}\0", expected.join("\0")).as_bytes());
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn acp_platform_spawn_reports_missing_program_with_explicit_path() {
    let cwd = tempfile::tempdir().unwrap();
    let env = HashMap::from([("PATH".into(), cwd.path().to_str().unwrap().into())]);
    let error = build_spawn_command("sh", &env).spawn().unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::NotFound);
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn acp_platform_spawn_runs_npm_style_shim_with_child_node() {
    use std::os::unix::fs::{PermissionsExt, symlink};

    let cwd = tempfile::Builder::new().prefix("npm 软件包 ").tempdir().unwrap();
    let entry = cwd.path().join("入口 文件.js");
    let node = cwd.path().join("node");
    symlink(&entry, cwd.path().join("代理 shim")).unwrap();
    std::fs::write(&entry, "#!/usr/bin/env node\n").unwrap();
    // Stand in for Node so the test does not require installing an upstream runtime.
    std::fs::write(&node, "#!/bin/sh\nprintf '%s\\n' \"$@\"\n").unwrap();
    for path in [&entry, &node] {
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)).unwrap();
    }
    let env = HashMap::from([("PATH".into(), cwd.path().to_str().unwrap().into())]);
    let run = || {
        let mut command = build_spawn_command("./代理 shim", &env);
        command.current_dir(cwd.path()).arg("中文 参数");
        command
    };
    let output = run().output().await.unwrap();
    assert!(output.status.success());
    assert_eq!(output.stdout, "./代理 shim\n中文 参数\n".as_bytes());

    std::fs::remove_file(node).unwrap();
    let missing_node = run().output().await.unwrap();
    assert_eq!(missing_node.status.code(), Some(127));
}
