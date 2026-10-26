//! Model-facing shell process protocol. Scope always comes from trusted runtime context.
use super::processes::{
    ShellProcessScope, ShellProcessSnapshot, ShellProcessState, ShellProcesses,
};
use super::*;
use crate::tools::context::current_tool_use_context;
use crate::tools::traits::{ToolCallResult, ToolRenderHint};
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::OnceLock;

struct ProcessMetadata {
    scope: ShellProcessScope,
    semantics: ExitSemantics,
    command: String,
    detector: Option<Arc<SyscallAnomalyDetector>>,
}

fn rejected(message: String) -> ToolCallResult {
    ToolCallResult::from_legacy_result(ToolResult {
        success: false,
        output: message.clone(),
        error: Some(message),
    })
}

fn metadata() -> &'static Mutex<HashMap<String, ProcessMetadata>> {
    static RECORDS: OnceLock<Mutex<HashMap<String, ProcessMetadata>>> = OnceLock::new();
    RECORDS.get_or_init(Mutex::default)
}

pub(super) fn trusted_tenant() -> String {
    // Capture before synchronous bridges replace the caller's task-local runtime.
    serde_json::to_string(&crate::session::ui_store::current_tenant_scope())
        .expect("TenantScope serialization is infallible")
}

fn trusted_scope(tenant: &str, workspace: &Path) -> anyhow::Result<ShellProcessScope> {
    let context = current_tool_use_context()
        .ok_or_else(|| anyhow::anyhow!("Shell process requires a trusted session context"))?;
    if context.session().trim().is_empty()
        || matches!(context.session(), "specs" | "default" | "default-session")
    {
        anyhow::bail!("Shell process requires an explicit trusted session");
    }
    ShellProcessScope::new(tenant.to_string(), workspace, context.session().to_string())
}

impl ShellTool {
    pub(super) async fn call_session(
        &self,
        input: serde_json::Value,
    ) -> anyhow::Result<ToolCallResult> {
        let args = self.parse_args(&input)?;
        // Validate scope before consuming budget or starting a child.
        let scope = trusted_scope(&self.tenant, &self.security.workspace_dir)?;
        if self.security.is_rate_limited() {
            return Ok(rejected("Rate limit exceeded: too many actions in the last hour".into()));
        }
        let plan = match self.validate_execution_plan(&args) {
            Ok(plan) => plan,
            Err(error) => return Ok(rejected(error.to_string())),
        };
        if !self.security.record_action() {
            return Ok(rejected("Rate limit exceeded: action budget exhausted".into()));
        }
        let semantics = ExitSemantics::for_parsed_command(&parse_command(&plan.effective_command));
        let command = plan.effective_command.clone();
        let security = self.security.clone();
        let runtime = self.runtime.clone();
        let host = ShellProcesses::shared()?;
        let snapshot = host
            .start_with_timeout(
                scope.clone(),
                args.timeout_ms.map(Duration::from_millis),
                move || {
                    let mut cmd = if plan.sandbox_decision.use_sandbox {
                        SandboxExecutor::new(plan.sandbox_config).build_command(
                            runtime.as_ref(),
                            &plan.effective_command,
                            &args.workdir,
                        )?
                    } else {
                        runtime.build_shell_command(&plan.effective_command, &args.workdir)?
                    };
                    apply_allowed_shell_environment(&mut cmd, &security);
                    Ok(cmd)
                },
            )
            .await?;
        {
            let mut records = metadata().lock();
            // Mirror the host's bounded retention, including expired terminal entries.
            records.retain(|id, record| host.snapshot(&record.scope, id).is_ok());
            records.insert(
                snapshot.process_id.clone(),
                ProcessMetadata {
                    scope: scope.clone(),
                    semantics,
                    command,
                    detector: self.syscall_detector.clone(),
                },
            );
        }
        let deadline = tokio::time::Instant::now() + Duration::from_millis(args.yield_time_ms);
        let mut observed = snapshot;
        while observed.state == ShellProcessState::Running && tokio::time::Instant::now() < deadline
        {
            observed = host
                .wait(
                    &scope,
                    &observed.process_id,
                    observed.next_cursor,
                    deadline.saturating_duration_since(tokio::time::Instant::now()),
                )
                .await?;
        }
        let snapshot = host.snapshot(&scope, &observed.process_id)?;
        session_result(&scope, snapshot, "bash")
    }
}

fn session_result(
    scope: &ShellProcessScope,
    snapshot: ShellProcessSnapshot,
    title: &str,
) -> anyhow::Result<ToolCallResult> {
    let records = metadata().lock();
    let record = records
        .get(&snapshot.process_id)
        .filter(|record| &record.scope == scope)
        .ok_or_else(|| anyhow::anyhow!("Shell process metadata unavailable"))?;
    let state = match snapshot.state {
        ShellProcessState::Running => "running",
        ShellProcessState::Exited => "exited",
        ShellProcessState::Cancelled => "cancelled",
        ShellProcessState::TimedOut => "timed_out",
    };
    let output = normalize_line_endings(snapshot.output);
    if let Some(detector) = &record.detector {
        let _ = detector.inspect_command_output(&record.command, &output, "", snapshot.exit_code);
    }
    let legacy = match snapshot.state {
        ShellProcessState::Running => {
            ToolResult { success: true, output: output.clone(), error: None }
        }
        ShellProcessState::Exited => tool_result_from_interpretation(
            record.semantics.interpret(snapshot.exit_code),
            String::new(),
            output.clone(),
        ),
        ShellProcessState::Cancelled | ShellProcessState::TimedOut => ToolResult {
            success: false,
            output: output.clone(),
            error: Some(format!("Shell process {state}")),
        },
    };
    let mut result = ToolCallResult::from_legacy_result(legacy);
    for (key, value) in [
        ("process_id", json!(snapshot.process_id)),
        ("state", json!(state)),
        ("next_cursor", json!(snapshot.next_cursor)),
        ("truncated", json!(snapshot.truncated)),
        ("dropped_bytes", json!(snapshot.dropped_bytes)),
    ] {
        result.data[key] = value;
    }
    if snapshot.state != ShellProcessState::Running {
        result.data["exit_code"] = json!(snapshot.exit_code);
    }
    // Running is successful acceptance; only state=exited reports command completion.
    result.model_result = result.data.clone();
    let mut hint = ToolRenderHint::titled(title);
    hint.metadata = result.data.clone();
    result.render_hint = Some(hint);
    Ok(result)
}

/// Reads incremental output from an already authorized command.
pub struct BashWaitTool {
    security: Arc<SecurityPolicy>,
    tenant: String,
}
/// Stops an already authorized command and waits for process-tree cleanup.
pub struct BashCancelTool {
    security: Arc<SecurityPolicy>,
    tenant: String,
}

impl BashWaitTool {
    pub fn new(security: Arc<SecurityPolicy>) -> Self {
        Self { security, tenant: trusted_tenant() }
    }
}
impl BashCancelTool {
    pub fn new(security: Arc<SecurityPolicy>) -> Self {
        Self { security, tenant: trusted_tenant() }
    }
}

fn process_id(input: &serde_json::Value) -> anyhow::Result<&str> {
    input
        .get("process_id")
        .and_then(|value| value.as_str())
        .filter(|id| !id.is_empty())
        .ok_or_else(|| anyhow::anyhow!("Missing 'process_id' parameter"))
}

fn cursor(input: &serde_json::Value) -> anyhow::Result<u64> {
    match input.get("cursor") {
        None => Ok(0),
        Some(value) => {
            value.as_u64().ok_or_else(|| anyhow::anyhow!("'cursor' must be a non-negative integer"))
        }
    }
}

#[async_trait]
impl Tool for BashWaitTool {
    fn name(&self) -> &str {
        "bash_wait"
    }
    fn description(&self) -> &str {
        "Read incremental output from a running bash process. Continue with next_cursor; do not restart the command. No stdin or PTY."
    }
    fn parameters_schema(&self) -> serde_json::Value {
        json!({"type":"object","properties":{
            "process_id":{"type":"string"}, "cursor":{"type":"integer","minimum":0,"default":0},
            "yield_time_ms":{"type":"integer","minimum":250,"maximum":30000,"default":10000}
        },"required":["process_id"]})
    }
    fn spec(&self) -> ToolSpec {
        ToolSpec::new(self.name(), self.description(), self.parameters_schema())
            .with_read_only(true)
            .with_concurrency_safe(true)
    }
    async fn check_permissions(&self, input: &serde_json::Value) -> anyhow::Result<()> {
        let scope = trusted_scope(&self.tenant, &self.security.workspace_dir)?;
        let _ = cursor(input)?;
        let _ = parse_yield_time_ms(input)?;
        ShellProcesses::shared()?.snapshot(&scope, process_id(input)?)?;
        Ok(())
    }
    async fn call(&self, input: serde_json::Value) -> anyhow::Result<ToolCallResult> {
        let scope = trusted_scope(&self.tenant, &self.security.workspace_dir)?;
        let snapshot = ShellProcesses::shared()?
            .wait(
                &scope,
                process_id(&input)?,
                cursor(&input)?,
                Duration::from_millis(parse_yield_time_ms(&input)?),
            )
            .await?;
        session_result(&scope, snapshot, self.name())
    }
    async fn execute(&self, input: serde_json::Value) -> anyhow::Result<ToolResult> {
        let result = self.call(input).await?;
        Ok(ToolResult {
            success: result.is_success(),
            output: result.model_result.to_string(),
            error: result.data["error"].as_str().map(str::to_owned),
        })
    }
}

#[async_trait]
impl Tool for BashCancelTool {
    fn name(&self) -> &str {
        "bash_cancel"
    }
    fn description(&self) -> &str {
        "Cancel a bash process in the current session and reap its process tree. Does not change or reapprove the original command."
    }
    fn parameters_schema(&self) -> serde_json::Value {
        json!({"type":"object","properties":{"process_id":{"type":"string"}},"required":["process_id"]})
    }
    fn spec(&self) -> ToolSpec {
        ToolSpec::new(self.name(), self.description(), self.parameters_schema())
            .with_concurrency_safe(true)
    }
    async fn check_permissions(&self, input: &serde_json::Value) -> anyhow::Result<()> {
        let scope = trusted_scope(&self.tenant, &self.security.workspace_dir)?;
        ShellProcesses::shared()?.snapshot(&scope, process_id(input)?)?;
        Ok(())
    }
    async fn call(&self, input: serde_json::Value) -> anyhow::Result<ToolCallResult> {
        let scope = trusted_scope(&self.tenant, &self.security.workspace_dir)?;
        let snapshot = ShellProcesses::shared()?.cancel(&scope, process_id(&input)?).await?;
        session_result(&scope, snapshot, self.name())
    }
    async fn execute(&self, input: serde_json::Value) -> anyhow::Result<ToolResult> {
        let result = self.call(input).await?;
        Ok(ToolResult {
            success: result.is_success(),
            output: result.model_result.to_string(),
            error: result.data["error"].as_str().map(str::to_owned),
        })
    }
}
