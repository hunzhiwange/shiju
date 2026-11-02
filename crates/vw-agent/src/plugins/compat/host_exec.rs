//! Codex command translation; authorization and process ownership stay with the host.
use crate::tools::traits::{Tool, ToolCallResult, ToolResult};
use async_trait::async_trait;
use serde_json::{Value, json};

pub(crate) struct HostExecTool(pub(crate) crate::tools::shell::ShellTool);

fn translate(input: &Value) -> anyhow::Result<Value> {
    let object =
        input.as_object().ok_or_else(|| anyhow::anyhow!("exec input must be an object"))?;
    for key in object.keys() {
        if !matches!(key.as_str(), "cmd" | "workdir" | "timeout_ms" | "yield_time_ms") {
            anyhow::bail!(
                "Unsupported exec parameter: {key}; use host bash tools for supported process operations"
            );
        }
    }
    let cmd = input
        .get("cmd")
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| anyhow::anyhow!("cmd must be a nonempty string"))?;
    let mut mapped =
        json!({"command":cmd,"description":"Execute plugin command through host policy"});
    if let Some(workdir) = input.get("workdir") {
        anyhow::ensure!(workdir.is_string(), "workdir must be a string");
        mapped["workdir"] = workdir.clone();
    }
    for (source, target) in [("timeout_ms", "timeout"), ("yield_time_ms", "yield_time_ms")] {
        if let Some(value) = input.get(source) {
            anyhow::ensure!(
                value.as_u64().is_some_and(|n| n > 0),
                "{source} must be a positive integer"
            );
            mapped[target] = value.clone();
        }
    }
    Ok(mapped)
}

#[async_trait]
impl Tool for HostExecTool {
    fn name(&self) -> &str {
        "exec_command"
    }
    fn description(&self) -> &str {
        "Execute a command using host security and session isolation. Unsupported execution modes fail explicitly."
    }
    fn parameters_schema(&self) -> Value {
        json!({"type":"object","properties":{"cmd":{"type":"string"},"workdir":{"type":"string"},"timeout_ms":{"type":"integer"},"yield_time_ms":{"type":"integer"}},"required":["cmd"],"additionalProperties":false})
    }
    fn spec(&self) -> crate::tools::traits::ToolSpec {
        crate::tools::traits::ToolSpec::new(
            self.name(),
            self.description(),
            self.parameters_schema(),
        )
        .with_destructive(true)
        .with_concurrency_safe(false)
    }
    async fn check_permissions(&self, input: &Value) -> anyhow::Result<()> {
        self.0.check_permissions(&translate(input)?).await
    }
    async fn call(&self, input: Value) -> anyhow::Result<ToolCallResult> {
        self.0.call(translate(&input)?).await
    }
    async fn execute(&self, input: Value) -> anyhow::Result<ToolResult> {
        self.0.execute(translate(&input)?).await
    }
}
