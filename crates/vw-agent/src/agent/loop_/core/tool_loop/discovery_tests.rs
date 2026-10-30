//! Request-level discovery regressions using scripted models and real local files.

use super::*;
use crate::providers::traits::ProviderCapabilities;
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

    async fn execute(&self, _: serde_json::Value) -> anyhow::Result<crate::tools::ToolResult> {
        Ok(crate::tools::ToolResult {
            success: true,
            output: std::fs::read_to_string(&self.path)?,
            error: None,
        })
    }
}

fn discovery_tools(workspace: &std::path::Path) -> Vec<Box<dyn Tool>> {
    use crate::tools::discovery::ToolDiscoveryCatalog;
    use crate::tools::tool_search::ToolSearchTool;
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
    responses: Arc<Mutex<VecDeque<crate::providers::ChatResponse>>>,
    requests: Arc<Mutex<Vec<DiscoveryRequest>>>,
    native: bool,
}

impl DiscoveryProvider {
    fn new(native: bool, calls: Vec<Vec<(&str, &str, serde_json::Value)>>) -> Self {
        let mut responses = VecDeque::new();
        for calls in calls {
            let calls: Vec<_> = calls
                .into_iter()
                .map(|(id, name, input)| crate::providers::ToolCall {
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
            responses.push_back(crate::providers::ChatResponse {
                text,
                tool_calls: if native { calls } else { Vec::new() },
                usage: None,
                reasoning_content: None,
            });
        }
        responses.push_back(crate::providers::ChatResponse {
            text: Some("final answer".to_string()),
            tool_calls: Vec::new(),
            usage: None,
            reasoning_content: None,
        });
        Self { responses: Arc::new(Mutex::new(responses)), requests: Arc::default(), native }
    }

    fn requests(&self) -> Vec<DiscoveryRequest> {
        self.requests.lock().unwrap().clone()
    }
}

#[async_trait]
impl Provider for DiscoveryProvider {
    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities { native_tool_calling: self.native, vision: false }
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
        request: crate::providers::ChatRequest<'_>,
        _: &str,
        _: f64,
    ) -> anyhow::Result<crate::providers::ChatResponse> {
        self.requests.lock().unwrap().push((
            request.messages.to_vec(),
            request.tools.unwrap_or_default().iter().map(|spec| spec.id.clone()).collect(),
        ));
        self.responses
            .lock()
            .unwrap()
            .pop_front()
            .ok_or_else(|| anyhow::anyhow!("script exhausted"))
    }
}

#[tokio::test]
async fn codex_discovery_core_agent_discovers_and_calls_only_on_next_request() {
    let workspace = tempfile::tempdir().unwrap();
    std::fs::write(workspace.path().join("ledger"), "ledger-file-content").unwrap();
    let provider = DiscoveryProvider::new(
        true,
        vec![
            vec![("search", "tool_search", serde_json::json!({"query":"ledger"}))],
            vec![("read", DISCOVERY_LEDGER, serde_json::json!({}))],
        ],
    );
    let mut agent = crate::agent::Agent::builder()
        .provider(Box::new(provider.clone()))
        .tools(discovery_tools(workspace.path()))
        .memory(Arc::new(crate::memory::NoneMemory))
        .observer(Arc::new(crate::observability::NoopObserver))
        .workspace_dir(workspace.path().to_path_buf())
        .model_name("fixture/model".into())
        .build()
        .unwrap();
    assert_eq!(agent.turn("inspect ledger").await.unwrap(), "final answer");
    let requests = provider.requests();
    assert_eq!(requests[0].1, ["tool_search"]);
    assert!(
        !requests[0].0.iter().any(|message| message.content.contains("\"mcp__fixture__ledger\"")),
        "initial prompt must not leak deferred parameter schemas"
    );
    assert_eq!(requests[1].1, [DISCOVERY_LEDGER, "tool_search"]);
    let results: Vec<_> = requests[2]
        .0
        .iter()
        .filter(|message| message.role == "tool")
        .map(|message| serde_json::from_str::<serde_json::Value>(&message.content).unwrap())
        .collect();
    assert_eq!(results[0]["tool_call_id"], "search");
    assert_eq!(results[1]["tool_call_id"], "read");
    assert!(results[1]["content"].as_str().unwrap().contains("ledger-file-content"));
}

async fn run_discovery_loop(
    provider: &DiscoveryProvider,
    tools: Vec<Box<dyn Tool>>,
    excluded: &[String],
) -> Vec<ChatMessage> {
    let mut history = vec![ChatMessage::user("inspect ledger")];
    assert_eq!(
        run_tool_call_loop(
            provider,
            &mut history,
            &tools,
            &crate::observability::NoopObserver,
            "fixture",
            "fixture/model",
            0.0,
            true,
            None,
            "cli",
            &crate::config::MultimodalConfig::default(),
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

#[tokio::test]
async fn codex_discovery_core_text_protocol_exposes_only_current_specs_without_persisting_them() {
    let workspace = tempfile::tempdir().unwrap();
    std::fs::write(workspace.path().join("ledger"), "text-ledger-content").unwrap();
    let provider = DiscoveryProvider::new(
        false,
        vec![
            vec![("search", "tool_search", serde_json::json!({"query":"ledger"}))],
            vec![("read", DISCOVERY_LEDGER, serde_json::json!({}))],
        ],
    );
    let history = run_discovery_loop(&provider, discovery_tools(workspace.path()), &[]).await;
    let requests = provider.requests();
    assert!(requests.iter().all(|(_, tools)| tools.is_empty()));
    let prompt = |index: usize| {
        requests[index]
            .0
            .iter()
            .filter(|message| message.content.contains("## Tool Use Protocol"))
            .map(|message| message.content.as_str())
            .collect::<String>()
    };
    assert!(prompt(0).contains("**tool_search**"));
    assert!(!prompt(0).contains(DISCOVERY_LEDGER));
    assert!(prompt(1).contains(&format!("**{DISCOVERY_LEDGER}**")));
    assert!(!prompt(1).contains(DISCOVERY_QUEUE));
    assert!(history.iter().any(|message| message.content.contains("text-ledger-content")));
    assert!(history.iter().all(|message| !message.content.contains("## Tool Use Protocol")));
}

#[tokio::test]
async fn codex_discovery_core_same_round_search_does_not_allow_deferred_call() {
    let workspace = tempfile::tempdir().unwrap();
    std::fs::write(workspace.path().join("ledger"), "must-not-read-ledger").unwrap();
    let provider = DiscoveryProvider::new(
        true,
        vec![vec![
            ("search", "tool_search", serde_json::json!({"query":"ledger"})),
            ("read", DISCOVERY_LEDGER, serde_json::json!({})),
        ]],
    );
    let history = run_discovery_loop(&provider, discovery_tools(workspace.path()), &[]).await;
    let result = history
        .iter()
        .find(|message| {
            message.role == "tool" && message.content.contains("\"tool_call_id\":\"read\"")
        })
        .unwrap();
    assert!(result.content.contains("not exposed in this model request"));
    assert!(!result.content.contains("must-not-read-ledger"));
}

#[tokio::test]
async fn codex_discovery_core_failed_and_repeated_searches_keep_visibility_consistent() {
    let workspace = tempfile::tempdir().unwrap();
    let provider = DiscoveryProvider::new(
        true,
        vec![
            vec![("bad", "tool_search", serde_json::json!({"query":""}))],
            vec![("first", "tool_search", serde_json::json!({"query":"ledger"}))],
            vec![("second", "tool_search", serde_json::json!({"query":"ledger"}))],
        ],
    );
    run_discovery_loop(&provider, discovery_tools(workspace.path()), &[]).await;
    let requests = provider.requests();
    assert_eq!(requests[0].1, ["tool_search"]);
    assert_eq!(requests[1].1, ["tool_search"]);
    assert_eq!(requests[2].1, [DISCOVERY_LEDGER, "tool_search"]);
    assert_eq!(requests[3].1, requests[2].1);
}

#[tokio::test]
async fn codex_discovery_core_excluded_tool_cannot_be_searched_or_called() {
    let workspace = tempfile::tempdir().unwrap();
    std::fs::write(workspace.path().join("ledger"), "excluded-ledger-content").unwrap();
    let provider = DiscoveryProvider::new(
        true,
        vec![
            vec![("search", "tool_search", serde_json::json!({"query":"ledger"}))],
            vec![("read", DISCOVERY_LEDGER, serde_json::json!({}))],
        ],
    );
    let history = run_discovery_loop(
        &provider,
        discovery_tools(workspace.path()),
        &[DISCOVERY_LEDGER.into()],
    )
    .await;
    assert!(
        provider.requests().iter().all(|(_, tools)| !tools.contains(&DISCOVERY_LEDGER.to_string()))
    );
    let results: Vec<_> = history.iter().filter(|message| message.role == "tool").collect();
    assert!(results[0].content.contains("\\\"tools\\\":[]"));
    assert!(results[1].content.contains("not available in this channel"));
}

#[tokio::test]
async fn codex_discovery_core_denied_search_keeps_authorized_mcp_directly_visible() {
    let workspace = tempfile::tempdir().unwrap();
    let provider = DiscoveryProvider::new(true, Vec::new());
    run_discovery_loop(&provider, discovery_tools(workspace.path()), &["tool_search".into()]).await;
    assert_eq!(provider.requests()[0].1, [DISCOVERY_LEDGER, DISCOVERY_QUEUE]);
}

#[tokio::test]
async fn codex_discovery_core_batch_rejects_same_round_and_excluded_inner_calls() {
    let workspace = tempfile::tempdir().unwrap();
    std::fs::write(workspace.path().join("ledger"), "batch-ledger-content").unwrap();
    for excluded in [Vec::new(), vec![DISCOVERY_LEDGER.to_string()]] {
        let mut tools = discovery_tools(workspace.path());
        let inner: Vec<Arc<dyn Tool>> = vec![Arc::new(DiscoveryFileTool {
            id: DISCOVERY_LEDGER,
            path: workspace.path().join("ledger"),
        })];
        tools.push(Box::new(crate::tools::BatchTool::new(Arc::new(inner))));
        let provider = DiscoveryProvider::new(
            true,
            vec![vec![
                ("search", "tool_search", serde_json::json!({"query":"ledger"})),
                (
                    "batch",
                    "batch",
                    serde_json::json!({"tool_calls":[{"tool":DISCOVERY_LEDGER,"args":{}}]}),
                ),
            ]],
        );
        let history = run_discovery_loop(&provider, tools, &excluded).await;
        let result = history
            .iter()
            .find(|message| {
                message.role == "tool" && message.content.contains("\"tool_call_id\":\"batch\"")
            })
            .unwrap();
        assert!(result.content.contains("not exposed in this model request"));
        assert!(!result.content.contains("batch-ledger-content"));
    }
}

#[tokio::test]
async fn codex_discovery_core_empty_registry_provides_no_tool_protocol() {
    let provider = DiscoveryProvider::new(false, Vec::new());
    run_discovery_loop(&provider, Vec::new(), &[]).await;
    let requests = provider.requests();
    assert!(requests[0].1.is_empty());
    assert!(requests[0].0.iter().all(|message| !message.content.contains("## Tool Use Protocol")));
}

#[tokio::test]
async fn codex_discovery_core_loaded_batch_calls_preserve_inner_permission_checks() {
    let workspace = tempfile::tempdir().unwrap();
    std::fs::write(workspace.path().join("ledger"), "permitted-batch-content").unwrap();
    let mut tools = discovery_tools(workspace.path());
    let inner: Vec<Arc<dyn Tool>> = vec![Arc::new(DiscoveryFileTool {
        id: DISCOVERY_LEDGER,
        path: workspace.path().join("ledger"),
    })];
    tools.push(Box::new(crate::tools::BatchTool::new(Arc::new(inner))));
    let provider = DiscoveryProvider::new(
        true,
        vec![
            vec![("search", "tool_search", serde_json::json!({"query":"ledger"}))],
            vec![(
                "permitted",
                "batch",
                serde_json::json!({"tool_calls":[{"tool":DISCOVERY_LEDGER,"args":{}}]}),
            )],
            vec![(
                "denied",
                "batch",
                serde_json::json!({"tool_calls":[{"tool":DISCOVERY_LEDGER,"args":{"deny":true}}]}),
            )],
        ],
    );
    let history = run_discovery_loop(&provider, tools, &[]).await;
    let results: Vec<_> = history
        .iter()
        .filter(|message| message.role == "tool")
        .map(|message| serde_json::from_str::<serde_json::Value>(&message.content).unwrap())
        .collect();
    assert!(results[1]["content"].as_str().unwrap().contains("permitted-batch-content"));
    assert!(results[2]["content"].as_str().unwrap().contains("ledger access denied"));
    assert!(!results[2]["content"].as_str().unwrap().contains("permitted-batch-content"));
}
