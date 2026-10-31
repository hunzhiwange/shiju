//! Observe actual SDK debug events in an isolated test process, without new dependencies.
use super::*;
use std::sync::Mutex;
use tracing::{
    Metadata, Subscriber,
    field::{Field, Visit},
    span::{Attributes, Id, Record},
};

#[derive(Clone, Default)]
struct Capture {
    records: Arc<Mutex<String>>,
}
impl Subscriber for Capture {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        metadata.target().starts_with("rmcp")
    }
    fn new_span(&self, _span: &Attributes<'_>) -> Id {
        Id::from_u64(1)
    }
    fn record(&self, _span: &Id, _values: &Record<'_>) {}
    fn record_follows_from(&self, _span: &Id, _follows: &Id) {}
    fn event(&self, event: &tracing::Event<'_>) {
        struct Fields<'a>(&'a mut String);
        impl Visit for Fields<'_> {
            fn record_debug(&mut self, _field: &Field, value: &dyn std::fmt::Debug) {
                if self.0.len() < 262_144 {
                    self.0.push_str(&format!("{value:?}\n"));
                }
            }
        }
        event.record(&mut Fields(&mut self.records.lock().unwrap()));
    }
    fn enter(&self, _span: &Id) {}
    fn exit(&self, _span: &Id) {}
}

#[tokio::test]
async fn codex_plugin_compat_t24_keeps_user_answers_out_of_sdk_debug_traces() {
    if std::env::var("VW_T24_TRACE_CHILD").as_deref() != Ok("1") {
        let name = format!(
            "{}::codex_plugin_compat_t24_keeps_user_answers_out_of_sdk_debug_traces",
            module_path!().split_once("::").unwrap().1
        );
        let mut command = tokio::process::Command::new(std::env::current_exe().unwrap());
        command
            .args(["--exact", &name, "--nocapture"])
            .env("VW_T24_TRACE_CHILD", "1")
            .kill_on_drop(true);
        let output =
            tokio::time::timeout(Duration::from_secs(15), command.output()).await.unwrap().unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            output.status.success(),
            "SDK privacy fixture failed: {stdout}\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            stdout.contains("1 passed; 0 failed"),
            "isolated trace fixture must execute one test"
        );
        return;
    }
    let root = tempfile::tempdir().unwrap();
    let host = McpFormSession::register("t24-private", root.path()).unwrap();
    let mut events = host.subscribe();
    let server = server(root.path(), "forms");
    let context =
        Arc::new(ToolUseContext::new("t24-private", Some(root.path().display().to_string())));
    let capture = Capture::default();
    tracing::subscriber::set_global_default(capture.clone()).unwrap();
    scope_tool_use_context(context, async {
        let closed = tokio_util::sync::CancellationToken::new();
        let binding = crate::plugins::compat::mcp_elicitation::current_host()
            .unwrap()
            .bind("forms", closed.clone());
        let _operation = binding.activate();
        let client = crate::tools::mcp_common::open_server_peer(
            server,
            Duration::from_secs(5),
            closed,
            Some(binding),
        )
        .await
        .unwrap();
        let result = {
            let operation = async {
                Ok(client
                    .peer()
                    .call_tool(CallToolRequestParam {
                        name: "form".into(),
                        arguments: Some(
                            json!({"suppress_reply_content":true}).as_object().unwrap().clone(),
                        ),
                    })
                    .await?
                    .structured_content
                    .unwrap())
            };
            tokio::pin!(operation);
            let req = request(&mut events, &mut operation.as_mut()).await;
            let mut response = answer();
            response.content.as_mut().unwrap()["name"] = json!("user-private-value");
            host.respond(&req, response).unwrap();
            operation.await.unwrap()
        };
        assert_eq!(result["form_reply"]["result"]["received_private_value"], true);
        client.cancel().await.unwrap();
    })
    .await;
    let records = capture.records.lock().unwrap();
    assert!(
        records.contains("response message"),
        "the fixture must observe actual SDK response events"
    );
    assert!(
        !records.contains("user-private-value"),
        "target: SDK traces must not contain private user answers"
    );
}
