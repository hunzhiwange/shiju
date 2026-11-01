//! 将受信任插件钩子声明接入 `HookRunner` 的运行时桥接层。
//!
//! 仅为通过信任校验的声明创建进程处理器，并约束执行超时和输入输出边界。

use anyhow::{Context, Result, anyhow};
use async_trait::async_trait;
use serde_json::{Value, json};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::ExitStatus;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::Command;

use crate::app::agent::config::Config;
use crate::app::agent::hooks::{HookHandler, HookResult, HookRunner};
use crate::app::agent::tools::ToolResult;

use super::{
    PluginDiscoveryOptions, PluginHook, PluginHookEvent, PluginHookTrustStatus,
    PluginHookTrustStore, PluginHooksManifest, discover_plugins, load_plugin_hook_source,
    plugin_hook_trust_file,
};

const DEFAULT_HOOK_TIMEOUT_SECS: u64 = 30;
const MAX_DIAGNOSTIC_CHARS: usize = 512;

/// HookRunner handler that executes one trusted plugin hook command.
/// 将单个已信任命令钩子适配为统一钩子处理器。
pub(crate) struct CommandPluginHookHandler {
    plugin_name: String,
    name: String,
    hook: PluginHook,
    matcher: super::compat::hook_matcher::HookMatcher,
    codex_protocol: bool,
    data_directory: Option<PathBuf>,
    additional_context_limit: Option<usize>,
}

impl CommandPluginHookHandler {
    fn new(plugin_name: String, hook: PluginHook) -> Self {
        let name = format!("plugin:{}:{}", plugin_name, hook.event.as_str());
        Self {
            plugin_name,
            name,
            hook,
            matcher: super::compat::hook_matcher::HookMatcher::All,
            codex_protocol: false,
            data_directory: None,
            additional_context_limit: None,
        }
    }

    fn supports_event(event: PluginHookEvent) -> bool {
        matches!(
            event,
            PluginHookEvent::BeforeToolCall
                | PluginHookEvent::AfterToolCall
                | PluginHookEvent::BeforeLlmCall
        )
    }

    async fn execute_payload(&self, mut payload: Value) -> Result<Value> {
        if self.codex_protocol {
            super::compat::hook_command::bind_input(&mut payload, self.hook.event)?;
        }
        let input =
            serde_json::to_vec(&payload).context("failed to serialize plugin hook payload")?;
        let output = run_hook_command(
            &self.plugin_name,
            &self.hook,
            &input,
            self.codex_protocol,
            self.data_directory.as_deref(),
        )
        .await?;
        if self.codex_protocol && self.hook.event == PluginHookEvent::BeforeLlmCall {
            if !output.stdout.is_empty() || !output.status.success() {
                tracing::warn!("PreModelRequest hook output is observation-only and was ignored");
            }
            return Ok(Value::Null);
        }
        if self.codex_protocol {
            let parsed =
                super::compat::hook_command::interpret(output.status.code(), &output.stdout)?;
            if let Some(text) =
                parsed.pointer("/hookSpecificOutput/additionalContext").and_then(Value::as_str)
            {
                if matches!(
                    self.hook.event,
                    PluginHookEvent::BeforeToolCall | PluginHookEvent::AfterToolCall
                ) {
                    super::compat::hook_command::retain_context(
                        text,
                        self.additional_context_limit,
                        self.data_directory.as_deref().context("missing hook data directory")?,
                    )?;
                } else {
                    tracing::warn!("ignored additionalContext for unsupported hook event");
                }
            }
            return Ok(parsed);
        }
        if output.status.success() {
            return Ok(Value::Null);
        }

        Err(anyhow!(hook_failure_diagnostic(
            &self.plugin_name,
            self.hook.event,
            output.status,
            &output.stdout,
            &output.stderr,
        )))
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl HookHandler for CommandPluginHookHandler {
    fn name(&self) -> &str {
        &self.name
    }

    async fn before_llm_call(
        &self,
        messages: Vec<crate::providers::ChatMessage>,
        model: String,
    ) -> HookResult<(Vec<crate::providers::ChatMessage>, String)> {
        if self.hook.event != PluginHookEvent::BeforeLlmCall {
            return HookResult::Continue((messages, model));
        }
        match self.execute_payload(json!({"event": "before_llm_call", "model": &model})).await {
            Ok(_) => HookResult::Continue((messages, model)),
            Err(error) if self.codex_protocol => {
                tracing::warn!(error = %error, "PreModelRequest hook failed");
                HookResult::Continue((messages, model))
            }
            Err(error) => HookResult::Cancel(error.to_string()),
        }
    }

    async fn before_tool_call(&self, name: String, args: Value) -> HookResult<(String, Value)> {
        if self.hook.event != PluginHookEvent::BeforeToolCall || !self.matcher.matches(Some(&name))
        {
            return HookResult::Continue((name, args));
        }

        let payload = json!({
            "event": self.hook.event.as_str(),
            "plugin": self.plugin_name.as_str(),
            "tool": {
                "name": &name,
                "args": &args,
            },
        });
        match self.execute_payload(payload).await {
            Ok(output) => {
                let specific = &output["hookSpecificOutput"];
                if let Some(updated) = specific.get("updatedInput") {
                    if specific["hookEventName"] != "PreToolUse" || !updated.is_object() {
                        return HookResult::Cancel("invalid Codex updatedInput output".into());
                    }
                    return HookResult::Continue((name, updated.clone()));
                }
                HookResult::Continue((name, args))
            }
            Err(error) => HookResult::Cancel(error.to_string()),
        }
    }

    async fn on_after_tool_call(&self, tool: &str, result: &ToolResult, duration: Duration) {
        if self.hook.event != PluginHookEvent::AfterToolCall || !self.matcher.matches(Some(tool)) {
            return;
        }

        let payload = json!({
            "event": self.hook.event.as_str(),
            "plugin": self.plugin_name.as_str(),
            "tool": {
                "name": tool,
            },
            "result": result,
            "duration_ms": duration_millis(duration),
        });
        if let Err(error) = self.execute_payload(payload).await {
            tracing::warn!(
                hook = self.name.as_str(),
                error = %error,
                "plugin after_tool_call hook failed"
            );
        }
    }
}

/// 发现插件钩子，仅为信任摘要仍匹配当前声明的钩子创建处理器。
///
/// 清单发生任何安全相关变化都会改变摘要，使钩子重新进入待审核状态。
pub(crate) fn trusted_plugin_hook_handlers(config: &Config) -> Result<Vec<Box<dyn HookHandler>>> {
    if !config.hooks.enabled || !config.hooks.plugins.enabled {
        return Ok(Vec::new());
    }

    let options = plugin_hook_discovery_options(config);
    let discovery = discover_plugins(&options).context("failed to discover plugin hooks")?;
    let trust_store =
        PluginHookTrustStore::load_or_default(&plugin_hook_trust_file(&options.config_dir))?;
    let mut handlers: Vec<Box<dyn HookHandler>> = Vec::new();

    for plugin in discovery.enabled {
        for source in &plugin.manifest.hooks {
            let parsed =
                load_plugin_hook_source(source, &plugin.manifest.root, &options.workspace_dir)
                    .with_context(|| {
                        let source_path = match source {
                            super::PluginHookSource::Path(path) => &path.canonical_path,
                            super::PluginHookSource::Inline(_) => &plugin.manifest.manifest_path,
                        };
                        format!(
                            "failed to load hooks for plugin '{}': {}",
                            plugin.manifest.name,
                            source_path.display()
                        )
                    })?;
            let hooks = match parsed {
                PluginHooksManifest::Legacy(hooks) => hooks,
                PluginHooksManifest::Codex(manifest) => {
                    if manifest.hooks.values().all(Vec::is_empty) {
                        continue;
                    }
                    handlers.extend(codex_handlers(
                        &plugin.manifest.name,
                        &plugin.manifest.root,
                        &manifest,
                        &trust_store,
                    )?);
                    continue;
                }
            };

            for hook in hooks {
                if trust_store.review_status(&plugin.manifest.name, &hook)
                    != PluginHookTrustStatus::Trusted
                {
                    tracing::debug!(
                        plugin = plugin.manifest.name.as_str(),
                        event = hook.event.as_str(),
                        "skipping untrusted plugin hook"
                    );
                    continue;
                }
                if !CommandPluginHookHandler::supports_event(hook.event) {
                    tracing::warn!(
                        plugin = plugin.manifest.name.as_str(),
                        event = hook.event.as_str(),
                        "skipping unsupported plugin hook event"
                    );
                    continue;
                }

                handlers.push(Box::new(CommandPluginHookHandler::new(
                    plugin.manifest.name.clone(),
                    hook,
                )));
            }
        }
    }

    Ok(handlers)
}

/// 将当前所有受信任插件钩子注册到运行时。
pub(crate) fn register_trusted_plugin_hooks(
    runner: &mut HookRunner,
    config: &Config,
) -> Result<usize> {
    let handlers = trusted_plugin_hook_handlers(config)?;
    let registered = handlers.len();
    for handler in handlers {
        runner.register(handler);
    }
    Ok(registered)
}

fn plugin_hook_discovery_options(config: &Config) -> PluginDiscoveryOptions {
    PluginDiscoveryOptions::new(
        config.workspace_dir.clone(),
        plugin_config_dir(config),
        config.plugins.clone(),
    )
}

fn plugin_config_dir(config: &Config) -> PathBuf {
    config
        .config_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| config.workspace_dir.clone())
}

struct CommandHookOutput {
    status: ExitStatus,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

async fn run_hook_command(
    plugin_name: &str,
    hook: &PluginHook,
    input: &[u8],
    codex_protocol: bool,
    data_directory: Option<&Path>,
) -> Result<CommandHookOutput> {
    let timeout_secs = hook.timeout_secs.unwrap_or(DEFAULT_HOOK_TIMEOUT_SECS).max(1);
    let mut command = Command::new(&hook.command);
    command
        .args(&hook.args)
        .current_dir(&hook.working_dir)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true);
    if codex_protocol {
        for name in [
            "CODEX_EXEC_SERVER_NOISE_AUTH_TOKEN",
            "NODE_REPL_AUTH_TOKEN",
            "CODEX_GUARDIAN_DECISIONS_API_KEY",
            "OPENAI_FEDERATION_RULE_ID",
            "OPENAI_IDENTITY_TOKEN_FILE",
            "OPENAI_WORKLOAD_IDENTITY_CONTEXT",
        ] {
            command.env_remove(name);
        }
        let data = super::compat::hook_command::prepare_data_directory(
            data_directory.context("missing Codex hook data directory")?,
            &hook.working_dir,
        )?;
        command.env("PLUGIN_DATA", &data);
        command.env("CLAUDE_PLUGIN_DATA", &data);
        command.env("PLUGIN_ROOT", &hook.working_dir);
        command.env("CLAUDE_PLUGIN_ROOT", &hook.working_dir);
        if let Some(context) = crate::tools::current_tool_use_context()
            && let Some(cwd) = context.workspace_root()
        {
            command.current_dir(cwd);
        }
    }
    command.env("VIBEWINDOW_PLUGIN_NAME", plugin_name);
    command.env("VIBEWINDOW_PLUGIN_HOOK_EVENT", hook.event.as_str());
    command.env("VIBEWINDOW_PLUGIN_HOOK_MODE", hook.mode.as_str());

    let (mut child, _process_tree) =
        crate::tools::shell::processes::termination::spawn(&mut command).with_context(|| {
            format!("failed to spawn plugin hook {}:{}", plugin_name, hook.event.as_str())
        })?;
    let stdin = child.stdin.take();
    let stdout = child.stdout.take().context("missing hook stdout")?;
    let stderr = child.stderr.take().context("missing hook stderr")?;
    let execution = async {
        let write = async {
            if let Some(mut stdin) = stdin {
                match stdin.write_all(input).await {
                    Ok(()) => {}
                    Err(error) if error.kind() == ErrorKind::BrokenPipe => {}
                    Err(error) => return Err(anyhow!(error).context("failed to write hook input")),
                }
            }
            Ok(())
        };
        let ((), stdout, stderr, status) =
            tokio::try_join!(write, read_hook_output(stdout), read_hook_output(stderr), async {
                child.wait().await.context("failed to wait for hook")
            },)?;
        Ok::<_, anyhow::Error>(CommandHookOutput { status, stdout, stderr })
    };
    tokio::time::timeout(Duration::from_secs(timeout_secs), execution).await.map_err(|_| {
        anyhow!(
            "plugin hook {}:{} timed out after {timeout_secs}s",
            plugin_name,
            hook.event.as_str()
        )
    })?
}

async fn read_hook_output(reader: impl tokio::io::AsyncRead + Unpin) -> Result<Vec<u8>> {
    const LIMIT: u64 = 1024 * 1024;
    let mut output = Vec::new();
    reader.take(LIMIT + 1).read_to_end(&mut output).await.context("failed to read hook output")?;
    anyhow::ensure!(output.len() as u64 <= LIMIT, "plugin hook output limit exceeded");
    Ok(output)
}

fn duration_millis(duration: Duration) -> u64 {
    duration.as_millis().min(u128::from(u64::MAX)) as u64
}

fn hook_failure_diagnostic(
    plugin_name: &str,
    event: PluginHookEvent,
    status: ExitStatus,
    stdout: &[u8],
    stderr: &[u8],
) -> String {
    let mut parts = vec![format!(
        "plugin hook {}:{} failed with {}",
        plugin_name,
        event.as_str(),
        exit_status_label(status)
    )];
    if let Some(stdout) = stream_diagnostic("stdout", stdout) {
        parts.push(stdout);
    }
    if let Some(stderr) = stream_diagnostic("stderr", stderr) {
        parts.push(stderr);
    }
    parts.join("; ")
}

fn exit_status_label(status: ExitStatus) -> String {
    match status.code() {
        Some(code) => format!("exit status {code}"),
        None => "process terminated by signal".to_string(),
    }
}

fn stream_diagnostic(label: &str, bytes: &[u8]) -> Option<String> {
    if bytes.is_empty() {
        return None;
    }
    let text = String::from_utf8_lossy(bytes);
    let redacted = redact_sensitive_lines(&text);
    Some(format!("{label}: {}", truncate_for_diagnostic(&redacted)))
}

fn redact_sensitive_lines(text: &str) -> String {
    text.lines()
        .map(|line| {
            let lower = line.to_ascii_lowercase();
            if ["api_key", "apikey", "authorization", "bearer ", "password", "secret", "token"]
                .iter()
                .any(|needle| lower.contains(needle))
            {
                "[redacted sensitive hook output line]"
            } else {
                line
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn truncate_for_diagnostic(text: &str) -> String {
    let mut out = String::new();
    for ch in text.chars().take(MAX_DIAGNOSTIC_CHARS) {
        out.push(ch);
    }
    if text.chars().count() > MAX_DIAGNOSTIC_CHARS {
        out.push_str("...[truncated]");
    }
    out
}

/// Admission compiles every matcher before creating any executable handler.
pub(crate) fn codex_handlers(
    name: &str,
    root: &Path,
    manifest: &super::CodexHooksManifest,
    trust: &PluginHookTrustStore,
) -> Result<Vec<Box<dyn HookHandler>>> {
    use super::compat::hook_matcher::HookMatcher;
    use super::{CodexHookEvent as E, CodexHookHandler as H, PluginHookMode};
    let mut admitted = Vec::new();
    for (event, groups) in &manifest.hooks {
        for group in groups {
            let matcher = HookMatcher::new(*event, group.matcher.as_deref())?;
            for handler in &group.hooks {
                let mapped = match event {
                    E::PreToolUse => PluginHookEvent::BeforeToolCall,
                    E::PostToolUse => PluginHookEvent::AfterToolCall,
                    E::PreModelRequest => PluginHookEvent::BeforeLlmCall,
                    _ => {
                        anyhow::bail!("Codex hook execution is not supported for event: {event:?}")
                    }
                };
                let H::Command {
                    command,
                    command_windows,
                    timeout,
                    r#async,
                    additional_context_limit,
                    ..
                } = handler
                else {
                    anyhow::bail!("Codex hook execution is not supported for non-command handlers");
                };
                anyhow::ensure!(
                    !r#async,
                    "Codex hook execution is not supported for async handlers"
                );
                #[cfg(windows)]
                let (program, args) = (
                    "cmd.exe",
                    vec!["/C".to_owned(), command_windows.as_ref().unwrap_or(command).clone()],
                );
                #[cfg(not(windows))]
                let (program, args) = {
                    let _ = command_windows;
                    ("/bin/sh", vec!["-lc".to_owned(), command.clone()])
                };
                anyhow::ensure!(!command.trim().is_empty(), "empty Codex hook command");
                let hook = PluginHook {
                    event: mapped,
                    command: program.to_owned(),
                    args,
                    timeout_secs: *timeout,
                    working_dir: root.canonicalize()?,
                    mode: PluginHookMode::Blocking,
                    manifest_path: root.canonicalize()?,
                };
                let mut handler = CommandPluginHookHandler::new(name.to_owned(), hook);
                handler.matcher = matcher.clone();
                handler.codex_protocol = true;
                handler.additional_context_limit = *additional_context_limit;
                use sha2::{Digest, Sha256};
                let identity = format!("{}\0{}", root.canonicalize()?.display(), name);
                handler.data_directory = Some(
                    trust
                        .host_directory()?
                        .join("plugin-hook-data")
                        .join(hex::encode(Sha256::digest(identity.as_bytes()))),
                );
                admitted.push(Box::new(handler) as Box<dyn HookHandler>);
            }
        }
    }
    if !trust.codex_hooks_trusted(name, root, manifest)? {
        admitted.clear();
    }
    if admitted.is_empty() {
        return Ok(Vec::new());
    }
    Ok(vec![Box::new(OrderedCodexHooks {
        name: format!("plugin:{name}:codex"),
        handlers: admitted,
    })])
}

/// Keep declaration order even when the host broadcasts observer events concurrently.
struct OrderedCodexHooks {
    name: String,
    handlers: Vec<Box<dyn HookHandler>>,
}
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl HookHandler for OrderedCodexHooks {
    fn name(&self) -> &str {
        &self.name
    }
    async fn before_tool_call(
        &self,
        mut name: String,
        mut args: Value,
    ) -> HookResult<(String, Value)> {
        for handler in &self.handlers {
            match handler.before_tool_call(name, args).await {
                HookResult::Continue(value) => (name, args) = value,
                HookResult::Cancel(reason) => return HookResult::Cancel(reason),
            }
        }
        HookResult::Continue((name, args))
    }
    async fn before_llm_call(
        &self,
        mut messages: Vec<crate::providers::ChatMessage>,
        mut model: String,
    ) -> HookResult<(Vec<crate::providers::ChatMessage>, String)> {
        for handler in &self.handlers {
            match handler.before_llm_call(messages, model).await {
                HookResult::Continue(value) => (messages, model) = value,
                HookResult::Cancel(reason) => return HookResult::Cancel(reason),
            }
        }
        HookResult::Continue((messages, model))
    }
    async fn on_after_tool_call(&self, tool: &str, result: &ToolResult, duration: Duration) {
        for handler in &self.handlers {
            handler.on_after_tool_call(tool, result, duration).await;
        }
    }
}
