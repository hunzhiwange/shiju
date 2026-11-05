use vw_api_types::agent_apps::AppId;

/// Fixed npm argv for the authorized executor, never a shell command.
/// No caller-supplied package, version, script or extra argument is accepted.
/// The executor must pass these as separate arguments to its authorized npm
/// executable, including on Windows/WSL; do not join them into shell source.
pub fn install_args(app: AppId) -> Result<&'static [&'static str], &'static str> {
    use AppId::*;
    let args: &'static [&'static str] = match app {
        Claude => &[
            "i",
            "-g",
            "@anthropic-ai/claude-code@latest",
            "--ignore-scripts=false",
            "--include=optional",
            "--allow-scripts=@anthropic-ai/claude-code",
        ],
        Codex => &["i", "-g", "@openai/codex@latest"],
        Gemini => &["i", "-g", "@google/gemini-cli@latest"],
        GrokBuild => &[
            "i",
            "-g",
            "@xai-official/grok@latest",
            "--ignore-scripts=false",
            "--include=optional",
            "--allow-scripts=@xai-official/grok",
        ],
        OpenCode => &[
            "i",
            "-g",
            "opencode-ai@latest",
            "--ignore-scripts=false",
            "--include=optional",
            "--allow-scripts=opencode-ai",
        ],
        OpenClaw => &["i", "-g", "openclaw@latest"],
        Pi => &["i", "-g", "@earendil-works/pi-coding-agent@latest"],
        Mcode => &[
            "i",
            "-g",
            "@minimax-ai/code@latest",
            "--ignore-scripts=false",
            "--include=optional",
            "--allow-scripts=@minimax-ai/code,better-sqlite3",
        ],
        Kimi => &["i", "-g", "@moonshot-ai/kimi-code@latest"],
        Dsh => &["i", "-g", "@deepseek-ai/dsh@latest", "--engine-strict"],
        ClaudeDesktop | Hermes | Agy | Omp => {
            return Err("application has no reviewed npm installation recipe");
        }
    };
    Ok(args)
}
