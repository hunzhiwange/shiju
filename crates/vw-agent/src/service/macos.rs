use crate::app::agent::config::Config;
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::common::{build_launchd_env_vars, run_capture, run_checked, xml_escape};

/// macOS launchd 服务的标识符标签。
pub(super) const SERVICE_LABEL: &str = "com.vibewindow.daemon";

pub(super) fn config_dir_arg(config: &Config) -> Option<String> {
    config.config_path.parent().map(|path| path.display().to_string())
}

pub(super) fn install_macos(config: &Config) -> Result<()> {
    let file = macos_service_file()?;
    if let Some(parent) = file.parent() {
        fs::create_dir_all(parent)?;
    }

    let exe = std::env::current_exe().context("Failed to resolve current executable")?;
    let logs_dir =
        config.config_path.parent().map_or_else(|| PathBuf::from("."), PathBuf::from).join("logs");
    fs::create_dir_all(&logs_dir)?;

    let stdout = logs_dir.join("daemon.stdout.log");
    let stderr = logs_dir.join("daemon.stderr.log");
    let env_block = build_launchd_env_vars();
    let plist =
        build_launchd_plist(&exe, config_dir_arg(config).as_deref(), &stdout, &stderr, &env_block);

    fs::write(&file, plist)?;
    println!("✅ Installed launchd service: {}", file.display());
    println!("   Start with: vibewindow service start");
    Ok(())
}

pub(super) fn build_launchd_plist(
    exe: &Path,
    config_dir: Option<&str>,
    stdout: &Path,
    stderr: &Path,
    env_block: &str,
) -> String {
    let config_dir_arg = config_dir
        .map(|dir| {
            format!("\n    <string>--config-dir</string>\n    <string>{}</string>", xml_escape(dir))
        })
        .unwrap_or_default();

    format!(
        r#"<?xml version=\"1.0\" encoding=\"UTF-8\"?>
<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">
<plist version=\"1.0\">
<dict>
  <key>Label</key>
  <string>{label}</string>
  <key>ProgramArguments</key>
  <array>
    <string>{exe}</string>
    {config_dir_arg}
    <string>daemon</string>
  </array>
  <key>RunAtLoad</key>
  <true/>
  <key>KeepAlive</key>
  <true/>{env_block}
  <key>StandardOutPath</key>
  <string>{stdout}</string>
  <key>StandardErrorPath</key>
  <string>{stderr}</string>
</dict>
</plist>
"#,
        label = SERVICE_LABEL,
        exe = xml_escape(&exe.display().to_string()),
        env_block = env_block,
        stdout = xml_escape(&stdout.display().to_string()),
        stderr = xml_escape(&stderr.display().to_string())
    )
}

pub(super) fn start_macos() -> Result<()> {
    let plist = macos_service_file()?;
    run_checked(Command::new("launchctl").arg("load").arg("-w").arg(&plist))?;
    run_checked(Command::new("launchctl").arg("start").arg(SERVICE_LABEL))?;
    println!("✅ Service started");
    Ok(())
}

pub(super) fn restart_macos() -> Result<()> {
    if let Some(target) = current_launchd_gui_service_target()
        && run_checked(Command::new("launchctl").arg("kickstart").arg("-k").arg(target)).is_ok()
    {
        println!("✅ Service restarted");
        return Ok(());
    }

    start_macos()?;
    println!("✅ Service restarted");
    Ok(())
}

pub(super) fn stop_macos() -> Result<()> {
    let plist = macos_service_file()?;
    let _ = run_checked(Command::new("launchctl").arg("stop").arg(SERVICE_LABEL));
    let _ = run_checked(Command::new("launchctl").arg("unload").arg("-w").arg(&plist));
    println!("✅ Service stopped");
    Ok(())
}

pub(super) fn status_macos() -> Result<()> {
    let out = run_capture(Command::new("launchctl").arg("list"))?;
    let running = out.lines().any(|line| line.contains(SERVICE_LABEL));
    println!("Service: {}", if running { "✅ running/loaded" } else { "❌ not loaded" });
    println!("Unit: {}", macos_service_file()?.display());
    Ok(())
}

pub(super) fn uninstall_macos() -> Result<()> {
    let file = macos_service_file()?;
    if file.exists() {
        fs::remove_file(&file).with_context(|| format!("Failed to remove {}", file.display()))?;
    }
    println!("✅ Service uninstalled ({})", file.display());
    Ok(())
}

pub(super) fn macos_service_file() -> Result<PathBuf> {
    let home = directories::UserDirs::new()
        .map(|u| u.home_dir().to_path_buf())
        .context("Could not find home directory")?;
    Ok(home.join("Library").join("LaunchAgents").join(format!("{SERVICE_LABEL}.plist")))
}

pub(super) fn launchd_gui_service_target_for_uid(uid: &str) -> Option<String> {
    let uid = uid.trim();
    if uid.is_empty() || !uid.chars().all(|ch| ch.is_ascii_digit()) {
        return None;
    }
    Some(format!("gui/{uid}/{SERVICE_LABEL}"))
}

fn current_launchd_gui_service_target() -> Option<String> {
    let output = Command::new("id").arg("-u").output().ok()?;
    if !output.status.success() {
        return None;
    }
    launchd_gui_service_target_for_uid(&String::from_utf8_lossy(&output.stdout))
}
