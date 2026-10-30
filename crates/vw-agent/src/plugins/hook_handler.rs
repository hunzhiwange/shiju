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
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

use crate::app::agent::config::Config;
use crate::app::agent::hooks::{HookHandler, HookResult, HookRunner};
use crate::app::agent::tools::ToolResult;

use super::{
    PluginDiscoveryOptions, PluginHook, PluginHookEvent, PluginHookTrustStatus,
    PluginHookTrustStore, discover_plugins, load_plugin_hooks_manifest, plugin_hook_trust_file,
};

const DEFAULT_HOOK_TIMEOUT_SECS: u64 = 30;
const MAX_DIAGNOSTIC_CHARS: usize = 512;

/// HookRunner handler that executes one trusted plugin hook command.
/// 将单个已信任命令钩子适配为统一钩子处理器。
pub(crate) struct CommandPluginHookHandler {
    plugin_name: String,
    name: String,
    hook: PluginHook,
}

impl CommandPluginHookHandler {
    fn new(plugin_name: String, hook: PluginHook) -> Self {
        let name = format!("plugin:{}:{}", plugin_name, hook.event.as_str());
        Self { plugin_name, name, hook }
    }

    fn supports_event(event: PluginHookEvent) -> bool {
        matches!(
            event,
            PluginHookEvent::BeforeToolCall
                | PluginHookEvent::AfterToolCall
                | PluginHookEvent::BeforeLlmCall
        )
    }

    async fn execute_payload(&self, payload: Value) -> Result<()> {
        let input =
            serde_json::to_vec(&payload).context("failed to serialize plugin hook payload")?;
        let output = run_hook_command(&self.plugin_name, &self.hook, &input).await?;
        if output.status.success() {
            return Ok(());
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
            Ok(()) => HookResult::Continue((messages, model)),
            Err(error) => HookResult::Cancel(error.to_string()),
        }
    }

    async fn before_tool_call(&self, name: String, args: Value) -> HookResult<(String, Value)> {
        if self.hook.event != PluginHookEvent::BeforeToolCall {
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
            Ok(()) => HookResult::Continue((name, args)),
            Err(error) => HookResult::Cancel(error.to_string()),
        }
    }

    async fn on_after_tool_call(&self, tool: &str, result: &ToolResult, duration: Duration) {
        if self.hook.event != PluginHookEvent::AfterToolCall {
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
            let hooks_path = match source {
                super::PluginHookSource::Path(path) => path,
                super::PluginHookSource::Inline(_) => {
                    anyhow::bail!(
                        "inline hook execution is not supported for plugin '{}'",
                        plugin.manifest.name
                    );
                }
            };
            let hooks = load_plugin_hooks_manifest(
                &hooks_path.canonical_path,
                &plugin.manifest.root,
                &options.workspace_dir,
            )
            .with_context(|| {
                format!(
                    "failed to load hooks for plugin '{}': {}",
                    plugin.manifest.name,
                    hooks_path.canonical_path.display()
                )
            })?;

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
    command.env("VIBEWINDOW_PLUGIN_NAME", plugin_name);
    command.env("VIBEWINDOW_PLUGIN_HOOK_EVENT", hook.event.as_str());
    command.env("VIBEWINDOW_PLUGIN_HOOK_MODE", hook.mode.as_str());

    let mut child = command.spawn().with_context(|| {
        format!("failed to spawn plugin hook {}:{}", plugin_name, hook.event.as_str())
    })?;

    if let Some(mut stdin) = child.stdin.take() {
        match stdin.write_all(input).await {
            Ok(()) => {}
            Err(error) if error.kind() == ErrorKind::BrokenPipe => {}
            Err(error) => {
                tracing::warn!(
                    plugin = plugin_name,
                    event = hook.event.as_str(),
                    error = %error,
                    "failed to write plugin hook stdin"
                );
            }
        }
    }

    let output = tokio::time::timeout(Duration::from_secs(timeout_secs), child.wait_with_output())
        .await
        .map_err(|_| {
            anyhow!(
                "plugin hook {}:{} timed out after {}s",
                plugin_name,
                hook.event.as_str(),
                timeout_secs
            )
        })?
        .with_context(|| {
            format!("failed to wait for plugin hook {}:{}", plugin_name, hook.event.as_str())
        })?;

    Ok(CommandHookOutput { status: output.status, stdout: output.stdout, stderr: output.stderr })
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
