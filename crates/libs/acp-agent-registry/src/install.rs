//! ACP 内置代理安装执行：固定来源、用户级 uv 引导与进程树清理。
use config_acp_install::acp_install_command;
use std::process::Stdio;

/// 按网关所在平台安装内置代理，不接收任意命令、包名或 URL。
pub async fn install_builtin_agent(agent: &str) -> Result<(), String> {
    install(agent, &mut InstallLog::default()).await
}

/// 安装过程仅供发起者查看，调用方无需写入日志或持久化。
#[derive(Default)]
pub struct InstallLog {
    pub steps: Vec<String>,
    deadline: Option<tokio::time::Instant>,
}

impl InstallLog {
    pub(super) fn step(&mut self, label: &str) {
        self.steps.push(label.to_owned());
    }
}

pub async fn install_builtin_agent_with_log(
    agent: &str,
    log: &mut InstallLog,
) -> Result<(), String> {
    let seconds = match agent {
        "aionrs" => 900,
        "cortex-code" => 600,
        _ => 300,
    };
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(seconds);
    log.deadline = Some(deadline);
    install(agent, log).await
}

async fn install(agent: &str, log: &mut InstallLog) -> Result<(), String> {
    log.step("检查网关平台与安装方式");
    if crate::managed_runtime::install(agent, log).await? {
        return Ok(());
    }
    let (program, args) = acp_install_command(agent)
        .ok_or_else(|| "该 ACP 暂无一键安装命令，请使用官方安装器".to_string())?;
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(300);
    if program == "uv"
        && installed_uv().is_none()
        && shell_command::resolve_executable("uv").is_none()
    {
        let (bootstrap, args) = uv_bootstrap_command();
        run_installer(log, "安装 uv", bootstrap, args, deadline).await?;
    }
    // uv 的安装目录可能刚刚创建，不能只依赖网关启动时缓存的 PATH。
    let executable = if program == "uv" {
        installed_uv().unwrap_or_else(|| program.into())
    } else {
        program.into()
    };
    run_installer(log, "运行 ACP 安装器", &executable, args, deadline).await?;
    Ok(())
}

pub(super) async fn ensure_uv(
    log: &mut InstallLog,
    deadline: tokio::time::Instant,
) -> Result<String, String> {
    if let Some(path) = installed_uv().or_else(|| {
        shell_command::resolve_executable("uv").map(|path| path.to_string_lossy().into_owned())
    }) {
        return Ok(path);
    }
    let (program, args) = uv_bootstrap_command();
    run_installer(log, "安装 uv", program, args, deadline).await?;
    installed_uv().ok_or_else(|| "uv bootstrap did not install its executable".into())
}

fn uv_bootstrap_command() -> (&'static str, &'static [&'static str]) {
    if cfg!(windows) {
        (
            "powershell",
            &[
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "$ErrorActionPreference='Stop'; & ([scriptblock]::Create((Invoke-RestMethod 'https://astral.sh/uv/install.ps1')))",
            ],
        )
    } else {
        (
            "bash",
            &[
                "-c",
                "set -euo pipefail; curl -fsSL --proto '=https' https://astral.sh/uv/install.sh | sh",
            ],
        )
    }
}

/// 可在网关主机终端运行的官方 CLI 安装命令。
pub fn manual_install_commands(agent: &str) -> Vec<String> {
    let Some((program, args)) = acp_install_command(agent) else {
        return Vec::new();
    };
    let mut commands = Vec::new();
    if program == "uv" {
        let (bootstrap, args) = uv_bootstrap_command();
        commands.push(manual_shell_line(bootstrap, args));
    }
    commands.push(manual_shell_line(program, args));
    commands
}

fn manual_shell_line(program: &str, args: &[&str]) -> String {
    // Windows 终端使用 PowerShell 单引号；脚本直接在当前 PowerShell 执行，
    // 避免通过旧版 PowerShell 的原生命令参数传递后二次解析。
    if cfg!(windows)
        && program == "powershell"
        && let Some(script) = args.last()
    {
        return format!("& {{ {} }}", script.replace("; exit $LASTEXITCODE", ""));
    }

    shell_line(program, args)
}

fn shell_line(program: &str, args: &[&str]) -> String {
    let quote = |arg: &str| {
        if !arg.is_empty()
            && arg.bytes().all(|b| b.is_ascii_alphanumeric() || b"_@%+=:,./-".contains(&b))
        {
            arg.to_owned()
        } else if cfg!(windows) {
            format!("'{}'", arg.replace('\'', "''"))
        } else {
            format!("'{}'", arg.replace('\'', "'\"'\"'"))
        }
    };
    let command = std::iter::once(program)
        .chain(args.iter().copied())
        .map(quote)
        .collect::<Vec<_>>()
        .join(" ");
    if cfg!(windows) { format!("& {command}") } else { command }
}

pub(super) fn installed_uv() -> Option<String> {
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))?;
    let bin = std::env::var_os("UV_INSTALL_DIR")
        .or_else(|| std::env::var_os("XDG_BIN_HOME"))
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from(home).join(".local/bin"));
    let path = bin.join(if cfg!(windows) { "uv.exe" } else { "uv" });
    path.is_file().then(|| path.to_string_lossy().into_owned())
}

// 请求断开或超时也必须停止 shell 启动的下载、解压等子进程。
struct InstallerTree(Option<u32>);
impl Drop for InstallerTree {
    fn drop(&mut self) {
        if let Some(pid) = self.0.take() {
            #[cfg(unix)]
            unsafe {
                libc::kill(-(pid as i32), libc::SIGKILL);
            }
            #[cfg(windows)]
            let _ = std::process::Command::new("taskkill")
                .args(["/PID", &pid.to_string(), "/T", "/F"])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn();
        }
    }
}

pub(super) async fn run_installer(
    log: &mut InstallLog,
    label: &str,
    program: &str,
    args: &[&str],
    deadline: tokio::time::Instant,
) -> Result<(), String> {
    log.step(label);
    // Keep the actual Windows argument boundaries, including the original script.
    let invocation =
        if cfg!(windows) { format!("{program} {args:?}") } else { shell_line(program, args) };
    log.steps.push(invocation);
    let deadline = log.deadline.map_or(deadline, |limit| limit.min(deadline));
    let timeout_error = || format!("{label}超时，请检查网关主机网络与安装状态");
    if tokio::time::Instant::now() >= deadline {
        return Err(timeout_error());
    }
    let mut command =
        acp_host_io::spawn_command_options::build_spawn_command(program, &Default::default());
    #[cfg(unix)]
    command.process_group(0);
    let mut child = command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(|error| {
            format!("无法启动 {program}，请检查网关主机上的运行环境与 PATH（{error}）")
        })?;
    let mut tree = InstallerTree(child.id());
    let mut stdout = OutputTail::default();
    let mut stderr = OutputTail::default();
    let stdout_pipe = child.stdout.take().expect("installer stdout is piped");
    let stderr_pipe = child.stderr.take().expect("installer stderr is piped");
    // 同时消费两个管道，保留退出或超时前最后一段输出。
    let result = {
        let drain = async {
            tokio::try_join!(stdout.read(stdout_pipe), stderr.read(stderr_pipe)).map(|_| ())
        };
        tokio::pin!(drain);
        tokio::time::timeout_at(deadline, async {
            tokio::select! {
                status = child.wait() => {
                    // A background process can keep a pipe open after the installer exits.
                    if let Ok(output) = tokio::time::timeout(
                        std::time::Duration::from_millis(100), &mut drain,
                    ).await {
                        output?;
                    }
                    status
                }
                output = &mut drain => {
                    output?;
                    child.wait().await
                }
            }
        })
        .await
    };
    stdout.append_to(log, "stdout");
    stderr.append_to(log, "stderr");

    match result {
        Ok(Ok(status)) => {
            tree.0 = None;
            log.steps.push(status.to_string());
            if status.success() { Ok(()) } else { Err(format!("{label}失败（{status}）")) }
        }
        Ok(Err(error)) => Err(format!("{label}进程读取失败（{error}）")),
        Err(_) => {
            drop(tree);
            let _ = child.kill().await;
            let _ = child.wait().await;
            Err(timeout_error())
        }
    }
}

#[derive(Default)]
struct OutputTail {
    bytes: std::collections::VecDeque<u8>,
    truncated: bool,
}

impl OutputTail {
    async fn read(&mut self, mut pipe: impl tokio::io::AsyncRead + Unpin) -> std::io::Result<()> {
        use tokio::io::AsyncReadExt;
        const LIMIT: usize = 8192;
        let mut chunk = [0; 4096];
        loop {
            let size = pipe.read(&mut chunk).await?;
            if size == 0 {
                return Ok(());
            }
            let excess = (self.bytes.len() + size).saturating_sub(LIMIT);
            self.truncated |= excess > 0;
            self.bytes.drain(..excess);
            self.bytes.extend(&chunk[..size]);
        }
    }

    fn append_to(mut self, log: &mut InstallLog, stream: &str) {
        let mut bytes: &[u8] = self.bytes.make_contiguous();
        if self.truncated {
            // 丢弃边界处残缺的一行，避免暴露被截断的凭据后缀。
            bytes =
                bytes.iter().position(|byte| *byte == b'\n').map_or(&[], |end| &bytes[end + 1..]);
        }
        let output = String::from_utf8_lossy(bytes);
        if !output.trim().is_empty() || self.truncated {
            let notice = if self.truncated { "（仅保留输出末尾）" } else { "" };
            log.steps.push(format!("{stream}{notice}\n{}", sanitize_output(&output)));
        }
    }
}

fn sanitize_output(output: &str) -> String {
    const SENSITIVE: &[&str] = &[
        "token",
        "password",
        "passwd",
        "credential",
        "secret",
        "authorization",
        "bearer ",
        "api_key",
        "api-key",
        "apikey",
        "private key",
        "skey",
        "sk-",
        "github_pat_",
        "ghp_",
        "xox",
        "akia",
        "aiza",
        "gho_",
        "ghu_",
        "npm_config_auth",
        "_auth",
        "npm_token",
        "glpat-",
    ];
    let secrets: Vec<String> = std::env::vars_os()
        .filter(|(key, _)| {
            let name = key.to_string_lossy().to_ascii_lowercase();
            SENSITIVE.iter().any(|marker| name.contains(marker)) || name.ends_with("_key")
        })
        .filter_map(|(_, value)| value.into_string().ok())
        .filter(|value| !value.is_empty())
        .collect();
    output
        .lines()
        .map(|line| {
            // Terminal escapes can split sensitive markers across formatting runs.
            if line.chars().any(|c| c.is_control() && c != '\t') {
                return "[已隐藏包含终端控制字符的输出]".to_owned();
            }
            let lower = line.to_ascii_lowercase();
            if SENSITIVE.iter().any(|marker| lower.contains(marker))
                || secrets.iter().any(|value| line.contains(value))
                || line.split("://").skip(1).any(|url| {
                    url.split(|c: char| c == '/' || c.is_whitespace())
                        .next()
                        .is_some_and(|host| host.contains('@'))
                })
            {
                "[已隐藏可能包含凭据的输出]".to_owned()
            } else {
                line.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
#[path = "install_tests.rs"]
mod tests;
