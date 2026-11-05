use super::native_tools::{NativeCallCollector, native_search_output};
use super::types::ToolCallKind;
use aisdk::core::{ToolSearchCall, ToolSearchExecution};
use serde_json::json;

fn search_call() -> ToolSearchCall {
    ToolSearchCall {
        id: Some("item-1".into()),
        call_id: Some("call-1".into()),
        status: Some("completed".into()),
        execution: ToolSearchExecution::Client,
        arguments: json!({"query":"weather"}),
        encrypted_content: None,
    }
}

#[test]
fn native_tool_search_event_conversion_preserves_search_call_id() {
    let mut collector = NativeCallCollector::default();
    let call = collector.search(search_call(), true).unwrap().unwrap();
    assert_eq!(call.id, "call-1");
    assert_eq!(call.kind, ToolCallKind::Search);
    assert_eq!(call.namespace, None);
    assert_eq!(call.name, "tool_search");
    assert!(collector.search(search_call(), true).unwrap().is_none());
}

#[test]
fn native_tool_search_event_rejects_unsupported_and_missing_identity() {
    let mut collector = NativeCallCollector::default();
    assert!(collector.search(search_call(), false).is_err());
    let mut call = search_call();
    call.call_id = None;
    assert!(collector.search(call, true).is_err());
    let mut call = search_call();
    call.execution = ToolSearchExecution::Server;
    assert!(collector.search(call, true).is_err());
}

#[tokio::test]
async fn native_tool_search_event_empty_output_is_dedicated_item() {
    let catalog = catalog(vec![]);
    let mut collector = NativeCallCollector::default();
    let call = collector.search(search_call(), true).unwrap().unwrap();
    let output = native_search_output(&call, &catalog, false).unwrap();
    let wire = output_wire(output).await;
    assert_eq!(wire["type"], "tool_search_output");
    assert_eq!(wire["call_id"], "call-1");
    assert_eq!(wire["tools"], json!([]));
}

struct SpecTool(crate::tools::ToolSpec);
#[async_trait::async_trait]
impl crate::tools::Tool for SpecTool {
    fn name(&self) -> &str {
        &self.0.id
    }
    fn description(&self) -> &str {
        &self.0.description
    }
    fn parameters_schema(&self) -> serde_json::Value {
        self.0.input_schema.clone()
    }
    fn spec(&self) -> crate::tools::ToolSpec {
        self.0.clone()
    }
    async fn execute(&self, _: serde_json::Value) -> anyhow::Result<crate::tools::ToolResult> {
        anyhow::bail!("conversion must not execute tools")
    }
}

fn catalog(mut specs: Vec<crate::tools::ToolSpec>) -> crate::tools::catalog::ToolCatalogSnapshot {
    specs.push(crate::tools::ToolSpec::new("tool_search", "search", json!({"type":"object"})));
    crate::tools::catalog::ToolCatalogSnapshot::from_runtime_tools(
        specs
            .into_iter()
            .map(|spec| {
                std::sync::Arc::new(SpecTool(spec)) as std::sync::Arc<dyn crate::tools::Tool>
            })
            .collect(),
    )
}

fn weather_spec(id: &str, namespace: Option<&str>) -> crate::tools::ToolSpec {
    let mut spec = crate::tools::ToolSpec::new(
        id,
        "weather forecast",
        json!({"type":"object", "properties":{"city":{"type":"string"}}}),
    );
    spec.exposure = vw_api_types::tools::ToolExposure::Deferred;
    spec.identity.name = "forecast".into();
    spec.identity.namespace = namespace.map(str::to_owned);
    spec
}

#[tokio::test]
async fn native_tool_search_event_results_only_use_authorized_deferred_catalog() {
    use aisdk::core::Message;
    let allowed = weather_spec("stable.weather", Some("climate"));
    let mut hidden = weather_spec("hidden", None);
    hidden.exposure = vw_api_types::tools::ToolExposure::Hidden;
    let mut direct = weather_spec("direct", None);
    direct.exposure = vw_api_types::tools::ToolExposure::Direct;
    let mut code = weather_spec("code", None);
    code.exposure = vw_api_types::tools::ToolExposure::CodeModeOnly;
    let snapshot = catalog(vec![allowed, hidden, direct, code]);
    let call = NativeCallCollector::default().search(search_call(), true).unwrap().unwrap();
    let output = native_search_output(&call, &snapshot, true).unwrap();
    let wire = output_wire(output.clone()).await;
    assert_eq!(wire["tools"].as_array().unwrap().len(), 1);
    assert_eq!(wire["tools"][0]["name"], "climate");
    assert_eq!(wire["tools"][0]["tools"][0]["name"], "forecast");
    assert!(matches!(output, Message::ToolSearchOutput(_)));
    // Conversion cannot expand the runtime catalog or load any tool.
    assert_eq!(snapshot.specs().len(), 5);
    assert!(!snapshot.authorizes("model.injected"));
}

#[test]
fn native_tool_search_event_rejects_untrusted_arguments_and_catalog() {
    for arguments in [
        json!({"query":"weather", "tools":[{"name":"injected"}]}),
        json!({"query":""}),
        json!({"query":"weather", "limit":0}),
        json!({"query":"weather", "limit":33}),
    ] {
        let mut call = search_call();
        call.arguments = arguments;
        assert!(NativeCallCollector::default().search(call, true).is_err());
    }
    let mut raw = search_call();
    raw.status = Some("in_progress".into());
    assert!(NativeCallCollector::default().search(raw, true).is_err());
    let mut call = NativeCallCollector::default().search(search_call(), true).unwrap().unwrap();
    assert!(
        native_search_output(&call, &crate::tools::catalog::ToolCatalogSnapshot::default(), false)
            .is_err()
    );
    call.kind = ToolCallKind::Function;
    assert!(native_search_output(&call, &catalog(vec![]), false).is_err());
}

#[test]
fn native_tool_search_event_conflicting_duplicates_fail_closed() {
    let mut collector = NativeCallCollector::default();
    collector.search(search_call(), true).unwrap();
    let mut call = search_call();
    call.arguments = json!({"query":"other"});
    assert!(collector.search(call, true).is_err());
    let mut function = super::types::ToolCall {
        kind: ToolCallKind::Function,
        namespace: Some("climate".into()),
        id: "function-1".into(),
        name: "stable.weather".into(),
        arguments: "{}".into(),
    };
    assert!(collector.record(function.clone()).unwrap().is_some());
    assert!(collector.record(function.clone()).unwrap().is_none());
    function.namespace = Some("other".into());
    assert!(collector.record(function).is_err());
}

/// Exercise the SDK serializer through its public API and a bounded loopback server.
async fn output_wire(output: aisdk::core::Message) -> serde_json::Value {
    use aisdk::core::language_model::LanguageModel;
    use aisdk::core::{DynamicModel, LanguageModelRequest};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        tokio::time::timeout(std::time::Duration::from_secs(15), async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut bytes = Vec::new();
            loop {
                let mut buffer = [0; 4096];
                let count = socket.read(&mut buffer).await.unwrap();
                assert!(count > 0);
                bytes.extend_from_slice(&buffer[..count]);
                let Some(end) = bytes.windows(4).position(|part| part == b"\r\n\r\n") else { continue; };
                let headers = std::str::from_utf8(&bytes[..end]).unwrap();
                let length: usize = headers.lines().find_map(|line| {
                    let (key, value) = line.split_once(':')?;
                    key.eq_ignore_ascii_case("content-length").then(|| value.trim().parse().unwrap())
                }).unwrap();
                if bytes.len() < end + 4 + length { continue; }
                let body: serde_json::Value = serde_json::from_slice(&bytes[end + 4..end + 4 + length]).unwrap();
                let response = r#"{"id":"response-fixture","output":[]}"#;
                socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", response.len(), response).as_bytes()).await.unwrap();
                return body;
            }
        }).await.unwrap()
    });
    let model = aisdk::providers::OpenAI::<DynamicModel>::builder()
        .api_key("fixture")
        .model_name("fixture")
        .base_url(format!("http://{address}/v1"))
        .build()
        .unwrap();
    let request = LanguageModelRequest::builder().model(model).messages(vec![output]).build();
    let mut options = std::ops::Deref::deref(&request).clone();
    options.max_retries = Some(0);
    let mut model = request.model;
    let result =
        tokio::time::timeout(std::time::Duration::from_secs(15), model.generate_text(options))
            .await;
    let body = server.await.unwrap();
    result.unwrap().unwrap();
    body["input"][0].clone()
}
