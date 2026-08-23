//! 验证配置命令的载荷构造、各输出格式以及写入错误传播。
//!
//! 命令解析、内存输出与失败 fixture 共同固定公开 CLI 的可观察契约。

use std::collections::HashMap;
use std::fs;
use std::future::Future;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

use crate::cli::flags::GlobalFlags;
use crate::{
    AuthPolicy, ConfigError, InitGlobalConfigFileResult, NonInteractivePermissionPolicy,
    OutputFormat, PermissionMode, ResolvedAcpxConfig, default_global_config_path,
};

use super::*;

struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new(label: &str) -> Self {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after unix epoch")
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("vw-acp-config-command-tests-{label}-{unique}"));
        fs::create_dir_all(&path).expect("create temp dir");
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

struct EnvVarGuard {
    _lock: tokio::sync::MutexGuard<'static, ()>,
    saved: Vec<(&'static str, Option<String>)>,
}

impl EnvVarGuard {
    fn new(keys: &[&'static str]) -> Self {
        let lock = crate::queue_paths::TEST_HOME_ENV_LOCK.blocking_lock();
        let saved = keys.iter().map(|key| (*key, std::env::var(key).ok())).collect();
        Self { _lock: lock, saved }
    }

    fn set(&self, key: &str, value: &Path) {
        unsafe { std::env::set_var(key, value) };
    }

    fn clear(&self, key: &str) {
        unsafe { std::env::remove_var(key) };
    }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        for (key, value) in &self.saved {
            match value {
                Some(value) => unsafe { std::env::set_var(key, value) },
                None => unsafe { std::env::remove_var(key) },
            }
        }
    }
}

struct FailingWriter;

impl Write for FailingWriter {
    fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
        Err(io::Error::other("write failed"))
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn block_on<T>(future: impl Future<Output = T>) -> T {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build tokio runtime")
        .block_on(future)
}

fn config() -> ResolvedAcpxConfig {
    ResolvedAcpxConfig {
        default_agent: "codex".to_string(),
        default_permissions: PermissionMode::ApproveAll,
        non_interactive_permissions: NonInteractivePermissionPolicy::Deny,
        auth_policy: AuthPolicy::Skip,
        ttl_ms: 30_000,
        timeout_ms: Some(1_000),
        queue_max_depth: 8,
        format: OutputFormat::Text,
        agents: HashMap::new(),
        auth: HashMap::new(),
        disable_exec: false,
        mcp_servers: Vec::new(),
        mcp_server_management: Vec::new(),
        global_path: "/tmp/global.json".to_string(),
        project_path: "/tmp/project.json".to_string(),
        has_global_config: true,
        has_project_config: false,
    }
}

fn flags(format: OutputFormat) -> GlobalFlags {
    GlobalFlags {
        agent: None,
        cwd: "/tmp/repo".to_string(),
        auth_policy: Some(AuthPolicy::Skip),
        non_interactive_permissions: NonInteractivePermissionPolicy::Deny,
        json_strict: false,
        suppress_reads: false,
        timeout: None,
        ttl: 30_000,
        verbose: false,
        format,
        model: None,
        allowed_tools: None,
        max_turns: None,
        prompt_retries: None,
        approve_all: false,
        approve_reads: false,
        deny_all: false,
    }
}

// 下列行为用例同时检查格式分支、参数边界和错误传播。
#[test]
fn config_show_payload_includes_paths_and_loaded_flags() {
    let payload = config_show_payload(&config());
    let value = serde_json::to_value(payload).expect("payload json");

    assert_eq!(value["defaultAgent"], "codex");
    assert_eq!(value["paths"]["global"], "/tmp/global.json");
    assert_eq!(value["loaded"]["global"].as_bool(), Some(true));
    assert_eq!(value["loaded"]["project"].as_bool(), Some(false));
}

#[test]
fn config_init_payload_preserves_path_and_created_flag() {
    let payload = config_init_payload(InitGlobalConfigFileResult {
        path: "/tmp/config.json".to_string(),
        created: true,
    });

    assert_eq!(payload.path, "/tmp/config.json");
    assert!(payload.created);
}

#[test]
fn write_config_show_emits_json_for_json_format() {
    let mut output = Vec::new();

    write_config_show(&mut output, &flags(OutputFormat::Json), &config()).expect("write config");
    let value: Value = serde_json::from_slice(&output).expect("json output");

    assert_eq!(value["paths"]["project"], "/tmp/project.json");
    assert_eq!(value["ttl"], 30);
}

#[test]
fn write_config_show_emits_pretty_json_for_text_format() {
    let mut output = Vec::new();

    write_config_show(&mut output, &flags(OutputFormat::Text), &config()).expect("write config");
    let text = String::from_utf8(output).expect("utf8 output");

    assert!(text.contains("\"defaultAgent\": \"codex\""));
    assert!(text.ends_with('\n'));
}

#[test]
fn write_config_show_propagates_writer_errors() {
    let mut text_writer = FailingWriter;
    let text_error = write_config_show(&mut text_writer, &flags(OutputFormat::Text), &config())
        .expect_err("text writer error should bubble up");
    assert!(matches!(text_error, ConfigCommandError::Io(_)));

    let mut json_writer = FailingWriter;
    let json_error = write_config_show(&mut json_writer, &flags(OutputFormat::Json), &config())
        .expect_err("json writer error should bubble up");
    assert!(matches!(json_error, ConfigCommandError::Io(_)));
}

#[test]
fn write_config_init_result_emits_json_for_json_format() {
    let mut output = Vec::new();
    let payload = config_init_payload(InitGlobalConfigFileResult {
        path: "/tmp/config.json".to_string(),
        created: true,
    });

    write_config_init_result(&mut output, &flags(OutputFormat::Json), &payload)
        .expect("write config init");
    let value: Value = serde_json::from_slice(&output).expect("json output");

    assert_eq!(value["path"], "/tmp/config.json");
    assert_eq!(value["created"].as_bool(), Some(true));
}

#[test]
fn write_config_init_result_emits_path_only_for_quiet_format() {
    let mut output = Vec::new();
    let payload = config_init_payload(InitGlobalConfigFileResult {
        path: "/tmp/config.json".to_string(),
        created: true,
    });

    write_config_init_result(&mut output, &flags(OutputFormat::Quiet), &payload)
        .expect("write config init");

    assert_eq!(String::from_utf8(output).expect("utf8 output"), "/tmp/config.json\n");
}

#[test]
fn write_config_init_result_reports_created_config_for_text_format() {
    let mut output = Vec::new();
    let payload = config_init_payload(InitGlobalConfigFileResult {
        path: "/tmp/config.json".to_string(),
        created: true,
    });

    write_config_init_result(&mut output, &flags(OutputFormat::Text), &payload)
        .expect("write config init");

    assert_eq!(String::from_utf8(output).expect("utf8 output"), "Created /tmp/config.json\n");
}

#[test]
fn write_config_init_result_reports_existing_config_for_text_format() {
    let mut output = Vec::new();
    let payload = config_init_payload(InitGlobalConfigFileResult {
        path: "/tmp/config.json".to_string(),
        created: false,
    });

    write_config_init_result(&mut output, &flags(OutputFormat::Text), &payload)
        .expect("write config init");

    assert_eq!(
        String::from_utf8(output).expect("utf8 output"),
        "Config already exists: /tmp/config.json\n"
    );
}

#[test]
fn write_config_init_result_propagates_writer_errors() {
    let payload = config_init_payload(InitGlobalConfigFileResult {
        path: "/tmp/config.json".to_string(),
        created: true,
    });

    let mut json_writer = FailingWriter;
    let json_error =
        write_config_init_result(&mut json_writer, &flags(OutputFormat::Json), &payload)
            .expect_err("json writer error should bubble up");
    assert!(matches!(json_error, ConfigCommandError::Io(_)));

    let mut quiet_writer = FailingWriter;
    let quiet_error =
        write_config_init_result(&mut quiet_writer, &flags(OutputFormat::Quiet), &payload)
            .expect_err("quiet writer error should bubble up");
    assert!(matches!(quiet_error, ConfigCommandError::Io(_)));

    let mut text_writer = FailingWriter;
    let text_error =
        write_config_init_result(&mut text_writer, &flags(OutputFormat::Text), &payload)
            .expect_err("text writer error should bubble up");
    assert!(matches!(text_error, ConfigCommandError::Io(_)));
}

#[test]
fn write_config_init_propagates_default_path_errors() {
    if cfg!(windows) {
        return;
    }

    let env = EnvVarGuard::new(&["HOME"]);
    env.clear("HOME");
    let mut output = Vec::new();

    let error = block_on(write_config_init(&mut output, &flags(OutputFormat::Quiet)))
        .expect_err("missing home should bubble up as a config error");

    assert!(matches!(error, ConfigCommandError::Config(ConfigError::HomeDirUnavailable)));
    assert!(output.is_empty());
}

#[test]
fn write_config_init_creates_default_config_and_writes_quiet_path() {
    let temp = TempDir::new("write-init");
    let env = EnvVarGuard::new(&["HOME"]);
    env.set("HOME", temp.path());
    let expected_path = default_global_config_path().expect("default global path");
    let mut output = Vec::new();

    block_on(write_config_init(&mut output, &flags(OutputFormat::Quiet)))
        .expect("write config init");

    assert_eq!(
        String::from_utf8(output).expect("utf8 output"),
        format!("{}\n", expected_path.to_string_lossy())
    );
    assert!(expected_path.exists());
}

#[test]
fn handle_config_command_dispatches_show_and_init() {
    let temp = TempDir::new("handle-command");
    let env = EnvVarGuard::new(&["HOME"]);
    env.set("HOME", temp.path());
    let expected_path = default_global_config_path().expect("default global path");

    block_on(handle_config_command(ConfigCommand::Show, &flags(OutputFormat::Quiet), &config()))
        .expect("handle show");
    block_on(handle_config_command(ConfigCommand::Init, &flags(OutputFormat::Quiet), &config()))
        .expect("handle init");

    assert!(expected_path.exists());
}

#[test]
fn config_command_variants_are_distinct() {
    assert_eq!(ConfigCommand::Show, ConfigCommand::Show);
    assert_ne!(ConfigCommand::Show, ConfigCommand::Init);
}
