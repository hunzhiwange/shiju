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

type AsyncHookOwners = std::sync::Arc<
    std::sync::Mutex<std::collections::HashMap<String, Vec<std::sync::Weak<super::AsyncHooks>>>>,
>;

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
    trusted_content: Option<String>,
    mcp: Option<super::CodexHookHandler>,
    data_directory: Option<PathBuf>,
    additional_context_limit: Option<usize>,
    asynchronous: bool,
    active_async: AsyncHookOwners,
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
            trusted_content: None,
            mcp: None,
            data_directory: None,
            additional_context_limit: None,
            asynchronous: false,
            active_async: Default::default(),
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
        if let Some(expected) = &self.trusted_content {
            let current = super::compat::hook_trust_content::plugin_content_hash(
                &self.hook.working_dir,
                &|| false,
            )?;
            anyhow::ensure!(
                &current == expected,
                "plugin contents changed; hook approval required"
            );
        }
        if self.codex_protocol
            && !matches!(
                self.hook.event,
                PluginHookEvent::SessionStart
                    | PluginHookEvent::SessionEnd
                    | PluginHookEvent::UserPromptSubmit
                    | PluginHookEvent::BeforeLlmCall
                    | PluginHookEvent::PostModelRequest
                    | PluginHookEvent::PreCompact
                    | PluginHookEvent::PostCompact
                    | PluginHookEvent::SubagentStart
                    | PluginHookEvent::Stop
                    | PluginHookEvent::Interrupt
                    | PluginHookEvent::SubagentStop
            )
        {
            super::compat::hook_command::bind_input(&mut payload, self.hook.event)?;
        }
        let input =
            serde_json::to_vec(&payload).context("failed to serialize plugin hook payload")?;
        if self.asynchronous {
            let context = crate::tools::current_tool_use_context()
                .context("async hook requires active turn")?;
            let owner = context.async_hooks.clone();
            {
                let mut active = self
                    .active_async
                    .lock()
                    .map_err(|_| anyhow!("async hook registry poisoned"))?;
                let owners = active.entry(context.session().to_owned()).or_default();
                owners.retain(|owner| owner.strong_count() > 0);
                if !owners.iter().any(|entry| entry.ptr_eq(&std::sync::Arc::downgrade(&owner))) {
                    owners.push(std::sync::Arc::downgrade(&owner));
                }
            }
            let destination = {
                let mut delivery = context.as_ref().clone();
                delivery.async_hooks = std::sync::Arc::default();
                std::sync::Arc::new(delivery)
            };
            let turn = std::sync::Arc::downgrade(&owner);
            let plugin = self.plugin_name.clone();
            let hook = self.hook.clone();
            let data = self.data_directory.clone();
            let limit = self.additional_context_limit;
            let cancellation = context.abort_token().cloned();
            owner.schedule(async move {
                let execution = run_hook_command(&plugin, &hook, &input, true, data.as_deref());
                let output = if let Some(cancellation) = cancellation {
                    tokio::select! { biased; _ = cancellation.cancelled() => return, output = execution => output }
                } else { execution.await };
                match output {
                    Ok(output) if output.status.success() => {
                        let value = serde_json::from_slice::<Value>(&output.stdout).ok().or_else(|| {
                            (hook.event == PluginHookEvent::UserPromptSubmit
                                && !output.stdout.starts_with(b"{"))
                                .then(|| json!({"hookSpecificOutput":{"additionalContext":String::from_utf8_lossy(&output.stdout).trim()}}))
                        });
                        if let Some(value) = value
                            && let Some(text) = value
                                .pointer("/hookSpecificOutput/additionalContext")
                                .and_then(Value::as_str)
                            && let Some(turn) = turn.upgrade()
                            && !turn.is_closed()
                            && matches!(hook.event, PluginHookEvent::BeforeToolCall | PluginHookEvent::AfterToolCall | PluginHookEvent::UserPromptSubmit)
                            && let Some(data) = data.as_deref()
                        {
                            let text = text.to_owned();
                            crate::tools::scope_tool_use_context(destination, async {
                                if super::compat::hook_command::retain_context(&text, limit, data).is_err() {
                                    tracing::warn!("failed to retain async hook context");
                                }
                            }).await;
                        }
                    }
                    _ => tracing::warn!("asynchronous plugin hook failed"),
                }
            })?;
            return Ok(Value::Null);
        }
        let output = if let Some(handler) = &self.mcp {
            super::compat::hook_mcp::execute(&self.hook.working_dir, handler, &payload).await.map(
                |stdout| {
                    #[cfg(unix)]
                    use std::os::unix::process::ExitStatusExt;
                    #[cfg(windows)]
                    use std::os::windows::process::ExitStatusExt;
                    CommandHookOutput {
                        status: ExitStatus::from_raw(0),
                        stdout,
                        stderr: Vec::new(),
                    }
                },
            )
        } else {
            run_hook_command(
                &self.plugin_name,
                &self.hook,
                &input,
                self.codex_protocol,
                self.data_directory.as_deref(),
            )
            .await
        };
        let output = match output {
            Ok(output) => output,
            Err(_)
                if self.codex_protocol
                    && matches!(
                        self.hook.event,
                        PluginHookEvent::PreCompact
                            | PluginHookEvent::PostCompact
                            | PluginHookEvent::SubagentStart
                            | PluginHookEvent::Stop
                            | PluginHookEvent::Interrupt
                            | PluginHookEvent::SubagentStop
                    ) =>
            {
                tracing::warn!("lifecycle hook execution failed");
                return Ok(Value::Null);
            }
            Err(error) => return Err(error),
        };
        if self.codex_protocol
            && matches!(
                self.hook.event,
                PluginHookEvent::BeforeLlmCall | PluginHookEvent::PostModelRequest
            )
        {
            if !output.stdout.is_empty() || !output.status.success() {
                tracing::warn!("PreModelRequest hook output is observation-only and was ignored");
            }
            return Ok(Value::Null);
        }
        if self.codex_protocol {
            let parsed =
                if matches!(self.hook.event, PluginHookEvent::Stop | PluginHookEvent::Interrupt) {
                    super::compat::hook_stop::parse(
                        output.status.code(),
                        &output.stdout,
                        &output.stderr,
                        self.hook.event == PluginHookEvent::Stop,
                    )?
                } else if matches!(
                    self.hook.event,
                    PluginHookEvent::SubagentStart | PluginHookEvent::SubagentStop
                ) {
                    super::compat::hook_subagent::parse(
                        output.status.code(),
                        &output.stdout,
                        &output.stderr,
                        self.hook.event == PluginHookEvent::SubagentStop,
                    )?
                } else if matches!(
                    self.hook.event,
                    PluginHookEvent::PreCompact | PluginHookEvent::PostCompact
                ) {
                    super::compat::hook_compact::parse(output.status.code(), &output.stdout)?
                } else if self.hook.event == PluginHookEvent::UserPromptSubmit {
                    super::compat::hook_prompt::parse(
                        output.status.code(),
                        &output.stdout,
                        &output.stderr,
                    )?
                } else if self.hook.event == PluginHookEvent::AfterToolCall {
                    super::compat::hook_post_tool::parse(
                        output.status.code(),
                        &output.stdout,
                        &output.stderr,
                    )?
                } else {
                    super::compat::hook_command::interpret(output.status.code(), &output.stdout)?
                };
            if self.hook.event == PluginHookEvent::AfterToolCall {
                super::compat::hook_post_tool::validate(&parsed)?;
            }
            if let Some(text) =
                parsed.pointer("/hookSpecificOutput/additionalContext").and_then(Value::as_str)
            {
                if matches!(
                    self.hook.event,
                    PluginHookEvent::BeforeToolCall
                        | PluginHookEvent::AfterToolCall
                        | PluginHookEvent::UserPromptSubmit
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
    async fn permission_request(&self, name: &str, input: &Value) -> Result<Option<bool>> {
        if self.hook.event != PluginHookEvent::PermissionRequest
            || !self.matcher.matches(Some(name))
        {
            return Ok(None);
        }
        anyhow::ensure!(!name.is_empty() && input.is_object(), "invalid permission request input");
        let output = self.execute_payload(json!({"tool":{"name":name,"args":input}})).await?;
        super::compat::hook_permission::decision(&output)
    }

    fn name(&self) -> &str {
        &self.name
    }

    async fn turn_end(&self, input: &Value) -> Result<super::compat::hook_stop::Outcome> {
        let expected = match self.hook.event {
            PluginHookEvent::Stop => "Stop",
            PluginHookEvent::Interrupt => "Interrupt",
            _ => return Ok(Default::default()),
        };
        if !self.codex_protocol || input["hook_event_name"] != expected {
            return Ok(Default::default());
        }
        super::compat::hook_stop::validate(input)?;
        let output = self.execute_payload(input.clone()).await?;
        Ok(super::compat::hook_stop::outcome(&output, expected == "Stop"))
    }

    async fn subagent(&self, input: &Value) -> Result<Vec<String>> {
        let expected = match self.hook.event {
            PluginHookEvent::SubagentStart => "SubagentStart",
            PluginHookEvent::SubagentStop => "SubagentStop",
            _ => return Ok(Vec::new()),
        };
        if self.codex_protocol && input["hook_event_name"] == expected {
            super::compat::hook_subagent::validate(input)?;
            if self.matcher.matches(input["agent_type"].as_str()) {
                let output = self.execute_payload(input.clone()).await?;
                let contexts = super::compat::hook_subagent::contexts(&output, expected)?;
                if expected == "SubagentStart" {
                    for text in contexts {
                        super::compat::hook_command::retain_context(
                            &text,
                            self.additional_context_limit,
                            self.data_directory
                                .as_deref()
                                .context("missing hook data directory")?,
                        )?;
                    }
                    return Ok(crate::tools::current_tool_use_context()
                        .context("missing subagent hook context")?
                        .take_hook_contexts());
                }
                return Ok(contexts);
            }
        }
        Ok(Vec::new())
    }

    async fn compact(&self, input: &Value) -> Result<()> {
        let expected = match self.hook.event {
            PluginHookEvent::PreCompact => "PreCompact",
            PluginHookEvent::PostCompact => "PostCompact",
            _ => return Ok(()),
        };
        if self.codex_protocol && input["hook_event_name"] == expected {
            super::compat::hook_compact::validate(input)?;
            if self.matcher.matches(input["trigger"].as_str()) {
                self.execute_payload(input.clone()).await?;
            }
        }
        Ok(())
    }

    async fn model_request(&self, input: &Value) {
        let expected = match self.hook.event {
            PluginHookEvent::BeforeLlmCall => "PreModelRequest",
            PluginHookEvent::PostModelRequest => "PostModelRequest",
            _ => return,
        };
        if self.codex_protocol && input["hook_event_name"] == expected {
            if let Err(error) = super::compat::hook_model::validate(input) {
                tracing::warn!(%error, "invalid model hook input");
                return;
            }
            if self.execute_payload(input.clone()).await.is_err() {
                tracing::warn!("model observation hook failed");
            }
        }
    }

    async fn user_prompt_submit(&self, input: &Value) -> Result<()> {
        if self.hook.event != PluginHookEvent::UserPromptSubmit {
            return Ok(());
        }
        super::compat::hook_prompt::validate(input)?;
        self.execute_payload(input.clone()).await?;
        Ok(())
    }

    async fn start_session(&self, input: &Value) -> Result<()> {
        if self.hook.event != PluginHookEvent::SessionStart {
            return Ok(());
        }
        super::compat::hook_session::validate(input)?;
        if !self.matcher.matches(input["source"].as_str()) {
            return Ok(());
        }
        let output = self.execute_payload(input.clone()).await?;
        if let Some(text) =
            output.pointer("/hookSpecificOutput/additionalContext").and_then(Value::as_str)
        {
            super::compat::hook_command::retain_context(
                text,
                self.additional_context_limit,
                self.data_directory.as_deref().context("missing hook data directory")?,
            )?;
        }
        Ok(())
    }

    async fn on_session_end(&self, session_id: &str, _channel: &str) {
        let owners = self
            .active_async
            .lock()
            .map(|mut active| active.remove(session_id).unwrap_or_default())
            .unwrap_or_default();
        for owner in owners {
            if let Some(owner) = owner.upgrade() {
                owner.shutdown().await;
            }
        }
        if self.hook.event == PluginHookEvent::SessionEnd
            && self.matcher.matches(Some("other"))
            && let Err(error) = self.execute_payload(json!({"hook_event_name":"SessionEnd", "session_id":session_id, "reason":"other", "cwd":crate::tools::current_tool_use_context().and_then(|context|context.workspace_root()), "transcript_path":null})).await
        {
            tracing::warn!(error = %error, "SessionEnd command failed");
        }
    }

    async fn before_llm_call(
        &self,
        messages: Vec<crate::providers::ChatMessage>,
        model: String,
    ) -> HookResult<(Vec<crate::providers::ChatMessage>, String)> {
        if self.codex_protocol || self.hook.event != PluginHookEvent::BeforeLlmCall {
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
            Ok(output) => match super::compat::hook_pre_tool::updated_input(&output) {
                Ok(Some(updated)) => HookResult::Continue((name, updated)),
                Ok(None) => HookResult::Continue((name, args)),
                Err(error) => HookResult::Cancel(error.to_string()),
            },
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
            if self.codex_protocol {
                if let Some(context) = crate::tools::current_tool_use_context() {
                    let _ = context.push_hook_context(format!("PostToolUse hook failed: {error}"));
                }
            }
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
        if let Some(cwd) = serde_json::from_slice::<Value>(input)
            .ok()
            .and_then(|value| value.get("cwd").and_then(Value::as_str).map(str::to_owned))
        {
            command.current_dir(cwd);
        }
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
    let content = super::compat::hook_trust_content::plugin_content_hash(root, &|| false)?;
    let mut admitted = Vec::new();
    let active_async: AsyncHookOwners = Default::default();
    for (event, groups) in &manifest.hooks {
        for group in groups {
            let matcher = HookMatcher::new(*event, group.matcher.as_deref())?;
            for handler in &group.hooks {
                let mapped = match event {
                    E::PermissionRequest => PluginHookEvent::PermissionRequest,
                    E::PreToolUse => PluginHookEvent::BeforeToolCall,
                    E::PostToolUse => PluginHookEvent::AfterToolCall,
                    E::PreModelRequest => PluginHookEvent::BeforeLlmCall,
                    E::SubagentStart => PluginHookEvent::SubagentStart,
                    E::SubagentStop => PluginHookEvent::SubagentStop,
                    E::Stop => PluginHookEvent::Stop,
                    E::Interrupt => PluginHookEvent::Interrupt,
                    E::PreCompact => PluginHookEvent::PreCompact,
                    E::PostCompact => PluginHookEvent::PostCompact,
                    E::PostModelRequest => PluginHookEvent::PostModelRequest,

                    E::UserPromptSubmit => PluginHookEvent::UserPromptSubmit,
                    E::SessionStart => PluginHookEvent::SessionStart,
                    E::SessionEnd => PluginHookEvent::SessionEnd,
                };
                if matches!(handler, H::McpTool { .. }) {
                    if *event == E::SessionEnd {
                        tracing::warn!("SessionEnd MCP hooks are unsupported");
                        continue;
                    }
                    let hook = PluginHook {
                        event: mapped,
                        command: String::new(),
                        args: Vec::new(),
                        timeout_secs: None,
                        working_dir: root.canonicalize()?,
                        mode: PluginHookMode::Blocking,
                        manifest_path: root.canonicalize()?,
                    };
                    let mut executable = CommandPluginHookHandler::new(name.to_owned(), hook);
                    executable.codex_protocol = true;
                    executable.trusted_content = Some(content.clone());
                    executable.matcher = matcher.clone();
                    executable.mcp = Some(handler.clone());
                    use sha2::{Digest, Sha256};
                    let identity = format!("{}\0{}", root.canonicalize()?.display(), name);
                    executable.data_directory = Some(
                        trust
                            .host_directory()?
                            .join("plugin-hook-data")
                            .join(hex::encode(Sha256::digest(identity.as_bytes()))),
                    );

                    admitted.push(Box::new(executable) as Box<dyn HookHandler>);
                    continue;
                }
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
                    timeout_secs: if matches!(*event, E::SessionEnd | E::Interrupt) {
                        Some(timeout.unwrap_or(1).clamp(1, 3))
                    } else {
                        *timeout
                    },
                    working_dir: root.canonicalize()?,
                    mode: PluginHookMode::Blocking,
                    manifest_path: root.canonicalize()?,
                };
                let mut handler = CommandPluginHookHandler::new(name.to_owned(), hook);
                handler.active_async = active_async.clone();
                handler.matcher = matcher.clone();
                handler.codex_protocol = true;
                handler.trusted_content = Some(content.clone());
                anyhow::ensure!(
                    !*r#async || *event != E::PermissionRequest,
                    "asynchronous PermissionRequest hooks are unsupported"
                );
                handler.asynchronous = *r#async && *event != E::SessionEnd;
                if *r#async && *event == E::SessionEnd {
                    tracing::warn!("running async SessionEnd hook synchronously");
                }
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
    async fn turn_end(&self, input: &Value) -> Result<super::compat::hook_stop::Outcome> {
        let mut outcome = super::compat::hook_stop::Outcome::default();
        for handler in &self.handlers {
            outcome.merge(handler.turn_end(input).await?);
        }
        Ok(outcome)
    }
    async fn subagent(&self, input: &Value) -> Result<Vec<String>> {
        let mut contexts = Vec::new();
        for handler in &self.handlers {
            contexts.extend(handler.subagent(input).await?);
        }
        Ok(contexts)
    }

    async fn compact(&self, input: &Value) -> Result<()> {
        for handler in &self.handlers {
            handler.compact(input).await?;
        }
        Ok(())
    }

    async fn model_request(&self, input: &Value) {
        for handler in &self.handlers {
            handler.model_request(input).await;
        }
    }

    async fn user_prompt_submit(&self, input: &Value) -> Result<()> {
        for handler in &self.handlers {
            handler.user_prompt_submit(input).await?;
        }
        Ok(())
    }

    async fn permission_request(&self, name: &str, input: &Value) -> Result<Option<bool>> {
        let mut verdict = None;
        for handler in &self.handlers {
            let decision = handler.permission_request(name, input).await?;
            if decision == Some(false) {
                return Ok(Some(false));
            }
            if decision.is_some() {
                verdict = decision;
            }
        }
        Ok(verdict)
    }

    fn name(&self) -> &str {
        &self.name
    }
    async fn start_session(&self, input: &Value) -> Result<()> {
        for handler in &self.handlers {
            handler.start_session(input).await?;
        }
        Ok(())
    }
    async fn on_session_end(&self, session: &str, channel: &str) {
        for handler in &self.handlers {
            handler.on_session_end(session, channel).await;
        }
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
