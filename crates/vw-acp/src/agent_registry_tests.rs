//! 验证代理注册表合并、命令解析以及跨平台可执行文件发现规则。
//!
//! 隔离的命令、环境和临时路径 fixture 用于验证跨平台解析规则。

use std::{
    collections::HashMap,
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
    sync::{
        LazyLock, Mutex, MutexGuard,
        atomic::{AtomicUsize, Ordering},
    },
};

use crate::types::AcpAgentConfig;

use super::*;

static AGENT_REGISTRY_ENV_TEST_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));
static TEMP_DIR_COUNTER: AtomicUsize = AtomicUsize::new(0);

struct AgentRegistryEnvTestGuard {
    _registry: MutexGuard<'static, ()>,
    _process_env: tokio::sync::MutexGuard<'static, ()>,
}

fn agent_registry_env_test_lock() -> AgentRegistryEnvTestGuard {
    let registry =
        AGENT_REGISTRY_ENV_TEST_LOCK.lock().expect("agent registry env test lock should acquire");
    let process_env = crate::queue_paths::TEST_HOME_ENV_LOCK.blocking_lock();
    AgentRegistryEnvTestGuard { _registry: registry, _process_env: process_env }
}

struct EnvGuard {
    key: &'static str,
    original: Option<OsString>,
}

impl EnvGuard {
    fn set_os(key: &'static str, value: &OsStr) -> Self {
        let original = std::env::var_os(key);
        unsafe { std::env::set_var(key, value) };
        Self { key, original }
    }

    fn remove(key: &'static str) -> Self {
        let original = std::env::var_os(key);
        unsafe { std::env::remove_var(key) };
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

struct TestDir {
    path: PathBuf,
}

impl TestDir {
    fn new(name: &str) -> Self {
        let unique = TEMP_DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir()
            .join(format!("vw-acp-agent-registry-{name}-{}-{unique}", std::process::id()));
        std::fs::create_dir_all(&path).expect("temp dir should be created");
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

fn write_file(path: &Path) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent dir should be created");
    }
    std::fs::write(path, b"#!/bin/sh\n").expect("file should be written");
}

fn test_spec(command: &str, args: &[&str]) -> AgentCommandSpec {
    AgentCommandSpec {
        display_name: "Custom Agent".to_string(),
        command: command.to_string(),
        args: args.iter().map(|arg| (*arg).to_string()).collect(),
        env: HashMap::new(),
    }
}

// 下列行为用例覆盖规范化、发现优先级、兼容输入和缺失值回退。
#[test]
fn command_line_joins_command_and_args_without_mutating_env() {
    let mut env = HashMap::new();
    env.insert("KEY".to_string(), "VALUE".to_string());
    let spec = AgentCommandSpec {
        display_name: "Demo".to_string(),
        command: "demo".to_string(),
        args: vec!["--one".to_string(), "two".to_string()],
        env: env.clone(),
    };

    assert_eq!(spec.command_line(), "demo --one two");
    assert_eq!(AcpAgentConfig::from(&spec).env, env);
}

#[test]
fn normalize_agent_name_trims_and_lowercases() {
    assert_eq!(normalize_agent_name("  Codex CLI  "), "codex cli");
}

#[test]
fn built_in_specs_include_stable_user_keys() {
    let specs = built_in_agent_specs();
    let codex_args = &specs[DEFAULT_AGENT_NAME].args;
    let gemini_args = &specs["gemini"].args;
    let qoder_args = &specs["qoder"].args;
    let qwen_args = &specs["qwen"].args;

    assert!(specs.contains_key(DEFAULT_AGENT_NAME));
    assert!(specs.contains_key("claude"));
    assert_eq!(specs["auggie"].env["AUGMENT_DISABLE_AUTO_UPDATE"], "1");
    assert_eq!(codex_args.first().map(String::as_str), Some("-y"));
    assert!(codex_args.iter().any(|arg| arg == CODEX_ADAPTER_PACKAGE));
    assert!(!codex_args.iter().any(|arg| arg.contains("mcp_servers.node_repl")));
    assert!(gemini_args.windows(2).any(|args| args[0] == "--approval-mode" && args[1] == "yolo"));
    assert!(
        qoder_args
            .windows(2)
            .any(|args| args[0] == "--permission-mode" && args[1] == "bypass_permissions")
    );
    assert!(qwen_args.windows(2).any(|args| args[0] == "--approval-mode" && args[1] == "yolo"));
    assert!(
        built_in_agent_definitions()
            .iter()
            .all(|definition| definition.name == normalize_agent_name(definition.name))
    );
}

#[test]
fn merge_specs_normalizes_legacy_codex_acp_args() {
    let mut overrides = HashMap::new();
    overrides.insert(
        " Codex ".to_string(),
        test_spec(
            "npx",
            &[
                "-y",
                "@zed-industries/codex-acp@latest",
                "-c",
                "mcp_servers.node_repl.enabled=false",
                "--config=mcp_servers.node_repl.enabled=false",
            ],
        ),
    );

    let merged = merge_agent_specs(Some(&overrides));
    let spec = merged.get(DEFAULT_AGENT_NAME).expect("codex override should be normalized");

    assert_eq!(spec.command, "npx");
    assert_eq!(spec.args, ["-y", CODEX_ADAPTER_PACKAGE]);
}

#[test]
fn merge_registry_normalizes_legacy_codex_acp_command_line() {
    let mut overrides = HashMap::new();
    overrides.insert(
        "codex".to_string(),
        test_spec("npx", &["@openai/codex-acp", "-c=mcp_servers.node_repl.enabled=false"]),
    );

    let merged = merge_agent_registry(Some(&overrides));
    let command = merged.get(DEFAULT_AGENT_NAME).expect("codex command should resolve");

    assert_eq!(command, &format!("npx {CODEX_ADAPTER_PACKAGE}"));
}

#[test]
fn opencode_binary_name_uses_platform_executable_name() {
    if cfg!(windows) {
        assert_eq!(opencode_binary_name(), "opencode.exe");
    } else {
        assert_eq!(opencode_binary_name(), "opencode");
    }
}

#[test]
fn resolve_opencode_command_prefers_existing_env_path() {
    let _env_lock = agent_registry_env_test_lock();
    let dir = TestDir::new("opencode-env");
    let binary = dir.path().join(opencode_binary_name());
    write_file(&binary);

    let _opencode_bin_guard = EnvGuard::set_os("OPENCODE_BIN", binary.as_os_str());

    assert_eq!(
        resolve_opencode_command().expect("opencode env path should resolve"),
        binary.to_string_lossy().to_string()
    );
}

#[test]
fn resolve_opencode_command_uses_home_bin_when_env_path_is_missing() {
    let _env_lock = agent_registry_env_test_lock();
    let home = TestDir::new("opencode-home");
    let missing = home.path().join("missing-opencode");
    let binary = home.path().join(".opencode").join("bin").join(opencode_binary_name());
    write_file(&binary);

    let _opencode_bin_guard = EnvGuard::set_os("OPENCODE_BIN", missing.as_os_str());
    let _home_guard = EnvGuard::set_os("HOME", home.path().as_os_str());

    assert_eq!(
        resolve_opencode_command().expect("home opencode path should resolve"),
        binary.to_string_lossy().to_string()
    );
}

#[test]
fn resolve_opencode_command_uses_userprofile_bin_when_home_is_missing() {
    let _env_lock = agent_registry_env_test_lock();
    let userprofile = TestDir::new("opencode-userprofile");
    let missing = userprofile.path().join("missing-opencode");
    let binary = userprofile.path().join(".opencode").join("bin").join(opencode_binary_name());
    write_file(&binary);

    let _opencode_bin_guard = EnvGuard::set_os("OPENCODE_BIN", missing.as_os_str());
    let _home_guard = EnvGuard::remove("HOME");
    let _userprofile_guard = EnvGuard::set_os("USERPROFILE", userprofile.path().as_os_str());

    assert_eq!(
        resolve_opencode_command().expect("USERPROFILE opencode path should resolve"),
        binary.to_string_lossy().to_string()
    );
}

#[test]
fn resolve_opencode_command_falls_back_to_path() {
    let _env_lock = agent_registry_env_test_lock();
    let home = TestDir::new("opencode-path-home");
    let bin_dir = TestDir::new("opencode-path-bin");
    let missing = home.path().join("missing-opencode");
    let binary = bin_dir.path().join(opencode_binary_name());
    write_file(&binary);
    let path_value = std::env::join_paths([bin_dir.path()]).expect("PATH should join");

    let _opencode_bin_guard = EnvGuard::set_os("OPENCODE_BIN", missing.as_os_str());
    let _home_guard = EnvGuard::set_os("HOME", home.path().as_os_str());
    let _path_guard = EnvGuard::set_os("PATH", path_value.as_os_str());

    assert_eq!(
        resolve_opencode_command().expect("PATH opencode binary should resolve"),
        binary.to_string_lossy().to_string()
    );
}

#[test]
fn resolve_local_agent_command_uses_first_available_candidate_from_path() {
    let _env_lock = agent_registry_env_test_lock();
    let home = TestDir::new("local-agent-home");
    let bin_dir = TestDir::new("local-agent-bin");
    let binary = bin_dir.path().join("second-agent");
    write_file(&binary);
    let path_value = std::env::join_paths([bin_dir.path()]).expect("PATH should join");

    let _home_guard = EnvGuard::set_os("HOME", home.path().as_os_str());
    let _path_guard = EnvGuard::set_os("PATH", path_value.as_os_str());

    assert_eq!(
        resolve_local_agent_command(&["first-agent", "second-agent"])
            .expect("second candidate should resolve"),
        binary.to_string_lossy().to_string()
    );
}

#[test]
fn built_in_agent_command_uses_local_candidate_args() {
    let _env_lock = agent_registry_env_test_lock();
    let home = TestDir::new("definition-home");
    let bin_dir = TestDir::new("definition-bin");
    let binary = bin_dir.path().join("demo-agent");
    write_file(&binary);
    let path_value = std::env::join_paths([bin_dir.path()]).expect("PATH should join");
    let definition = BuiltInAgentDefinition {
        name: "demo",
        display_name: "Demo",
        command: "npx",
        args: &["demo@latest", "acp"],
        env: &[],
        local_command_candidates: &["demo-agent"],
        local_args: &["--acp"],
    };

    let _home_guard = EnvGuard::set_os("HOME", home.path().as_os_str());
    let _path_guard = EnvGuard::set_os("PATH", path_value.as_os_str());

    let (command, args) = built_in_agent_command(&definition);

    assert_eq!(command, binary.to_string_lossy().to_string());
    assert_eq!(args, ["--acp"]);
}

#[test]
fn built_in_agent_command_falls_back_to_definition_command() {
    let definition = BuiltInAgentDefinition {
        name: "demo",
        display_name: "Demo",
        command: "npx",
        args: &["demo@latest", "acp"],
        env: &[],
        local_command_candidates: &["missing-demo-agent"],
        local_args: &["--acp"],
    };

    let (command, args) = built_in_agent_command(&definition);

    assert_eq!(command, "npx");
    assert_eq!(args, ["demo@latest", "acp"]);
}

#[test]
fn built_in_agent_command_uses_opencode_resolution_with_acp_arg() {
    let _env_lock = agent_registry_env_test_lock();
    let dir = TestDir::new("opencode-definition");
    let binary = dir.path().join(opencode_binary_name());
    write_file(&binary);
    let definition = BuiltInAgentDefinition {
        name: "opencode",
        display_name: "OpenCode",
        command: "npx",
        args: &["opencode-ai@latest", "acp"],
        env: &[],
        local_command_candidates: &["opencode"],
        local_args: &["ignored"],
    };

    let _opencode_bin_guard = EnvGuard::set_os("OPENCODE_BIN", binary.as_os_str());

    let (command, args) = built_in_agent_command(&definition);

    assert_eq!(command, binary.to_string_lossy().to_string());
    assert_eq!(args, ["acp"]);
}

#[test]
fn merge_specs_ignores_empty_overrides_and_normalizes_keys() {
    let mut overrides = HashMap::new();
    overrides.insert(
        " Custom ".to_string(),
        AgentCommandSpec {
            display_name: " Custom Agent ".to_string(),
            command: " custom-bin ".to_string(),
            args: vec!["--flag".to_string()],
            env: HashMap::new(),
        },
    );
    overrides.insert(
        "empty".to_string(),
        AgentCommandSpec {
            display_name: "Empty".to_string(),
            command: "   ".to_string(),
            args: Vec::new(),
            env: HashMap::new(),
        },
    );

    let merged = merge_agent_specs(Some(&overrides));

    assert_eq!(merged["custom"].display_name, "Custom Agent");
    assert_eq!(merged["custom"].command, "custom-bin");
    assert!(!merged.contains_key("empty"));
}

#[test]
fn merge_agent_registry_applies_non_empty_overrides_as_command_lines() {
    let mut overrides = HashMap::new();
    overrides.insert("  Custom  ".to_string(), test_spec("custom-bin", &["--stdio"]));
    overrides.insert("blank".to_string(), test_spec("   ", &[]));
    overrides.insert("   ".to_string(), test_spec("ignored-bin", &[]));

    let merged = merge_agent_registry(Some(&overrides));

    assert_eq!(merged["custom"], "custom-bin --stdio");
    assert!(!merged.contains_key("blank"));
    assert!(!merged.contains_key(""));
}

#[test]
fn resolve_agent_command_uses_aliases_and_preserves_unknown_input() {
    assert_eq!(resolve_agent_command("codex cli", None), built_in_agent_registry()["codex"]);
    assert_eq!(resolve_agent_command("./custom-agent", None), "./custom-agent");
}

#[test]
fn resolve_agent_spec_uses_aliases_and_rejects_unknown_names() {
    let codex = resolve_agent_spec("codex cli").expect("codex alias should resolve");

    assert_eq!(codex.display_name, built_in_agent_specs()["codex"].display_name);
    assert!(resolve_agent_spec("unknown-agent").is_none());
}

#[test]
fn resolve_agent_spec_with_overrides_prefers_custom_specs_and_alias_targets() {
    let mut overrides = HashMap::new();
    overrides.insert("  Codex  ".to_string(), test_spec("custom-codex", &["--acp"]));

    let direct = resolve_agent_spec_with_overrides("codex", Some(&overrides))
        .expect("direct override should resolve");
    let alias = resolve_agent_spec_with_overrides("codex cli", Some(&overrides))
        .expect("alias override should resolve");

    assert_eq!(direct.command, "custom-codex");
    assert_eq!(direct.args, ["--acp"]);
    assert_eq!(alias, direct);
}

#[test]
fn list_built_in_agents_includes_normalized_overrides_in_sorted_order() {
    let mut overrides = HashMap::new();
    overrides.insert("  zed-custom  ".to_string(), test_spec("zed-custom", &[]));

    let agents = list_built_in_agents(Some(&overrides));

    assert!(agents.windows(2).all(|pair| pair[0] <= pair[1]));
    assert!(agents.contains(&"zed-custom".to_string()));
}
