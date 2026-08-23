//! 在受限进程边界内校验并启动识矩应用 WebView。

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use reqwest::Url;
use vw_gateway_client::ShijuAppLaunchDto;

const RUNTIME_HOST: &str = "app.shiju.cloud";

/// 抽象受测的进程创建、标准输入写入与失败清理边界。
pub(super) trait LaunchProcess {
    type Child;

    fn spawn(&mut self, program: &Path, args: &[String]) -> Result<Self::Child, String>;
    fn write_stdin(&mut self, child: &mut Self::Child, input: &[u8]) -> Result<(), String>;
    fn terminate(&mut self, child: &mut Self::Child);
}

struct SystemProcess;

impl LaunchProcess for SystemProcess {
    type Child = Child;

    fn spawn(&mut self, program: &Path, args: &[String]) -> Result<Self::Child, String> {
        Command::new(program)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| error.to_string())
    }

    fn write_stdin(&mut self, child: &mut Self::Child, input: &[u8]) -> Result<(), String> {
        let mut stdin =
            child.stdin.take().ok_or_else(|| "无法获取 WebView 标准输入".to_string())?;
        stdin
            .write_all(input)
            .and_then(|_| stdin.flush())
            .map_err(|_| "无法写入 WebView 标准输入".to_string())
    }

    fn terminate(&mut self, child: &mut Self::Child) {
        let _ = child.kill();
        let _ = child.wait();
    }
}

fn validate_target(
    target: &ShijuAppLaunchDto,
    expected_installation_id: &str,
) -> Result<(), String> {
    if target.installation_id != expected_installation_id {
        return Err("识矩应用运行地址不受信任：安装实例不匹配".to_string());
    }
    let url =
        Url::parse(&target.runtime_url).map_err(|_| "识矩应用运行地址不受信任".to_string())?;
    let expected_path = format!("/app/{expected_installation_id}/");
    let trusted = url.scheme() == "https"
        && url.host_str() == Some(RUNTIME_HOST)
        && url.port().is_none()
        && url.username().is_empty()
        && url.password().is_none()
        && url.path() == expected_path
        && url.query().is_none()
        && url.fragment().is_none();
    if !trusted {
        return Err("识矩应用运行地址不受信任".to_string());
    }
    Ok(())
}

/// 校验运行地址与 WebView 程序后启动进程，并通过标准输入传递启动载荷。
pub(super) fn launch_with<P: LaunchProcess>(
    process: &mut P,
    candidates: &[PathBuf],
    target: &ShijuAppLaunchDto,
    expected_installation_id: &str,
) -> Result<P::Child, String> {
    validate_target(target, expected_installation_id)?;
    if target.runtime_set_cookie.is_empty()
        || target.runtime_set_cookie.chars().any(char::is_control)
    {
        return Err("识矩应用浏览器会话格式无效".to_string());
    }

    let args = vec![
        target.runtime_url.clone(),
        format!("--title={}", target.name),
        "--shiju-app-set-cookie-stdin".to_string(),
    ];
    let mut failures = Vec::new();
    for candidate in candidates {
        let mut child = match process.spawn(candidate, &args) {
            Ok(child) => child,
            Err(error) => {
                failures.push(format!("{}: {error}", candidate.display()));
                continue;
            }
        };
        let mut input = target.runtime_set_cookie.as_bytes().to_vec();
        input.push(b'\n');
        if let Err(error) = process.write_stdin(&mut child, &input) {
            process.terminate(&mut child);
            return Err(error);
        }
        return Ok(child);
    }
    Err(format!(
        "无法启动 vw-webview 辅助程序：{}",
        if failures.is_empty() {
            "未找到可用程序".to_string()
        } else {
            failures.join("；")
        }
    ))
}

/// 使用系统进程实现启动识矩应用 WebView。
pub(crate) fn launch_shiju_app(
    target: &ShijuAppLaunchDto,
    expected_installation_id: &str,
) -> Result<Child, String> {
    launch_with(
        &mut SystemProcess,
        &crate::app::webview_runtime::webview_helper_candidates(),
        target,
        expected_installation_id,
    )
}
