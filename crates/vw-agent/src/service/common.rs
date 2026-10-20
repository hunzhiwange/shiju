use anyhow::{Context, Result, bail};
use std::process::Command;

/// 需要转发到 launchd/systemd 服务的环境变量列表。
const SERVICE_ENV_VARS: &[&str] = &[
    "VIBEWINDOW_DATABASE_URL",
    "GEMINI_API_KEY",
    "GEMINI_CLI_CLIENT_ID",
    "GEMINI_CLI_CLIENT_SECRET",
    "OPENAI_API_KEY",
    "ANTHROPIC_API_KEY",
    "OPENROUTER_API_KEY",
];

pub(super) fn build_launchd_env_vars() -> String {
    let mut entries = Vec::new();
    for &var in SERVICE_ENV_VARS {
        if let Ok(val) = std::env::var(var) {
            if !val.is_empty() {
                entries.push(format!(
                    "    <key>{}</key>\n    <string>{}</string>",
                    xml_escape(var),
                    xml_escape(&val)
                ));
            }
        }
    }

    if entries.is_empty() {
        String::new()
    } else {
        format!("\n  <key>EnvironmentVariables</key>\n  <dict>\n{}\n  </dict>", entries.join("\n"))
    }
}

pub(super) fn build_systemd_env_vars() -> String {
    let mut lines = Vec::new();
    for &var in SERVICE_ENV_VARS {
        if let Ok(val) = std::env::var(var) {
            if !val.is_empty() {
                lines.push(format!("Environment=\"{var}={val}\""));
            }
        }
    }

    if lines.is_empty() { String::new() } else { format!("{}\n", lines.join("\n")) }
}

pub(super) fn run_checked(command: &mut Command) -> Result<()> {
    let output = command.output().context("Failed to spawn command")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("Command failed: {}", stderr.trim());
    }
    Ok(())
}

pub(super) fn run_capture(command: &mut Command) -> Result<String> {
    let output = command.output().context("Failed to spawn command")?;
    let mut text = String::from_utf8_lossy(&output.stdout).to_string();
    if text.trim().is_empty() {
        text = String::from_utf8_lossy(&output.stderr).to_string();
    }
    Ok(text)
}

pub(super) fn xml_escape(raw: &str) -> String {
    raw.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
