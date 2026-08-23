//! 时聚应用启动器安全测试，覆盖可信运行地址、辅助进程失败和密钥传递。
//! 会话凭据只能经标准输入交给子进程，任何错误信息都不得回显原始秘密。

use std::path::{Path, PathBuf};

use super::launcher::{LaunchProcess, launch_with};
use vw_gateway_client::ShijuAppLaunchDto;

#[derive(Default)]
struct MemoryProcess {
    argv: Vec<String>,
    stdin: Vec<u8>,
    spawn_error: Option<String>,
    write_error: Option<String>,
    terminated: bool,
}

impl LaunchProcess for MemoryProcess {
    type Child = ();

    fn spawn(&mut self, program: &Path, args: &[String]) -> Result<Self::Child, String> {
        self.argv =
            std::iter::once(program.display().to_string()).chain(args.iter().cloned()).collect();
        if let Some(error) = &self.spawn_error { Err(error.clone()) } else { Ok(()) }
    }

    fn write_stdin(&mut self, _child: &mut Self::Child, input: &[u8]) -> Result<(), String> {
        self.stdin.extend_from_slice(input);
        if let Some(error) = &self.write_error { Err(error.clone()) } else { Ok(()) }
    }

    fn terminate(&mut self, _child: &mut Self::Child) {
        self.terminated = true;
    }
}

fn target(runtime_url: &str) -> ShijuAppLaunchDto {
    ShijuAppLaunchDto {
        installation_id: "01K1APP0000000000000000001".to_string(),
        name: "会议纪要助手".to_string(),
        runtime_url: runtime_url.to_string(),
        runtime_set_cookie:
            "app_runtime_session=fixture-runtime-session-secret; Path=/; HttpOnly; Secure; SameSite=None"
                .to_string(),
    }
}

#[test]
fn launcher_passes_set_cookie_only_over_stdin() {
    let mut process = MemoryProcess::default();
    launch_with(
        &mut process,
        &[PathBuf::from("/safe/vw-webview")],
        &target("https://app.shiju.cloud/app/01K1APP0000000000000000001/"),
        "01K1APP0000000000000000001",
    )
    .unwrap();

    assert_eq!(
        process.argv,
        [
            "/safe/vw-webview",
            "https://app.shiju.cloud/app/01K1APP0000000000000000001/",
            "--title=会议纪要助手",
            "--shiju-app-set-cookie-stdin",
        ]
    );
    assert!(!process.argv.join(" ").contains("fixture-runtime-session-secret"));
    assert_eq!(
        process.stdin,
        b"app_runtime_session=fixture-runtime-session-secret; Path=/; HttpOnly; Secure; SameSite=None\n"
    );
    assert!(!process.terminated);
}

#[test]
fn launcher_rejects_untrusted_runtime_without_external_fallback() {
    for runtime_url in [
        "http://app.shiju.cloud/app/01K1APP0000000000000000001/",
        "https://evil.example/app/01K1APP0000000000000000001/",
        "https://app.shiju.cloud/app/OTHER/",
        "https://app.shiju.cloud/app/01K1APP0000000000000000001/?token=private",
    ] {
        let mut process = MemoryProcess::default();
        let error = launch_with(
            &mut process,
            &[PathBuf::from("/safe/vw-webview")],
            &target(runtime_url),
            "01K1APP0000000000000000001",
        )
        .unwrap_err();
        assert!(error.contains("不受信任"));
        assert!(process.argv.is_empty());
        assert!(process.stdin.is_empty());
    }
}

#[test]
fn launcher_terminates_child_when_stdin_write_fails() {
    let mut process =
        MemoryProcess { write_error: Some("pipe closed".to_string()), ..MemoryProcess::default() };
    let error = launch_with(
        &mut process,
        &[PathBuf::from("/safe/vw-webview")],
        &target("https://app.shiju.cloud/app/01K1APP0000000000000000001/"),
        "01K1APP0000000000000000001",
    )
    .unwrap_err();

    assert!(error.contains("标准输入"));
    assert!(process.terminated);
    assert!(!error.contains("fixture-runtime-session-secret"));
}

#[test]
fn launcher_reports_missing_helper_without_exposing_secret() {
    let mut process =
        MemoryProcess { spawn_error: Some("not found".to_string()), ..MemoryProcess::default() };
    let error = launch_with(
        &mut process,
        &[PathBuf::from("/missing/vw-webview")],
        &target("https://app.shiju.cloud/app/01K1APP0000000000000000001/"),
        "01K1APP0000000000000000001",
    )
    .unwrap_err();

    assert!(error.contains("vw-webview"));
    assert!(!error.contains("fixture-runtime-session-secret"));
}
