//! WasmModule 工具测试。
//!
//! 覆盖工具元数据、能力解析、模块列表、运行结果映射、速率限制和运行时类型保护，避免
//! wasm 工具在非 wasm 运行时或非法参数下暴露额外能力。

use super::super::*;
use crate::app::agent::config::WasmRuntimeConfig;
use crate::app::agent::runtime::{NativeRuntime, WasmRuntime};
use crate::app::agent::security::{AutonomyLevel, SecurityPolicy};
use serde_json::json;

fn test_security(workspace_dir: std::path::PathBuf) -> Arc<SecurityPolicy> {
    Arc::new(SecurityPolicy {
        autonomy: AutonomyLevel::Full,
        workspace_dir,
        ..SecurityPolicy::default()
    })
}

fn wasm_tool(workspace_dir: std::path::PathBuf) -> WasmModuleTool {
    let runtime: Arc<dyn RuntimeAdapter> = Arc::new(WasmRuntime::new(WasmRuntimeConfig::default()));
    WasmModuleTool::new(test_security(workspace_dir), runtime)
}

#[cfg(feature = "runtime-wasm")]
fn constant_exit_wasm(exit_code: u8) -> Vec<u8> {
    vec![
        0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, // magic and version
        0x01, 0x05, 0x01, 0x60, 0x00, 0x01, 0x7f, // () -> i32 type
        0x03, 0x02, 0x01, 0x00, // one function using type 0
        0x07, 0x07, 0x01, 0x03, b'r', b'u', b'n', 0x00, 0x00, // export run
        0x0a, 0x06, 0x01, 0x04, 0x00, 0x41, exit_code, 0x0b, // return exit_code
    ]
}

#[cfg(feature = "runtime-wasm")]
fn infinite_loop_wasm() -> Vec<u8> {
    vec![
        0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, // magic and version
        0x01, 0x05, 0x01, 0x60, 0x00, 0x01, 0x7f, // () -> i32 type
        0x03, 0x02, 0x01, 0x00, // one function using type 0
        0x07, 0x07, 0x01, 0x03, b'r', b'u', b'n', 0x00, 0x00, // export run
        0x0a, 0x0b, 0x01, 0x09, 0x00, 0x03, 0x40, 0x0c, 0x00, 0x0b, 0x41, 0x00, 0x0b,
    ]
}

#[test]
fn wasm_module_tool_name() {
    let dir = tempfile::tempdir().unwrap();
    let tool = wasm_tool(dir.path().to_path_buf());
    assert_eq!(tool.name(), "wasm_module");
}

#[test]
fn wasm_module_tool_description_explains_sandbox_source() {
    let dir = tempfile::tempdir().unwrap();
    let tool = wasm_tool(dir.path().to_path_buf());

    assert_eq!(tool.description(), "列出或执行来自 runtime.wasm.tools_dir 的沙箱化 WASM 模块");
}

#[test]
fn parameters_schema_describes_actions_capabilities_and_resource_limits() {
    let dir = tempfile::tempdir().unwrap();
    let schema = wasm_tool(dir.path().to_path_buf()).parameters_schema();

    assert_eq!(schema["type"], json!("object"));
    assert_eq!(schema["required"], json!(["action"]));
    assert_eq!(schema["properties"]["action"]["enum"], json!(["list", "run"]));
    assert_eq!(schema["properties"]["module"]["type"], json!("string"));
    assert_eq!(schema["properties"]["read_workspace"]["type"], json!("boolean"));
    assert_eq!(schema["properties"]["write_workspace"]["type"], json!("boolean"));
    assert_eq!(schema["properties"]["allowed_hosts"]["type"], json!("array"));
    assert_eq!(schema["properties"]["allowed_hosts"]["items"]["type"], json!("string"));
    assert_eq!(schema["properties"]["fuel_override"]["minimum"], json!(0));
    assert_eq!(schema["properties"]["memory_override_mb"]["minimum"], json!(0));
}

#[test]
fn parse_caps_defaults_missing_and_wrong_typed_scalar_fields() {
    let defaults = WasmModuleTool::parse_caps(&json!({})).unwrap();
    assert!(!defaults.read_workspace);
    assert!(!defaults.write_workspace);
    assert!(defaults.allowed_hosts.is_empty());
    assert_eq!(defaults.fuel_override, 0);
    assert_eq!(defaults.memory_override_mb, 0);

    let wrong_types = WasmModuleTool::parse_caps(&json!({
        "read_workspace": "yes",
        "write_workspace": 1,
        "fuel_override": -1,
        "memory_override_mb": "64"
    }))
    .unwrap();
    assert!(!wrong_types.read_workspace);
    assert!(!wrong_types.write_workspace);
    assert_eq!(wrong_types.fuel_override, 0);
    assert_eq!(wrong_types.memory_override_mb, 0);
}

#[tokio::test]
async fn list_action_returns_modules() {
    let dir = tempfile::tempdir().unwrap();
    let tools_dir = dir.path().join("tools/wasm");
    std::fs::create_dir_all(&tools_dir).unwrap();
    std::fs::write(tools_dir.join("alpha.wasm"), b"\0asm").unwrap();
    std::fs::write(tools_dir.join("beta.wasm"), b"\0asm").unwrap();
    std::fs::write(tools_dir.join("bad$name.wasm"), b"\0asm").unwrap();

    let tool = wasm_tool(dir.path().to_path_buf());

    let result = tool.execute(json!({"action": "list"})).await.unwrap();
    assert!(result.success);
    assert!(result.output.contains("alpha"));
    assert!(result.output.contains("beta"));
    assert!(!result.output.contains("bad$name"));
}

#[tokio::test]
async fn list_action_returns_empty_modules_when_tools_directory_is_missing() {
    let dir = tempfile::tempdir().unwrap();
    let result =
        wasm_tool(dir.path().to_path_buf()).execute(json!({"action": "list"})).await.unwrap();

    assert!(result.success);
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&result.output).unwrap(),
        json!({
            "modules": []
        })
    );
    assert!(result.error.is_none());
}

#[tokio::test]
async fn list_action_reports_tools_directory_read_error() {
    let dir = tempfile::tempdir().unwrap();
    let tools_root = dir.path().join("tools");
    std::fs::create_dir_all(&tools_root).unwrap();
    std::fs::write(tools_root.join("wasm"), b"not a directory").unwrap();

    let result =
        wasm_tool(dir.path().to_path_buf()).execute(json!({"action": "list"})).await.unwrap();

    assert!(!result.success);
    assert!(result.output.is_empty());
    assert!(result.error.unwrap_or_default().contains("Failed to read tools dir"));
}

#[tokio::test]
async fn run_action_requires_module() {
    let dir = tempfile::tempdir().unwrap();
    let tool = wasm_tool(dir.path().to_path_buf());

    let result = tool.execute(json!({"action": "run"})).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("module"));
}

#[tokio::test]
async fn run_action_rejects_non_array_allowed_hosts_before_execution() {
    let dir = tempfile::tempdir().unwrap();
    let error = wasm_tool(dir.path().to_path_buf())
        .execute(json!({
            "action": "run",
            "module": "example",
            "allowed_hosts": "api.example.com"
        }))
        .await
        .unwrap_err();

    assert_eq!(error.to_string(), "'allowed_hosts' must be an array of strings");
}

#[tokio::test]
async fn unsupported_action_returns_guidance_without_runtime_side_effects() {
    let dir = tempfile::tempdir().unwrap();
    let result =
        wasm_tool(dir.path().to_path_buf()).execute(json!({"action": "inspect"})).await.unwrap();

    assert!(!result.success);
    assert!(result.output.is_empty());
    assert_eq!(result.error.as_deref(), Some("Unsupported action 'inspect'. Use 'list' or 'run'."));
}

#[tokio::test]
async fn completed_action_consumes_budget_before_next_action() {
    let dir = tempfile::tempdir().unwrap();
    let security = Arc::new(SecurityPolicy {
        autonomy: AutonomyLevel::Full,
        workspace_dir: dir.path().to_path_buf(),
        max_actions_per_hour: 1,
        ..SecurityPolicy::default()
    });
    let runtime: Arc<dyn RuntimeAdapter> = Arc::new(WasmRuntime::new(WasmRuntimeConfig::default()));
    let tool = WasmModuleTool::new(security, runtime);

    let first = tool.execute(json!({"action": "unsupported"})).await.unwrap();
    assert_eq!(
        first.error.as_deref(),
        Some("Unsupported action 'unsupported'. Use 'list' or 'run'.")
    );

    let second = tool.execute(json!({"action": "list"})).await.unwrap();
    assert!(!second.success);
    assert_eq!(
        second.error.as_deref(),
        Some("Rate limit exceeded: too many actions in the last hour")
    );
}

#[cfg(feature = "runtime-wasm")]
#[tokio::test]
async fn run_action_maps_zero_and_nonzero_module_exit_codes() {
    let dir = tempfile::tempdir().unwrap();
    let tools_dir = dir.path().join("tools/wasm");
    std::fs::create_dir_all(&tools_dir).unwrap();
    std::fs::write(tools_dir.join("success.wasm"), constant_exit_wasm(0)).unwrap();
    std::fs::write(tools_dir.join("failure.wasm"), constant_exit_wasm(7)).unwrap();
    let tool = wasm_tool(dir.path().to_path_buf());

    let success = tool.execute(json!({"action": "run", "module": "success"})).await.unwrap();
    assert!(success.success);
    assert!(success.error.is_none());
    let success_output: serde_json::Value = serde_json::from_str(&success.output).unwrap();
    assert_eq!(success_output["module"], json!("success"));
    assert_eq!(success_output["exit_code"], json!(0));
    assert_eq!(success_output["stdout"], json!(""));
    assert_eq!(success_output["stderr"], json!(""));
    assert_eq!(success_output["module_sha256"].as_str().unwrap().len(), 64);

    let failure = tool.execute(json!({"action": "run", "module": "failure"})).await.unwrap();
    assert!(!failure.success);
    assert_eq!(failure.error.as_deref(), Some("WASM module exited with code 7"));
    let failure_output: serde_json::Value = serde_json::from_str(&failure.output).unwrap();
    assert_eq!(failure_output["module"], json!("failure"));
    assert_eq!(failure_output["exit_code"], json!(7));
    assert_eq!(failure_output["stderr"], json!(""));
}

#[cfg(feature = "runtime-wasm")]
#[tokio::test]
async fn run_action_uses_runtime_stderr_for_fuel_exhaustion() {
    let dir = tempfile::tempdir().unwrap();
    let tools_dir = dir.path().join("tools/wasm");
    std::fs::create_dir_all(&tools_dir).unwrap();
    std::fs::write(tools_dir.join("loop.wasm"), infinite_loop_wasm()).unwrap();

    let mut config = WasmRuntimeConfig::default();
    config.fuel_limit = 32;
    let runtime: Arc<dyn RuntimeAdapter> = Arc::new(WasmRuntime::new(config));
    let tool = WasmModuleTool::new(test_security(dir.path().to_path_buf()), runtime);

    let result = tool.execute(json!({"action": "run", "module": "loop"})).await.unwrap();
    assert!(!result.success);
    assert!(result.output.is_empty());
    let stderr = result.error.as_deref().unwrap();
    assert!(stderr.contains("all fuel consumed by WebAssembly"), "{stderr}");
}

#[cfg(feature = "runtime-wasm")]
#[tokio::test]
async fn run_action_reports_missing_module_error() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("tools/wasm")).unwrap();

    let result = wasm_tool(dir.path().to_path_buf())
        .execute(json!({"action": "run", "module": "missing"}))
        .await
        .unwrap();

    assert!(!result.success);
    assert!(result.output.is_empty());
    assert!(result.error.unwrap_or_default().contains("WASM module not found: missing"));
}

#[tokio::test]
async fn run_action_errors_without_runtime_wasm_feature() {
    if WasmRuntime::is_available() {
        return;
    }

    let dir = tempfile::tempdir().unwrap();
    let tools_dir = dir.path().join("tools/wasm");
    std::fs::create_dir_all(&tools_dir).unwrap();
    std::fs::write(tools_dir.join("hello.wasm"), b"\0asm\x01\0\0\0").unwrap();

    let tool = wasm_tool(dir.path().to_path_buf());

    let result = tool.execute(json!({"action": "run", "module": "hello"})).await.unwrap();
    assert!(!result.success);
    assert!(result.error.unwrap_or_default().contains("not available"));
}

#[tokio::test]
async fn tool_rejects_non_wasm_runtime() {
    let dir = tempfile::tempdir().unwrap();
    let security = test_security(dir.path().to_path_buf());
    let runtime: Arc<dyn RuntimeAdapter> = Arc::new(NativeRuntime::new());
    let tool = WasmModuleTool::new(security, runtime);

    let result = tool.execute(json!({"action": "list"})).await.unwrap();
    assert!(!result.success);
    assert!(result.error.unwrap_or_default().contains("runtime.kind = \"wasm\""));
}
