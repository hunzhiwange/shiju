use super::hook_mcp;
use serde_json::{Value, json};
use std::{fs, sync::Arc};
struct Fixture {
    temp: tempfile::TempDir,
    child: std::process::Child,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
impl Fixture {
    async fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let child = std::process::Command::new("python3")
            .args([
                format!("{}/testdata/plugin_compat/t45/server.py", env!("CARGO_MANIFEST_DIR")),
                temp.path().join("port").display().to_string(),
                temp.path().join("calls").display().to_string(),
            ])
            .spawn()
            .unwrap();
        let fixture = Self { temp, child };
        for _ in 0..200 {
            if fixture.temp.path().join("port").exists() {
                return fixture;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        panic!("fixture HTTP server failed to start")
    }
    fn plugin(&self, name: &str) -> std::path::PathBuf {
        let root = self.temp.path().join(".vibewindow/plugins").join(name);
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("plugin.json"),json!({"$schema":"https://agent-plugins.org/schemas/1.0.0/plugin.schema.json","name":name}).to_string()).unwrap();
        let port = fs::read_to_string(self.temp.path().join("port")).unwrap();
        fs::write(root.join("mcp.json"),json!({"$schema":"https://agent-plugins.org/schemas/1.0.0/mcp.schema.json","mcpServers":{"fixture":{"type":"streamable-http","url":format!("http://127.0.0.1:{port}/mcp")}}}).to_string()).unwrap();
        root
    }
    fn handler(&self) -> crate::plugins::CodexHookHandler {
        serde_json::from_value(json!({"type":"mcp_tool","server":"fixture","tool":"echo","input":{"message":"${tool_input.message}"}})).unwrap()
    }
    async fn call(&self, root: &std::path::Path, event: Value) -> anyhow::Result<Value> {
        self.call_policy(root, event, false).await
    }
    async fn call_policy(
        &self,
        root: &std::path::Path,
        event: Value,
        deny: bool,
    ) -> anyhow::Result<Value> {
        use crate::plugins::{
            CodexHooksManifest, PluginHookTrustStore, hook_handler::codex_handlers,
        };
        let manifest: CodexHooksManifest =
            serde_json::from_value(json!({"hooks":{"PreToolUse":[{"hooks":[self.handler()]}]}}))?;
        let state = tempfile::tempdir()?;
        let mut trust = PluginHookTrustStore::load_or_default(&state.path().join("trust.json"))?;
        trust.trust_codex_hooks("fixture", root, &manifest)?;
        let handlers = codex_handlers("fixture", root, &manifest, &trust)?;
        let context = Arc::new(
            crate::tools::ToolUseContext::new("t45", Some(self.temp.path().display().to_string()))
                .with_turn_id("turn")
                .with_tool_call_id("call")
                .with_model("fixture"),
        );
        let context = if deny {
            Arc::new(context.as_ref().clone().with_tool_exposure_snapshot(Vec::new()))
        } else {
            context
        };
        crate::tools::scope_tool_use_context(context.clone(), async {
            match handlers[0].before_tool_call("read".into(), event).await {
                crate::hooks::HookResult::Cancel(reason) => anyhow::bail!(reason),
                crate::hooks::HookResult::Continue(_) => {}
            }
            let contexts = context.take_hook_contexts();
            Ok(json!({"hookSpecificOutput":{"additionalContext": contexts.first()}}))
        })
        .await
    }
}
#[tokio::test]
async fn codex_plugin_compat_t45_calls_only_authorized_hook_server() {
    let fixture = Fixture::new().await;
    let root = fixture.plugin("fixture");
    let original = fs::read(root.join("mcp.json")).unwrap();
    let result = fixture.call(&root, json!({"message":"hello"})).await.unwrap();
    assert_eq!(result["hookSpecificOutput"]["additionalContext"], "fixture-context");
    let calls = fs::read_to_string(fixture.temp.path().join("calls")).unwrap();
    let call: Value = serde_json::from_str(calls.trim()).unwrap();
    assert_eq!(call["arguments"]["message"], "hello");
    assert_eq!(original, fs::read(root.join("mcp.json")).unwrap());
}
#[tokio::test]
async fn codex_plugin_compat_t45_rejects_invalid_input() {
    let fixture = Fixture::new().await;
    let root = fixture.plugin("fixture");
    assert!(fixture.call(&root, json!({})).await.is_err());
    assert!(!fixture.temp.path().join("calls").exists());
    assert!(hook_mcp::expand(&json!({"x":"${absent}"}), &json!({})).is_err());
    assert_eq!(
        hook_mcp::expand(&json!({"x":"${nested.count}"}), &json!({"nested":{"count":3}})).unwrap(),
        json!({"x":3})
    );
}
#[tokio::test]
async fn codex_plugin_compat_t45_preserves_isolation() {
    let fixture = Fixture::new().await;
    fixture.plugin("fixture");
    let other = fixture.temp.path().join("other");
    fs::create_dir(&other).unwrap();
    assert!(fixture.call(&other, json!({"message":"denied"})).await.is_err());
    assert!(!fixture.temp.path().join("calls").exists());
}

#[test]
fn codex_plugin_compat_t45_admits_trusted_mcp_and_skips_session_end() {
    use crate::plugins::{CodexHooksManifest, PluginHookTrustStore, hook_handler::codex_handlers};
    let root = tempfile::tempdir().unwrap();
    let state = tempfile::tempdir().unwrap();
    let manifest:CodexHooksManifest=serde_json::from_value(json!({"hooks":{"PreToolUse":[{"hooks":[{"type":"mcp_tool","server":"fixture","tool":"echo"}]}],"SessionEnd":[{"hooks":[{"type":"mcp_tool","server":"fixture","tool":"echo"}]}]}})).unwrap();
    let mut trust =
        PluginHookTrustStore::load_or_default(&state.path().join("trust.json")).unwrap();
    assert!(codex_handlers("fixture", root.path(), &manifest, &trust).unwrap().is_empty());
    trust.trust_codex_hooks("fixture", root.path(), &manifest).unwrap();
    assert_eq!(codex_handlers("fixture", root.path(), &manifest, &trust).unwrap().len(), 1);
}

#[tokio::test]
async fn codex_plugin_compat_t45_respects_tool_authorization() {
    let fixture = Fixture::new().await;
    let root = fixture.plugin("fixture");
    let error = fixture.call_policy(&root, json!({"message":"denied"}), true).await.unwrap_err();
    assert!(error.to_string().contains("not exposed"), "{error}");
    assert!(!fixture.temp.path().join("calls").exists());
}
