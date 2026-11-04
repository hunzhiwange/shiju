//! 内置 ACP 的固定安装命令，供界面展示和网关执行共用。
//!
//! 原生安装器来源：cursor.com/install、app.factory.ai/cli、
//! aaif-goose/goose 的 download_cli、trae.cn/trae-cli、cli.kiro.dev/install。
//! CodeBuddy 使用官方 @tencent-ai/codebuddy-code，Snow 使用 snow-ai；
//! Vibe 使用 mistral-vibe，Hermes 必须安装 hermes-agent 的 acp/cli extras。
//! 同时保持 Unix/Windows 显式映射，禁止从客户端传入脚本、URL 或包名。

/// 安装命令始终按网关操作系统选择，与桌面客户端所在系统无关。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AcpInstallPlatform {
    Unix,
    Windows,
}

/// 仅允许内置包和固定的官方安装器，不执行用户提供的 shell 文本。
pub fn acp_install_command(agent: &str) -> Option<(&'static str, &'static [&'static str])> {
    acp_install_command_for_platform(
        agent,
        if cfg!(windows) { AcpInstallPlatform::Windows } else { AcpInstallPlatform::Unix },
    )
}

/// 纯数据选择，允许在任一宿主上验证 Unix 与 Windows 的安装契约。
pub fn acp_install_command_for_platform(
    agent: &str,
    platform: AcpInstallPlatform,
) -> Option<(&'static str, &'static [&'static str])> {
    let args: &[&str] = match agent {
        "claude" => &["install", "-g", "@agentclientprotocol/claude-agent-acp@^0.26.0"],
        "codex" => &["install", "-g", "@agentclientprotocol/codex-acp@latest"],
        "gemini" => &["install", "-g", "@google/gemini-cli@latest"],
        "auggie" => &["install", "-g", "@augmentcode/auggie@latest"],
        "codebuddy" => &["install", "-g", "@tencent-ai/codebuddy-code"],
        "snow" => &["install", "-g", "snow-ai"],
        "copilot" => &["install", "-g", "@github/copilot"],
        "iflow" => &["install", "-g", "@iflow-ai/iflow-cli"],
        "kilocode" => &["install", "-g", "@kilocode/cli"],
        "opencode" => &["install", "-g", "opencode-ai@latest"],
        "openclaw" => &["install", "-g", "openclaw"],
        "pi" => &["install", "-g", "pi-acp@^0.0.22"],
        "qoder" => &["install", "-g", "@qoder-ai/qodercli@latest"],
        "qwen" => &["install", "-g", "@qwen-code/qwen-code@latest"],
        "kimi" => &["install", "-g", "@moonshot-ai/kimi-code@2.5.0"],
        "vibe" => return Some(("uv", &["tool", "install", "--python", "3.13", "mistral-vibe"])),
        "hermes" => {
            return Some(("uv", &["tool", "install", "--python", "3.13", "hermes-agent[acp,cli]"]));
        }
        "cursor" | "droid" | "goose" | "trae" | "kiro" => {
            return official_installer(agent, platform);
        }
        _ => return None,
    };
    Some(("npm", args))
}

// 固定 URL 和固定参数；下载失败必须使整条安装命令失败。Goose 禁止进入交互配置。
fn official_installer(
    agent: &str,
    platform: AcpInstallPlatform,
) -> Option<(&'static str, &'static [&'static str])> {
    match (agent, platform) {
        ("kiro", AcpInstallPlatform::Unix) => Some((
            "bash",
            &[
                "-c",
                "set -euo pipefail; curl -fsSL --proto '=https' https://cli.kiro.dev/install | bash",
            ],
        )),
        ("kiro", AcpInstallPlatform::Windows) => Some((
            "powershell",
            &[
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "$ErrorActionPreference='Stop'; & ([scriptblock]::Create((Invoke-RestMethod 'https://cli.kiro.dev/install.ps1')))",
            ],
        )),
        ("cursor", AcpInstallPlatform::Unix) => Some((
            "bash",
            &[
                "-c",
                "set -euo pipefail; curl -fsSL --proto '=https' https://cursor.com/install | bash",
            ],
        )),
        ("droid", AcpInstallPlatform::Unix) => Some((
            "bash",
            &[
                "-c",
                "set -euo pipefail; curl -fsSL --proto '=https' https://app.factory.ai/cli | sh",
            ],
        )),
        ("goose", AcpInstallPlatform::Unix) => Some((
            "bash",
            &[
                "-c",
                "set -euo pipefail; export CONFIGURE=false; curl -fsSL --proto '=https' https://github.com/aaif-goose/goose/releases/download/stable/download_cli.sh | bash",
            ],
        )),
        ("trae", AcpInstallPlatform::Unix) => Some((
            "bash",
            &[
                "-c",
                "set -euo pipefail; curl -fsSL --proto '=https' https://trae.cn/trae-cli/install.sh | bash",
            ],
        )),
        ("cursor", AcpInstallPlatform::Windows) => Some((
            "powershell",
            &[
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "$ErrorActionPreference='Stop'; & ([scriptblock]::Create((Invoke-RestMethod 'https://cursor.com/install?win32=true')))",
            ],
        )),
        ("droid", AcpInstallPlatform::Windows) => Some((
            "powershell",
            &[
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "$ErrorActionPreference='Stop'; & ([scriptblock]::Create((Invoke-RestMethod 'https://app.factory.ai/cli/windows')))",
            ],
        )),
        ("goose", AcpInstallPlatform::Windows) => Some((
            "powershell",
            &[
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "$ErrorActionPreference='Stop'; $env:CONFIGURE='false'; & ([scriptblock]::Create((Invoke-RestMethod 'https://github.com/aaif-goose/goose/releases/download/stable/download_cli.ps1')))",
            ],
        )),
        ("trae", AcpInstallPlatform::Windows) => Some((
            "powershell",
            &[
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "$ErrorActionPreference='Stop'; & ([scriptblock]::Create((Invoke-RestMethod 'https://trae.cn/trae-cli/install.ps1')))",
            ],
        )),
        _ => None,
    }
}

#[cfg(test)]
#[path = "acp_install_tests.rs"]
mod tests;
