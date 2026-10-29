//! Exercise native shell construction and the shared Windows process host together.
use super::*;
use crate::tools::shell::processes::{
    ShellProcessScope, ShellProcessSnapshot, ShellProcessState, ShellProcesses,
};
use std::time::{Duration, Instant};

fn run_command(
    host: &ShellProcesses,
    rt: &tokio::runtime::Runtime,
    owner: &ShellProcessScope,
    mut command: tokio::process::Command,
) -> ShellProcessSnapshot {
    // Match the real tool path: inheriting the parent environment would hide
    // missing SYSTEMROOT/USERPROFILE and PowerShell startup failures.
    command.env("VW_SHELL_TEST_API_TOKEN", "must-not-leak");
    crate::tools::shell::apply_allowed_shell_environment(
        &mut command,
        &crate::security::SecurityPolicy::default(),
    );
    assert!(!command.as_std().get_envs().any(|(key, _)| key == "VW_SHELL_TEST_API_TOKEN"));
    let started = rt.block_on(host.start(owner.clone(), move || Ok(command))).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while host.snapshot(owner, &started.process_id).unwrap().state == ShellProcessState::Running {
        assert!(Instant::now() < deadline, "shell command did not exit");
        std::thread::sleep(Duration::from_millis(10));
    }
    let result = host.snapshot(owner, &started.process_id).unwrap();
    assert_eq!(result.state, ShellProcessState::Exited);
    result
}

#[test]
fn shell_process_windows_cmd_executes_general_commands() {
    let root = tempfile::tempdir().unwrap();
    let directory = root.path().join("workspace with spaces");
    std::fs::create_dir_all(directory.join("child folder")).unwrap();
    std::fs::write(directory.join("input data.txt"), "beta\r\nalpha\r\n").unwrap();
    std::fs::write(
        directory.join("script with spaces.cmd"),
        "@echo off\r\necho %~1\r\nexit /b 11\r\n",
    )
    .unwrap();
    let owner = ShellProcessScope::new("tenant".into(), &directory, "commands".into()).unwrap();
    let host = ShellProcesses::new().unwrap();
    let rt = tokio::runtime::Runtime::new().unwrap();
    let runtime = NativeRuntime::new_for_test(Some(ShellProgram {
        kind: ShellKind::Cmd,
        program: PathBuf::from("cmd.exe"),
    }));
    for (script, expected, exit_code) in [
        ("echo builtin-ok", "builtin-ok", 0),
        ("cd /d \"child folder\" && cd", "child folder", 0),
        (r#""%SystemRoot%\System32\sort.exe" "input data.txt""#, "alpha\r\nbeta", 0),
        (
            r#"type "input data.txt" | "%SystemRoot%\System32\sort.exe" > "sorted data.txt" && type "sorted data.txt""#,
            "alpha\r\nbeta",
            0,
        ),
        (r#"if exist "input data.txt" (echo compound-ok) else (exit /b 9)"#, "compound-ok", 0),
        (r#"call "script with spaces.cmd" "argument with spaces""#, "argument with spaces", 11),
        ("echo stderr-ok 1>&2 & exit /b 7", "stderr-ok", 7),
        ("vw_missing_command_72683", "vw_missing_command_72683", 9009),
    ] {
        let command = runtime.build_shell_command(script, &directory).unwrap();
        let result = run_command(&host, &rt, &owner, command);
        assert_eq!(result.exit_code, Some(exit_code), "{script}: {}", result.output);
        assert!(result.output.contains(expected), "{script}: {}", result.output);
    }
    assert_eq!(
        std::fs::read_to_string(directory.join("sorted data.txt")).unwrap(),
        "alpha\r\nbeta\r\n"
    );
}

#[test]
fn shell_process_windows_powershell_executes_general_commands() {
    let root = tempfile::tempdir().unwrap();
    let directory = root.path().join("workspace with spaces");
    std::fs::create_dir(&directory).unwrap();
    let owner = ShellProcessScope::new("tenant".into(), &directory, "powershell".into()).unwrap();
    let host = ShellProcesses::new().unwrap();
    let rt = tokio::runtime::Runtime::new().unwrap();
    for (program, kind) in
        [("powershell.exe", ShellKind::PowerShell), ("pwsh.exe", ShellKind::Pwsh)]
    {
        let Ok(program) = which::which(program) else {
            eprintln!("{program} is not installed; skipping this shell variant");
            continue;
        };
        let runtime =
            NativeRuntime::new_for_test(Some(ShellProgram { kind, program: program.clone() }));
        for (script, expected, exit_code) in [
            ("Write-Output 'builtin-ok'", "builtin-ok", 0),
            (
                "if (-not $env:SystemRoot -or -not $env:USERPROFILE -or -not $env:ComSpec -or -not $env:PATHEXT) { throw 'missing Windows environment' }; if ($env:VW_SHELL_TEST_API_TOKEN) { throw 'secret leaked' }; Write-Output 'environment-ok'",
                "environment-ok",
                0,
            ),
            ("Write-Output '中文输出🙂'; [Console]::Error.WriteLine('中文错误')", "中文输出🙂", 0),
            (
                "'beta', 'alpha' | Sort-Object | Set-Content -LiteralPath 'sorted data.txt'; Get-Content -LiteralPath 'sorted data.txt'",
                "alpha\nbeta",
                0,
            ),
            ("& $env:ComSpec /D /C 'echo external-ok'; exit $LASTEXITCODE", "external-ok", 0),
            ("[Console]::Error.WriteLine('stderr-ok'); exit 7", "stderr-ok", 7),
            ("throw 'failure-ok'", "failure-ok", 1),
        ] {
            let command = runtime.build_shell_command(script, &directory).unwrap();
            let result = run_command(&host, &rt, &owner, command);
            assert_eq!(
                result.exit_code,
                Some(exit_code),
                "{program:?} {script}: {}",
                result.output
            );
            assert!(
                result.output.replace("\r\n", "\n").contains(expected),
                "{program:?} {script}: {}",
                result.output
            );
        }
    }
}

#[test]
fn shell_process_windows_git_bash_executes_general_commands() {
    let Ok(program) = which::which("bash.exe") else {
        eprintln!("Git Bash is not installed; skipping this shell variant");
        return;
    };
    // The system32 bash executable launches WSL, whose remote processes are
    // outside the native Windows process tree. Match native runtime detection.
    if is_windows_wsl_bash_launcher(&program) {
        eprintln!("Only the WSL launcher is available; skipping native Bash");
        return;
    }
    let root = tempfile::tempdir().unwrap();
    let directory = root.path().join("workspace with spaces");
    std::fs::create_dir(&directory).unwrap();
    let owner = ShellProcessScope::new("tenant".into(), &directory, "bash".into()).unwrap();
    let host = ShellProcesses::new().unwrap();
    let rt = tokio::runtime::Runtime::new().unwrap();
    let runtime =
        NativeRuntime::new_for_test(Some(ShellProgram { kind: ShellKind::Bash, program }));
    for (script, expected, exit_code) in [
        ("printf 'builtin-ok'", "builtin-ok", 0),
        ("pwd", "workspace with spaces", 0),
        (
            "printf 'beta\\nalpha\\n' | sort > 'sorted data.txt' && cat 'sorted data.txt'",
            "alpha\nbeta",
            0,
        ),
        ("printf 'stderr-ok' >&2; exit 7", "stderr-ok", 7),
        ("vw_missing_command_72683", "vw_missing_command_72683", 127),
    ] {
        let command = runtime.build_shell_command(script, &directory).unwrap();
        let result = run_command(&host, &rt, &owner, command);
        assert_eq!(result.exit_code, Some(exit_code), "{script}: {}", result.output);
        assert!(
            result.output.replace("\r\n", "\n").contains(expected),
            "{script}: {}",
            result.output
        );
    }
}

#[test]
fn shell_process_windows_tool_executes_powershell_with_filtered_environment() {
    use crate::security::{AutonomyLevel, SecurityPolicy};
    use crate::tools::shell::ShellTool;
    use crate::tools::traits::Tool;
    use std::sync::Arc;

    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("中文文件.txt"), "fixture").unwrap();
    let program = which::which("powershell.exe").expect("Windows PowerShell must be available");
    let runtime =
        NativeRuntime::new_for_test(Some(ShellProgram { kind: ShellKind::PowerShell, program }));
    let security = Arc::new(SecurityPolicy {
        autonomy: AutonomyLevel::Full,
        workspace_only: false,
        allowed_commands: vec!["Get-Location".into(), "Get-ChildItem".into()],
        workspace_dir: root.path().to_path_buf(),
        ..SecurityPolicy::default()
    });
    let tool = ShellTool::new(security, Arc::new(runtime));
    let context = Arc::new(crate::tools::context::ToolUseContext::new(
        uuid::Uuid::new_v4().to_string(),
        Some(root.path().to_string_lossy().to_string()),
    ));
    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt
        .block_on(crate::tools::scope_tool_use_context(
            context,
            tool.call(serde_json::json!({
                "command": "Get-Location; Get-ChildItem -Force",
                "description": "inspect current directory and list all files",
                "approved": true,
            })),
        ))
        .unwrap();
    assert!(result.is_success(), "{:?}", result.data);
    assert_eq!(result.data["exit_code"], 0);
    let output = result.data["output"].as_str().unwrap();
    assert!(output.contains("中文文件.txt"), "{output}");
    assert!(!output.contains('\0'));
}
