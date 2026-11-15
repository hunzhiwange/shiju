//! 顶层 CLI 参数引导与运行计划构建。
//!
//! 本模块位于 CLI 入口和具体命令执行之间，负责把原始 argv、stdin、
//! 当前工作目录与配置文件整理成可执行的计划对象。
//!
//! # 主要阶段
//!
//! - Bootstrap：识别版本查询、队列所有者模式、初始工作目录等早期分支
//! - Runtime Plan：基于解析后的配置生成输出策略、公共命令计划和提示词输入
//! - Prompt Input：统一处理位置参数、文件输入与 stdin 输入三种来源
//!
//! 这样可以把 CLI 入口保持为薄壳，同时让计划构建逻辑具备更好的可测试性。

use std::collections::HashSet;
use std::future::Future;
use std::io::Read;
use std::path::{Component, Path, PathBuf};

use crate::cli::flags::{parse_output_format, resolve_output_policy};
use crate::{
    EXIT_CODE_PERMISSION_DENIED, ExitCode, OutputFormat, OutputPolicy, PermissionStats,
    PromptInput, PromptInputValidationError, QUEUE_OWNER_PROCESS_MARKER, ResolvedAcpxConfig,
    build_public_cli_plan, is_codex_invocation, merge_prompt_source_with_text, parse_prompt_source,
    text_prompt,
};

/// CLI 根命令允许识别的顶层动词集合。
///
/// 该常量主要用于在早期参数分析阶段快速判断当前 argv 的意图，
/// 以便在完整命令树构建前先做模式分流。
pub const TOP_LEVEL_VERBS: &[&str] = &[
    "prompt", "exec", "cancel", "flow", "set-mode", "set", "sessions", "status", "config", "help",
];

/// 性能采集文件中记录的当前进程角色。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PerfCaptureRole {
    Cli,
    QueueOwner,
}

impl PerfCaptureRole {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Cli => "cli",
            Self::QueueOwner => "queue_owner",
        }
    }
}

/// 早期启动阶段产出的 CLI 引导计划。
///
/// 该结构只包含无需依赖完整配置即可判断的信息，
/// 例如是否进入队列所有者模式、是否打印版本、初始工作目录等。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliBootstrapPlan {
    pub cli_args: Vec<String>,
    pub perf_capture_role: PerfCaptureRole,
    pub print_version: bool,
    pub queue_owner_mode: bool,
    pub should_handle_skillflag: bool,
    pub initial_cwd: PathBuf,
    pub requested_json_strict: bool,
    pub suppress_reads: bool,
}

/// 基于配置和 argv 生成的运行期 CLI 计划。
///
/// 与 `CliBootstrapPlan` 相比，这一层已经包含输出策略和公共命令分发结果，
/// 可以直接供入口层执行。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliRuntimePlan {
    pub requested_output_format: OutputFormat,
    pub requested_output_policy: OutputPolicy,
    pub public_cli_plan: crate::PublicCliPlan,
}

/// CLI 计划构建与提示词输入处理过程中的错误集合。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CliCoreError {
    #[error("{0}")]
    PromptInputValidation(String),
    #[error("Prompt from --file is empty")]
    PromptFileEmpty,
    #[error("Prompt is required (pass as argument, --file, or pipe via stdin)")]
    PromptRequired,
    #[error("Prompt from stdin is empty")]
    PromptStdinEmpty,
    #[error("{0}")]
    Io(String),
}

impl From<PromptInputValidationError> for CliCoreError {
    fn from(value: PromptInputValidationError) -> Self {
        Self::PromptInputValidation(value.to_string())
    }
}

impl From<std::io::Error> for CliCoreError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value.to_string())
    }
}

pub fn top_level_verbs() -> HashSet<String> {
    TOP_LEVEL_VERBS.iter().map(|verb| (*verb).to_string()).collect()
}

pub fn command_argv(argv: &[String]) -> &[String] {
    let offset = command_arg_offset(argv);
    if argv.len() > offset { &argv[offset..] } else { &[] }
}

pub fn should_maybe_handle_skillflag(argv: &[String]) -> bool {
    argv.iter().any(|token| token == "--skill" || token.starts_with("--skill="))
}

pub fn is_version_requested(argv: &[String]) -> bool {
    argv.iter().any(|token| token == "--version" || token == "-V")
}

pub fn is_queue_owner_mode(argv: &[String]) -> bool {
    command_argv(argv).first().is_some_and(|token| token == QUEUE_OWNER_PROCESS_MARKER)
}

pub fn build_cli_bootstrap_plan(
    argv: &[String],
    current_dir: impl AsRef<Path>,
) -> CliBootstrapPlan {
    let cli_args = command_argv(argv).to_vec();
    let queue_owner_mode = is_queue_owner_mode(argv);

    CliBootstrapPlan {
        cli_args: cli_args.clone(),
        perf_capture_role: if queue_owner_mode {
            PerfCaptureRole::QueueOwner
        } else {
            PerfCaptureRole::Cli
        },
        print_version: is_version_requested(argv),
        queue_owner_mode,
        should_handle_skillflag: should_maybe_handle_skillflag(argv),
        initial_cwd: detect_initial_cwd(&cli_args, current_dir),
        requested_json_strict: detect_json_strict(&cli_args),
        suppress_reads: cli_args.iter().any(|token| token == "--suppress-reads"),
    }
}

pub fn build_cli_runtime_plan(argv: &[String], config: &ResolvedAcpxConfig) -> CliRuntimePlan {
    let requested_json_strict = detect_json_strict(argv);
    let requested_output_format = detect_requested_output_format(argv, config.format);
    let requested_output_policy = resolve_requested_output_policy(
        requested_output_format,
        requested_json_strict,
        argv.iter().any(|token| token == "--suppress-reads"),
    );
    let public_cli_plan = build_public_cli_plan(argv, config, &top_level_verbs());

    CliRuntimePlan { requested_output_format, requested_output_policy, public_cli_plan }
}

pub async fn read_prompt_input_from_stdin() -> Result<String, CliCoreError> {
    let result = tokio::task::spawn_blocking(|| {
        let mut stdin = std::io::stdin();
        read_prompt_input_from_reader(&mut stdin)
    })
    .await;

    flatten_stdin_read_result(result)
}

fn read_prompt_input_from_reader(reader: &mut impl Read) -> Result<String, std::io::Error> {
    let mut data = String::new();
    reader.read_to_string(&mut data)?;
    Ok(data)
}

fn flatten_stdin_read_result(
    result: Result<Result<String, std::io::Error>, impl ToString>,
) -> Result<String, CliCoreError> {
    match result {
        Ok(Ok(data)) => Ok(data),
        Ok(Err(error)) => Err(CliCoreError::from(error)),
        Err(error) => Err(CliCoreError::Io(error.to_string())),
    }
}

pub async fn read_prompt(
    prompt_parts: &[String],
    file_path: Option<&str>,
    cwd: impl AsRef<Path>,
    stdin_is_tty: bool,
) -> Result<PromptInput, CliCoreError> {
    read_prompt_with_stdin_reader(
        prompt_parts,
        file_path,
        cwd,
        stdin_is_tty,
        read_prompt_input_from_stdin,
    )
    .await
}

async fn read_prompt_with_stdin_reader<F, Fut>(
    prompt_parts: &[String],
    file_path: Option<&str>,
    cwd: impl AsRef<Path>,
    stdin_is_tty: bool,
    read_stdin: F,
) -> Result<PromptInput, CliCoreError>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<String, CliCoreError>>,
{
    if let Some(file_path) = file_path {
        let source =
            read_prompt_file_source_with_stdin_reader(file_path, cwd.as_ref(), read_stdin).await?;
        let prompt = merge_prompt_source_with_text(&source, &prompt_parts.join(" "))?;
        if prompt.is_empty() {
            return Err(CliCoreError::PromptFileEmpty);
        }
        return Ok(prompt);
    }

    let joined = prompt_parts.join(" ");
    if !joined.trim().is_empty() {
        return Ok(text_prompt(joined.trim().to_string()));
    }

    if stdin_is_tty {
        return Err(CliCoreError::PromptRequired);
    }

    prompt_from_stdin_result(read_stdin().await)
}

#[cfg(test)]
async fn read_prompt_file_source(file_path: &str, cwd: &Path) -> Result<String, CliCoreError> {
    read_prompt_file_source_with_stdin_reader(file_path, cwd, read_prompt_input_from_stdin).await
}

async fn read_prompt_file_source_with_stdin_reader<F, Fut>(
    file_path: &str,
    cwd: &Path,
    read_stdin: F,
) -> Result<String, CliCoreError>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<String, CliCoreError>>,
{
    if file_path == "-" {
        return read_stdin().await;
    }

    tokio::fs::read_to_string(resolve_path_like_node(cwd, file_path))
        .await
        .map_err(CliCoreError::from)
}

fn prompt_from_stdin_result(
    source: Result<String, CliCoreError>,
) -> Result<PromptInput, CliCoreError> {
    match source {
        Ok(source) => prompt_from_stdin_source(&source),
        Err(error) => Err(error),
    }
}

fn prompt_from_stdin_source(source: &str) -> Result<PromptInput, CliCoreError> {
    let prompt = parse_prompt_source(source)?;
    if prompt.is_empty() {
        return Err(CliCoreError::PromptStdinEmpty);
    }

    Ok(prompt)
}

pub fn apply_permission_exit_code(
    current_exit_code: ExitCode,
    permission_stats: &PermissionStats,
) -> ExitCode {
    let denied_or_cancelled = permission_stats.denied + permission_stats.cancelled;
    if permission_stats.requested > 0 && permission_stats.approved == 0 && denied_or_cancelled > 0 {
        return EXIT_CODE_PERMISSION_DENIED;
    }
    current_exit_code
}

pub fn resolve_compatible_config_id(
    agent_name: &str,
    agent_command: &str,
    config_id: &str,
) -> String {
    if is_codex_invocation(agent_name, agent_command) && config_id == "thought_level" {
        return "reasoning_effort".to_string();
    }
    config_id.to_string()
}

pub fn resolve_requested_output_policy(
    format: OutputFormat,
    json_strict: bool,
    suppress_reads: bool,
) -> OutputPolicy {
    let mut output_policy = resolve_output_policy(format, json_strict);
    output_policy.suppress_reads = suppress_reads;
    output_policy
}

pub fn detect_initial_cwd(argv: &[String], current_dir: impl AsRef<Path>) -> PathBuf {
    let current_dir = current_dir.as_ref();

    let mut index = 0;
    while index < argv.len() {
        let token = argv[index].as_str();
        if token == "--cwd" {
            if let Some(next) = argv.get(index + 1).filter(|value| value.as_str() != "--") {
                return resolve_path_like_node(current_dir, next);
            }
            break;
        }
        if let Some(value) = token.strip_prefix("--cwd=") {
            let value = value.trim();
            if !value.is_empty() {
                return resolve_path_like_node(current_dir, value);
            }
            break;
        }
        if token == "--" {
            break;
        }
        index += 1;
    }

    normalize_path_like_node(current_dir.to_path_buf())
}

pub fn detect_requested_output_format(argv: &[String], fallback: OutputFormat) -> OutputFormat {
    let mut detected_format = fallback;
    let mut index = 0;

    while index < argv.len() {
        let token = argv[index].as_str();
        if token == "--" {
            break;
        }

        if token == "--json-strict" || token.starts_with("--json-strict=") {
            return OutputFormat::Json;
        }

        if token == "--format" {
            if let Some(raw) = argv.get(index + 1)
                && let Ok(format) = parse_output_format(raw)
            {
                detected_format = format;
            }
            index += 1;
        } else if let Some(raw) = token.strip_prefix("--format=")
            && let Ok(format) = parse_output_format(raw.trim())
        {
            detected_format = format;
        }

        index += 1;
    }

    detected_format
}

pub fn detect_json_strict(argv: &[String]) -> bool {
    argv.iter()
        .take_while(|token| token.as_str() != "--")
        .any(|token| token == "--json-strict" || token.starts_with("--json-strict="))
}

fn resolve_path_like_node(current_dir: &Path, value: &str) -> PathBuf {
    let path = Path::new(value);
    if path.is_absolute() {
        return normalize_path_like_node(path.to_path_buf());
    }
    normalize_path_like_node(current_dir.join(path))
}

fn command_arg_offset(argv: &[String]) -> usize {
    if argv.is_empty() {
        return 0;
    }

    if argv.len() == 1 {
        return 1;
    }

    if is_script_launcher(&argv[0]) || looks_like_script_entry(&argv[1]) {
        return 2;
    }

    1
}

fn is_script_launcher(value: &str) -> bool {
    Path::new(value)
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| matches!(name, "node" | "bun" | "tsx" | "deno"))
}

fn looks_like_script_entry(value: &str) -> bool {
    Path::new(value)
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| matches!(extension, "js" | "cjs" | "mjs" | "ts" | "cts" | "mts"))
}

fn normalize_path_like_node(path: PathBuf) -> PathBuf {
    let mut normalized = PathBuf::new();

    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            Component::RootDir | Component::Prefix(_) | Component::Normal(_) => {
                normalized.push(component.as_os_str());
            }
        }
    }

    if normalized.as_os_str().is_empty() {
        PathBuf::from(std::path::MAIN_SEPARATOR.to_string())
    } else {
        normalized
    }
}

#[cfg(test)]
#[path = "cli_core_tests.rs"]
mod cli_core_tests;
