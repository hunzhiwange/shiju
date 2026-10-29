//! Windows desktop starts the local full daemon without a console or script host.
use super::{GatewayRuntimeEndpoint, background_command, gateway_health};
use std::fs::{self, OpenOptions};
use std::path::Path;
use std::process::{Child, Stdio};
use std::time::{Duration, Instant};

#[cfg(windows)]
pub(super) fn bootstrap(
    cli: &Path,
    config_dir: &Path,
    endpoint: &GatewayRuntimeEndpoint,
    build_id: &str,
) -> Result<(), String> {
    // Retain the child across retries: a slow start must not spawn another daemon.
    static CHILD: std::sync::Mutex<Option<Child>> = std::sync::Mutex::new(None);
    let mut child = CHILD.lock().map_err(|_| "Windows daemon startup lock poisoned".to_string())?;
    ensure_daemon(cli, config_dir, endpoint, build_id, &mut child, super::GATEWAY_STARTUP_TIMEOUT)
}

fn full_daemon_ready(health: &serde_json::Value, build_id: &str) -> bool {
    health.get("build_id").and_then(serde_json::Value::as_str) == Some(build_id)
        && health.pointer("/runtime/components/daemon/status").and_then(serde_json::Value::as_str)
            == Some("ok")
}

fn ensure_daemon(
    cli: &Path,
    config_dir: &Path,
    endpoint: &GatewayRuntimeEndpoint,
    build_id: &str,
    child: &mut Option<Child>,
    timeout: Duration,
) -> Result<(), String> {
    if gateway_health(endpoint).is_some_and(|health| full_daemon_ready(&health, build_id)) {
        return Ok(());
    }

    // An upgrade may have stopped the child retained by a previous startup attempt.
    if let Some(process) = child.as_mut()
        && process.try_wait().map_err(|error| format!("检查服务进程失败: {error}"))?.is_some()
    {
        *child = None;
    }

    let logs = config_dir.join("logs");
    if let Err(error) = fs::create_dir_all(&logs) {
        tracing::warn!(target: "vw_desktop", %error, "cannot create daemon logs; startup will continue");
    }
    // The scheduled cmd wrapper can hold the shared daemon logs exclusively on Windows.
    // Use independent logs and fall back to null output if they cannot be opened.
    let stdout = logs.join(format!("desktop-{}.daemon.stdout.log", std::process::id()));
    let stderr = logs.join(format!("desktop-{}.daemon.stderr.log", std::process::id()));
    if child.is_none() {
        #[cfg(windows)]
        record_startup(&format!(
            "spawning daemon; stdout={}; stderr={}",
            stdout.display(),
            stderr.display()
        ));
        let output = daemon_log_stdio(&stdout);
        let errors = daemon_log_stdio(&stderr);
        *child = Some(
            background_command(cli)
                .arg("--config-dir")
                .arg(config_dir)
                .arg("daemon")
                .current_dir(config_dir)
                .stdin(Stdio::null())
                .stdout(output)
                .stderr(errors)
                .spawn()
                .map_err(|error| format!("启动完整服务失败 {}: {error}", cli.display()))?,
        );
    }

    let started = Instant::now();
    loop {
        let process = child.as_mut().expect("daemon child is retained until exit");
        if let Some(status) =
            process.try_wait().map_err(|error| format!("检查服务进程失败: {error}"))?
        {
            *child = None;
            return Err(format!("完整服务已退出（{status}），请查看 {}", stderr.display()));
        }
        if gateway_health(endpoint).is_some_and(|health| full_daemon_ready(&health, build_id)) {
            return Ok(());
        }
        if started.elapsed() >= timeout {
            return Err(format!(
                "完整服务尚未就绪: {}，日志: {}",
                endpoint.describe(),
                stderr.display()
            ));
        }
        std::thread::sleep(super::GATEWAY_STARTUP_POLL_INTERVAL);
    }
}

// Log availability must never be a prerequisite for starting the service.
fn daemon_log_stdio(path: &Path) -> Stdio {
    match OpenOptions::new().create(true).append(true).open(path) {
        Ok(file) => Stdio::from(file),
        Err(error) => {
            let message = format!(
                "daemon log unavailable {}; continuing with null output: {error}",
                path.display()
            );
            #[cfg(windows)]
            record_startup(&message);
            tracing::warn!(target: "vw_desktop", "{message}");
            Stdio::null()
        }
    }
}

#[cfg(test)]
#[path = "config_cli_bootstrap_windows_tests.rs"]
mod tests;

#[cfg(windows)]
pub(super) fn record_startup(message: &str) {
    use std::io::Write;
    let Ok(home) = super::resolve_home_dir() else { return };
    let logs = vw_config_types::paths::home_config_dir(home).join("logs");
    let _ = fs::create_dir_all(&logs);
    let desktop = std::env::current_exe().unwrap_or_default();
    if let Ok(mut log) =
        OpenOptions::new().create(true).append(true).open(logs.join("desktop-bootstrap.log"))
    {
        let _ = writeln!(log, "{} | {} | {message}", std::process::id(), desktop.display());
    }
    tracing::info!(target: "vw_desktop", "{message}");
}
