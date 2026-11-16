//! Request-level discovery regressions using scripted models and real local files.

use crate::agent::{Agent, AgentHost};
use tool_context::{host::ContextHost, context::{ToolUseContext, scope_tool_use_context}};
use tool_types::Tool;
use model_types::{ChatMessage, Provider};
use std::sync::Arc;
use tool_assembly::catalog::{ToolCatalog, ToolCatalogSnapshot};
use tool_assembly::tool_search::ToolDiscoverySession;
use anyhow::Result;
use tokio_util::sync::CancellationToken;
use model_types::ProviderCapabilities;
use async_trait::async_trait;
use std::collections::VecDeque;
use std::sync::Mutex;

const DISCOVERY_LEDGER: &str = "mcp__fixture__ledger";
const DISCOVERY_QUEUE: &str = "mcp__fixture__queue";

struct DiscoveryFileTool {
    id: &'static str,
    path: std::path::PathBuf,
}

#[async_trait]
impl Tool for DiscoveryFileTool {
    fn spec(&self) -> tool_types::ToolSpec {
        let mut spec =
            tool_types::ToolSpec::new(self.name(), self.description(), self.parameters_schema())
                .with_read_only(self.is_read_only())
                .with_concurrency_safe(self.is_concurrency_safe());
        spec.exposure = vw_api_tools::ToolExposure::Deferred;
        spec.source = vw_api_tools::ToolSource::Mcp;
        spec.identity = vw_api_tools::ToolIdentity::new(
            Some("fixture".into()),
            self.name().rsplit("__").next().unwrap(),
        )
        .unwrap();
        spec
    }

    fn name(&self) -> &str {
        self.id
    }

    fn description(&self) -> &str {
        self.id
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({"type":"object", "properties": {self.id: {"type":"string"}}})
    }

    fn is_read_only(&self) -> bool {
        true
    }

    async fn check_permissions(&self, input: &serde_json::Value) -> anyhow::Result<()> {
        if input["deny"].as_bool().unwrap_or(false) {
            anyhow::bail!("ledger access denied");
        }
        Ok(())
    }

    async fn execute(&self, _: serde_json::Value) -> anyhow::Result<tool_types::ToolResult> {
        Ok(tool_types::ToolResult {
            success: true,
            output: std::fs::read_to_string(&self.path)?,
            error: None,
        })
    }
}

fn discovery_tools(workspace: &std::path::Path) -> Vec<Box<dyn Tool>> {
    use tool_assembly::discovery::ToolDiscoveryCatalog;
    use tool_assembly::tool_search::ToolSearchTool;
    let mut tools: Vec<Box<dyn Tool>> = vec![
        Box::new(DiscoveryFileTool { id: DISCOVERY_LEDGER, path: workspace.join("ledger") }),
        Box::new(DiscoveryFileTool { id: DISCOVERY_QUEUE, path: workspace.join("queue") }),
    ];
    let catalog = ToolDiscoveryCatalog::new(tools.iter().map(|tool| tool.spec()).collect());
    tools.push(Box::new(ToolSearchTool::new(&catalog)));
    tools
}

type DiscoveryRequest = (Vec<ChatMessage>, Vec<String>);

#[derive(Clone)]
struct DiscoveryProvider {
    responses: Arc<Mutex<VecDeque<model_types::ChatResponse>>>,
    requests: Arc<Mutex<Vec<DiscoveryRequest>>>,
    native: bool,
    search: bool,
    on_request: Option<Arc<dyn Fn(usize) + Send + Sync>>,
}

impl DiscoveryProvider {
    fn new(native: bool, calls: Vec<Vec<(&str, &str, serde_json::Value)>>) -> Self {
        let mut responses = VecDeque::new();
        for calls in calls {
            let calls: Vec<_> = calls
                .into_iter()
                .map(|(id, name, input)| model_types::ToolCall {
                    kind: Default::default(),
                    namespace: None,
                    id: id.to_string(),
                    name: name.to_string(),
                    arguments: input.to_string(),
                })
                .collect();
            let text = (!native).then(|| {
                calls
                    .iter()
                    .map(|call| {
                        format!(
                            "<tool_call>{{\"name\":\"{}\",\"arguments\":{}}}</tool_call>",
                            call.name, call.arguments,
                        )
                    })
                    .collect::<String>()
            });
            responses.push_back(model_types::ChatResponse {
                text,
                tool_calls: if native { calls } else { Vec::new() },
                usage: None,
                reasoning_content: None,
            });
        }
        responses.push_back(model_types::ChatResponse {
            text: Some("final answer".to_string()),
            tool_calls: Vec::new(),
            usage: None,
            reasoning_content: None,
        });
        Self {
            responses: Arc::new(Mutex::new(responses)),
            requests: Arc::default(),
            native,
            search: false,
            on_request: None,
        }
    }

    fn requests(&self) -> Vec<DiscoveryRequest> {
        self.requests.lock().unwrap().clone()
    }
}

#[async_trait]
impl Provider for DiscoveryProvider {
    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            native_tool_search: self.search,
            native_tool_calling: self.native,
            vision: false,
        }
    }

    async fn chat_with_system(
        &self,
        _: Option<&str>,
        _: &str,
        _: &str,
        _: f64,
    ) -> anyhow::Result<String> {
        anyhow::bail!("structured request required")
    }

    async fn chat(
        &self,
        request: model_types::ChatRequest<'_>,
        _: &str,
        _: f64,
    ) -> anyhow::Result<model_types::ChatResponse> {
        self.requests.lock().unwrap().push((
            request.messages.to_vec(),
            request.tools.unwrap_or_default().iter().map(|spec| spec.id.clone()).collect(),
        ));
        if let Some(on_request) = &self.on_request {
            on_request(self.requests.lock().unwrap().len());
        }
        self.responses
            .lock()
            .unwrap()
            .pop_front()
            .ok_or_else(|| anyhow::anyhow!("script exhausted"))
    }
}

async fn run_discovery_loop<H>(
    provider: &DiscoveryProvider,
    tools: Vec<Box<dyn Tool>>,
    excluded: &[String],
) -> Vec<ChatMessage>
where
    H: AgentHost,
    H::Context: tool_services::BatchHost + ContextHost<ToolCatalog = ToolCatalog, ToolCatalogSnapshot = ToolCatalogSnapshot, ToolDiscoverySession = ToolDiscoverySession>,
{
    let mut history = vec![ChatMessage::user("inspect ledger")];
    assert_eq!(
        run_tool_call_loop::<H>(
            provider,
            &mut history,
            &tools,
            &agent_observability::NoopObserver,
            "fixture",
            "fixture/model",
            0.0,
            true,
            None,
            "cli",
            &agent_config::schema::MultimodalConfig::default(),
            8,
            None,
            None,
            None,
            None,
            excluded,
        )
        .await
        .unwrap(),
        "final answer"
    );
    history
}

struct PendingCatalogTool {
    started: Arc<tokio::sync::Notify>,
    release: Arc<tokio::sync::Notify>,
}

#[async_trait]
impl Tool for PendingCatalogTool {
    fn name(&self) -> &str {
        "old_tool"
    }
    fn description(&self) -> &str {
        "pending old executor"
    }
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({"type":"object"})
    }
    async fn execute(&self, _: serde_json::Value) -> anyhow::Result<tool_types::ToolResult> {
        self.started.notify_one();
        self.release.notified().await;
        Ok(tool_types::ToolResult {
            success: true,
            output: "old executor completed".into(),
            error: None,
        })
    }
}

fn native_search_provider(calls: Vec<Vec<(&str, &str, serde_json::Value)>>) -> DiscoveryProvider {
    let mut provider = DiscoveryProvider::new(true, calls);
    provider.search = true;
    for response in provider.responses.lock().unwrap().iter_mut() {
        for call in &mut response.tool_calls {
            if call.name == "tool_search" {
                call.kind = model_types::ToolCallKind::Search;
            }
        }
    }
    provider
}

#[allow(clippy::too_many_arguments)]
async fn run_tool_call_loop<H: AgentHost>(
    provider: &dyn Provider, history: &mut Vec<ChatMessage>, tools: &[Box<dyn Tool>],
    observer: &dyn agent_observability::Observer, provider_name: &str, model: &str,
    temperature: f64, silent: bool, approval: Option<Arc<agent_approval::ApprovalManager>>,
    channel: &str, config: &agent_config::schema::MultimodalConfig, max_iterations: usize,
    cancellation: Option<CancellationToken>, delta: Option<tokio::sync::mpsc::Sender<String>>,
    hooks: Option<Arc<agent_hooks::HookRunner>>, security: Option<Arc<agent_security::SecurityPolicy>>,
    excluded: &[String],
) -> Result<String> {
    H::run_tool_call_loop_with_skills(provider, history, tools, observer, provider_name, model,
        temperature, silent, approval, channel, config, max_iterations, cancellation, delta,
        hooks, security, excluded, None).await
}

mod scenarios_1;
pub use scenarios_1::*;

mod scenarios_2;
pub use scenarios_2::*;

mod scenarios_3;
pub use scenarios_3::*;

mod scenarios_4;
pub use scenarios_4::*;
