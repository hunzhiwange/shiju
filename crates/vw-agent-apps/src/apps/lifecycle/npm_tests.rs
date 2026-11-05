use super::npm::install_args;
use vw_api_types::agent_apps::AppId::*;

#[test]
fn cc_apps_npm_install_allowlist_fixed_packages() {
    for (app, package) in [
        (Claude, "@anthropic-ai/claude-code@latest"),
        (Codex, "@openai/codex@latest"),
        (Gemini, "@google/gemini-cli@latest"),
        (GrokBuild, "@xai-official/grok@latest"),
        (OpenCode, "opencode-ai@latest"),
        (OpenClaw, "openclaw@latest"),
        (Pi, "@earendil-works/pi-coding-agent@latest"),
        (Mcode, "@minimax-ai/code@latest"),
        (Kimi, "@moonshot-ai/kimi-code@latest"),
        (Dsh, "@deepseek-ai/dsh@latest"),
    ] {
        let args = install_args(app).expect("reviewed npm app");
        assert_eq!(&args[..3], &["i", "-g", package]);
    }
}

#[test]
fn cc_apps_npm_install_allowlist_scripts_and_engine() {
    for (app, scripts) in [
        (Claude, "--allow-scripts=@anthropic-ai/claude-code"),
        (GrokBuild, "--allow-scripts=@xai-official/grok"),
        (OpenCode, "--allow-scripts=opencode-ai"),
        (Mcode, "--allow-scripts=@minimax-ai/code,better-sqlite3"),
    ] {
        assert_eq!(
            &install_args(app).unwrap()[3..],
            &["--ignore-scripts=false", "--include=optional", scripts]
        );
    }
    assert_eq!(&install_args(Dsh).unwrap()[3..], &["--engine-strict"]);
    for app in [Codex, Gemini, OpenClaw, Pi, Kimi] {
        assert_eq!(install_args(app).unwrap().len(), 3);
    }
}

#[test]
fn cc_apps_npm_install_allowlist_rejects_unsupported_apps_and_raw_input() {
    for app in [ClaudeDesktop, Hermes, Agy, Omp] {
        assert!(install_args(app).is_err());
    }
    for raw in ["@evil/package", "codex;touch /tmp/pwned", "--allow-scripts=*", "grok"] {
        assert!(
            serde_json::from_value::<vw_api_types::agent_apps::AppId>(serde_json::Value::String(
                raw.into()
            ))
            .is_err()
        );
    }
}
