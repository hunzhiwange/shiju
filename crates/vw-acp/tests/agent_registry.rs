//! 代理注册表解析和本地适配器发现测试。
//!
//! 这些用例确保内置代理、别名、动态覆盖和本地二进制优先级保持稳定，
//! 避免 CLI 入口解析到错误的代理命令。

use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use vw_acp::{
    AgentCommandSpec, DEFAULT_AGENT_NAME, built_in_agent_registry, built_in_agent_specs,
    list_built_in_agents, merge_agent_registry, merge_agent_specs, normalize_agent_name,
    resolve_agent_command, resolve_agent_spec,
};

/// 验证内置注册表覆盖 dhb 与 TypeScript 兼容代理。
#[test]
fn built_in_agent_registry_covers_dhb_and_ts_agents() {
    let registry = built_in_agent_registry();

    for name in [
        "auggie", "claude", "codex", "copilot", "cursor", "droid", "gemini", "iflow", "kilocode",
        "kimi", "kiro", "opencode", "openclaw", "pi", "qoder", "qwen", "trae",
    ] {
        assert!(registry.contains_key(name), "missing {name}");
    }
}

/// 验证代理命令解析支持别名和显式覆盖。
#[test]
fn resolve_agent_command_supports_aliases_and_overrides() {
    let mut overrides = HashMap::new();
    overrides.insert(
        " Custom ".to_string(),
        AgentCommandSpec {
            display_name: "Custom".to_string(),
            command: "custom-agent".to_string(),
            args: vec!["--acp".to_string()],
            env: HashMap::new(),
        },
    );

    assert_eq!(normalize_agent_name("  Codex CLI "), "codex cli");
    assert_eq!(resolve_agent_command("factory-droid", None), "droid exec --output-format acp");
    assert_eq!(resolve_agent_command("GitHubCopilot", None), built_in_agent_registry()["copilot"]);
    assert_eq!(resolve_agent_command("Custom", Some(&overrides)), "custom-agent --acp");
}

/// 验证支持的代理能解析为结构化规格。
#[test]
fn resolve_agent_spec_returns_structured_spec_for_supported_agents() {
    let spec = resolve_agent_spec("Claude Code").unwrap();

    assert_eq!(spec.display_name, "Claude Code");
    assert_eq!(spec.command, "npx");
    assert_eq!(
        spec.args,
        vec!["-y".to_string(), "@agentclientprotocol/claude-agent-acp@^0.26.0".to_string()]
    );
    assert!(spec.env.is_empty());
}

/// 验证注册表合并和列表输出顺序保持稳定。
#[test]
fn merge_and_list_are_stable() {
    let mut overrides = HashMap::new();
    overrides.insert(
        "z-agent".to_string(),
        AgentCommandSpec {
            display_name: "z-agent".to_string(),
            command: "zed".to_string(),
            args: vec!["--acp".to_string()],
            env: HashMap::new(),
        },
    );

    let merged = merge_agent_registry(Some(&overrides));
    let listed = list_built_in_agents(Some(&overrides));
    let specs = built_in_agent_specs();

    assert_eq!(DEFAULT_AGENT_NAME, "codex");
    assert_eq!(merged.get("z-agent").map(String::as_str), Some("zed --acp"));
    assert!(listed.contains(&"z-agent".to_string()));
    assert!(specs.contains_key("codex"));
    assert_eq!(
        specs.get("pi").map(|spec| spec.command_line()),
        Some("npx -y pi-acp@^0.0.22".to_string())
    );
}

/// 验证 Codex ACP 旧包名和旧 node_repl 配置参数会被归一化掉。
#[test]
fn codex_overrides_drop_obsolete_node_repl_config() {
    let mut overrides = HashMap::new();
    overrides.insert(
        "Codex".to_string(),
        AgentCommandSpec {
            display_name: "Codex CLI".to_string(),
            command: "npx".to_string(),
            args: vec![
                "-y".to_string(),
                "@zed-industries/codex-acp@latest".to_string(),
                "-c".to_string(),
                "mcp_servers.node_repl.enabled=false".to_string(),
            ],
            env: HashMap::new(),
        },
    );

    let specs = merge_agent_specs(Some(&overrides));
    let spec = specs.get(DEFAULT_AGENT_NAME).expect("codex override should resolve");

    assert_eq!(spec.args, ["-y", "@agentclientprotocol/codex-acp@latest"]);
    assert_eq!(
        merge_agent_registry(Some(&overrides)).get(DEFAULT_AGENT_NAME).map(String::as_str),
        Some("npx -y @agentclientprotocol/codex-acp@latest")
    );
}

struct EnvGuard {
    key: &'static str,
    original: Option<std::ffi::OsString>,
}

impl EnvGuard {
    fn set_os(key: &'static str, value: &std::ffi::OsStr) -> Self {
        let original = std::env::var_os(key);
        unsafe { std::env::set_var(key, value) };
        Self { key, original }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        match &self.original {
            Some(value) => unsafe { std::env::set_var(self.key, value) },
            None => unsafe { std::env::remove_var(self.key) },
        }
    }
}

fn unique_temp_dir() -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("vw-acp-agent-registry-{nanos}-{}", std::process::id()))
}

fn write_file(path: &Path, contents: &str) {
    fs::create_dir_all(path.parent().expect("temp file parent should exist"))
        .expect("temp file parent should be created");
    fs::write(path, contents).expect("temp file should be written");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let mut perms = fs::metadata(path).expect("temp file metadata should exist").permissions();
        perms.set_mode(0o755);
        fs::set_permissions(path, perms).expect("temp file permissions should be updated");
    }
}

/// 验证存在本地适配器二进制时内置规格会优先使用它们。
#[test]
fn built_in_agent_specs_prefer_local_adapter_binaries_when_available() {
    let temp_dir = unique_temp_dir();
    let adapter_name = if cfg!(windows) { "codex-acp.exe" } else { "codex-acp" };
    let adapter_path = temp_dir.join(adapter_name);
    write_file(&adapter_path, "#!/bin/sh\nexit 0\n");

    let mut path_entries = vec![temp_dir.clone()];
    if let Some(value) = std::env::var_os("PATH") {
        path_entries.extend(std::env::split_paths(&value));
    }
    let path_value = std::env::join_paths(path_entries).expect("PATH should be joinable");
    let _path_guard = EnvGuard::set_os("PATH", path_value.as_os_str());

    let specs = built_in_agent_specs();
    let spec = specs.get("codex").expect("codex spec should exist");

    assert_eq!(spec.command, adapter_path.to_string_lossy().to_string());
    assert!(spec.args.is_empty());

    let _ = fs::remove_dir_all(temp_dir);
}
