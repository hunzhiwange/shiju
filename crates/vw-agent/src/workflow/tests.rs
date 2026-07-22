use super::model::parse_workflow_yaml;
use super::runner::{
    WorkflowAgentProvider, WorkflowAgentRequest, WorkflowAgentResult, WorkflowKnowledgeChunk,
    WorkflowKnowledgeProvider, WorkflowKnowledgeRequest, WorkflowPauseState, WorkflowPauseStore,
    WorkflowRuntime, WorkflowToolProvider, WorkflowToolRequest, WorkflowToolResult,
    resume_workflow, run_workflow,
};
use super::template::render_template;
use super::variables::VariablePool;
use crate::providers::{ChatRequest, ChatResponse, Provider};
use anyhow::{Result, anyhow};
use axum::{Json, Router, extract::Query, http::HeaderMap, routing::get};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use vw_api_types::workflow::{WorkflowResumeRequest, WorkflowRunRequest, WorkflowRunStatus};

#[test]
fn parse_workflow_yaml_reads_dify_graph() {
    let graph = parse_workflow_yaml(
        r#"
workflow:
  graph:
    nodes:
      - id: start
        data:
          title: 开始
          type: start
      - id: answer
        data:
          title: 回复
          type: answer
    edges:
      - source: start
        target: answer
        sourceHandle: source
"#,
    )
    .expect("graph");

    assert_eq!(graph.start_node_ids, vec!["start"]);
    assert_eq!(graph.nodes["answer"].node_type, "answer");
    assert_eq!(graph.edges.len(), 1);
}

#[test]
fn render_template_replaces_dify_selectors() {
    let mut pool = VariablePool::default();
    pool.insert_selector(
        &["sys".to_string(), "query".to_string()],
        Value::String("查订单".to_string()),
    );

    assert_eq!(render_template("用户: {{#sys.query#}}", &pool), "用户: 查订单");
}

struct WorkflowTestProvider;

#[async_trait::async_trait]
impl Provider for WorkflowTestProvider {
    async fn chat_with_system(
        &self,
        _system_prompt: Option<&str>,
        _message: &str,
        _model: &str,
        _temperature: f64,
    ) -> Result<String> {
        Ok(r#"{"type":"orders"}"#.to_string())
    }

    async fn chat(
        &self,
        _request: ChatRequest<'_>,
        _model: &str,
        _temperature: f64,
    ) -> Result<ChatResponse> {
        Ok(ChatResponse {
            text: Some(r#"{"type":"orders"}"#.to_string()),
            tool_calls: Vec::new(),
            usage: None,
            reasoning_content: None,
        })
    }
}

struct ModelFallbackProvider {
    calls: Mutex<Vec<String>>,
}

#[async_trait::async_trait]
impl Provider for ModelFallbackProvider {
    async fn chat_with_system(
        &self,
        _system_prompt: Option<&str>,
        _message: &str,
        _model: &str,
        _temperature: f64,
    ) -> Result<String> {
        Ok("fallback".to_string())
    }

    async fn chat(
        &self,
        _request: ChatRequest<'_>,
        model: &str,
        _temperature: f64,
    ) -> Result<ChatResponse> {
        self.calls.lock().expect("calls lock").push(model.to_string());
        if model == "dify-only-model" {
            return Err(anyhow!("未找到模型：{model}"));
        }
        Ok(ChatResponse {
            text: Some("fallback model response".to_string()),
            tool_calls: Vec::new(),
            usage: None,
            reasoning_content: None,
        })
    }
}

struct ParameterExtractorProvider {
    response: String,
    unavailable_model: Option<String>,
    calls: Mutex<Vec<String>>,
}

#[async_trait::async_trait]
impl Provider for ParameterExtractorProvider {
    async fn chat_with_system(
        &self,
        _system_prompt: Option<&str>,
        _message: &str,
        _model: &str,
        _temperature: f64,
    ) -> Result<String> {
        Ok(self.response.clone())
    }

    async fn chat(
        &self,
        _request: ChatRequest<'_>,
        model: &str,
        _temperature: f64,
    ) -> Result<ChatResponse> {
        self.calls.lock().expect("calls lock").push(model.to_string());
        if self.unavailable_model.as_deref() == Some(model) {
            return Err(anyhow!("model not found: {model}"));
        }
        Ok(ChatResponse {
            text: Some(self.response.clone()),
            tool_calls: Vec::new(),
            usage: None,
            reasoning_content: None,
        })
    }
}

struct FakeKnowledgeProvider {
    requests: Mutex<Vec<WorkflowKnowledgeRequest>>,
}

#[async_trait::async_trait]
impl WorkflowKnowledgeProvider for FakeKnowledgeProvider {
    async fn retrieve(
        &self,
        request: WorkflowKnowledgeRequest,
    ) -> std::result::Result<Vec<WorkflowKnowledgeChunk>, String> {
        self.requests.lock().expect("requests lock").push(request);
        Ok(vec![WorkflowKnowledgeChunk {
            content: "退货需在七天内申请".to_string(),
            title: "退货规则".to_string(),
            metadata: json!({"source": "demo"}),
            score: Some(0.92),
        }])
    }
}

struct FakeWorkflowToolProvider {
    requests: Mutex<Vec<WorkflowToolRequest>>,
}

#[async_trait::async_trait]
impl WorkflowToolProvider for FakeWorkflowToolProvider {
    async fn call(
        &self,
        request: WorkflowToolRequest,
    ) -> std::result::Result<WorkflowToolResult, String> {
        if request.provider != "demo" || request.tool_name != "echo" {
            return Err("workflow tool 未授权或不存在".to_string());
        }
        let text = request
            .inputs
            .get("q")
            .and_then(Value::as_str)
            .ok_or_else(|| "workflow tool 参数 q 类型不匹配".to_string())?
            .to_string();
        self.requests.lock().expect("requests lock").push(request);
        Ok(WorkflowToolResult {
            result: Value::String(text.clone()),
            text: Some(text),
            json: Some(json!({"ok": true})),
            files: Vec::new(),
        })
    }
}

struct DeniedWorkflowToolProvider;

#[async_trait::async_trait]
impl WorkflowToolProvider for DeniedWorkflowToolProvider {
    async fn call(
        &self,
        _request: WorkflowToolRequest,
    ) -> std::result::Result<WorkflowToolResult, String> {
        Err("workflow tool 未授权".to_string())
    }
}

struct FakeWorkflowAgentProvider {
    requests: Mutex<Vec<WorkflowAgentRequest>>,
}

#[async_trait::async_trait]
impl WorkflowAgentProvider for FakeWorkflowAgentProvider {
    async fn run(
        &self,
        request: WorkflowAgentRequest,
        tool_provider: Arc<dyn WorkflowToolProvider>,
    ) -> std::result::Result<WorkflowAgentResult, String> {
        self.requests.lock().expect("requests lock").push(request.clone());
        let prompt =
            request.messages.last().map(|message| message.content.as_str()).unwrap_or_default();
        if prompt.contains("too-many") {
            return Ok(WorkflowAgentResult {
                answer: String::new(),
                tool_outputs: Vec::new(),
                reasoning: Some("stopped late".to_string()),
                iterations: request.max_iterations + 1,
                success: false,
            });
        }
        let tool = request.tools.first().ok_or_else(|| "fake agent missing tool".to_string())?;
        let input = prompt.strip_prefix("echo ").unwrap_or(prompt).to_string();
        let tool_result = tool_provider
            .call(WorkflowToolRequest {
                provider: tool.provider.clone(),
                tool_name: tool.tool_name.clone(),
                action: tool.action.clone(),
                credential_id: tool.credential_id.clone(),
                inputs: BTreeMap::from([("q".to_string(), Value::String(input))]),
            })
            .await?;
        let answer = tool_result.text.clone().unwrap_or_else(|| tool_result.result.to_string());
        Ok(WorkflowAgentResult {
            answer,
            tool_outputs: vec![tool_result.result],
            reasoning: Some("called echo".to_string()),
            iterations: 2,
            success: true,
        })
    }
}

struct FakeWorkflowPauseStore {
    states: Mutex<BTreeMap<String, WorkflowPauseState>>,
}

#[async_trait::async_trait]
impl WorkflowPauseStore for FakeWorkflowPauseStore {
    async fn save(&self, state: WorkflowPauseState) -> std::result::Result<(), String> {
        self.states.lock().expect("states lock").insert(state.run_id.clone(), state);
        Ok(())
    }

    async fn load(&self, run_id: &str) -> std::result::Result<Option<WorkflowPauseState>, String> {
        Ok(self.states.lock().expect("states lock").get(run_id).cloned())
    }

    async fn delete(&self, run_id: &str) -> std::result::Result<(), String> {
        self.states.lock().expect("states lock").remove(run_id);
        Ok(())
    }
}

#[tokio::test]
async fn run_workflow_supports_dify_branch_join_and_answer_continuation() {
    let workflow_yaml = r#"
workflow:
  graph:
    nodes:
      - id: start
        data:
          title: Start
          type: start
          variables:
            - label: Query
              type: paragraph
              variable: query
      - id: intent
        data:
          title: Intent
          type: llm
          model:
            name: qwen-turbo-2025-07-15
          prompt_template:
            - role: user
              text: 用户输入 {{#sys.query#}}
      - id: init
        data:
          title: Init
          type: code
          code_language: python3
          variables:
            - variable: query
              value_selector:
                - start
                - query
          code: |
            def main(query):
                return {"ready": True, "query": query}
      - id: branch
        data:
          title: Branch
          type: if-else
          cases:
            - case_id: orders
              logical_operator: and
              conditions:
                - comparison_operator: contains
                  variable_selector:
                    - intent
                    - text
                  value: '"type":"orders"'
                - comparison_operator: =
                  variable_selector:
                    - init
                    - ready
                  value: true
      - id: progress
        data:
          title: Progress
          type: answer
          answer: |
            正在查询 {{#init.query#}}
      - id: collect
        data:
          title: Collect
          type: code
          code_language: python3
          variables:
            - variable: progress_text
              value_selector:
                - progress
                - text
            - variable: intent_answer
              value_selector:
                - intent
                - answer
          code: |
            def main(progress_text, intent_answer):
                return {
                    "done": True,
                    "message": progress_text.strip() + " / " + intent_answer
                }
      - id: final
        data:
          title: Final
          type: answer
          answer: |
            {{#collect.message#}}
      - id: fallback
        data:
          title: Fallback
          type: answer
          answer: 未识别
    edges:
      - source: start
        sourceHandle: source
        target: intent
      - source: start
        sourceHandle: source
        target: init
      - source: intent
        sourceHandle: source
        target: branch
      - source: init
        sourceHandle: source
        target: branch
      - source: branch
        sourceHandle: orders
        target: progress
      - source: branch
        sourceHandle: false
        target: fallback
      - source: progress
        sourceHandle: source
        target: collect
      - source: collect
        sourceHandle: source
        target: final
"#;
    let runtime = WorkflowRuntime {
        provider: Arc::new(WorkflowTestProvider),
        knowledge_provider: None,
        document_extractor: None,
        tool_provider: None,
        agent_provider: None,
        pause_store: None,
        model: "test-model".to_string(),
        temperature: 0.0,
    };
    let request = WorkflowRunRequest {
        workflow_uuid: None,
        workflow_yaml: Some(workflow_yaml.to_string()),
        query: Some("查询订单".to_string()),
        inputs: Default::default(),
        max_steps: 20,
    };

    let response = run_workflow(runtime, request).await.expect("workflow response");

    assert_eq!(response.status, WorkflowRunStatus::Succeeded);
    assert_eq!(response.answer.as_deref(), Some("正在查询 查询订单 / {\"type\":\"orders\"}\n"));
    assert!(response.nodes.iter().any(|node| node.node_id == "collect"));
    let intent = response.nodes.iter().find(|node| node.node_id == "intent").expect("intent node");
    assert_eq!(intent.outputs.get("answer"), intent.outputs.get("text"));
}

#[tokio::test]
async fn run_workflow_code_node_can_import_requests() {
    let api_url = start_workflow_http_probe_server().await;
    let workflow_yaml = r#"
workflow:
  graph:
    nodes:
      - id: start
        data:
          title: Start
          type: start
          variables:
            - label: Api URL
              type: text-input
              variable: api_url
      - id: probe
        data:
          title: Probe
          type: code
          code_language: python3
          variables:
            - variable: api_url
              value_selector:
                - start
                - api_url
          code: |
            import requests

            def main(api_url):
                response = requests.get(
                    api_url,
                    params={"q": "hello"},
                    headers={"X-VW-Test": "bridge"},
                    timeout=10,
                )
                response.raise_for_status()
                payload = response.json()
                try:
                    requests.get(api_url.rsplit("/", 1)[0] + "/missing", timeout=10).raise_for_status()
                    http_error_is_raised = False
                except requests.exceptions.HTTPError as error:
                    http_error_is_raised = error.response.status_code == 404
                return {
                    "has_get": hasattr(requests, "get"),
                    "has_timeout": hasattr(requests.exceptions, "Timeout"),
                    "has_request_exception": hasattr(requests.exceptions, "RequestException"),
                    "http_error_is_raised": http_error_is_raised,
                    "status_code": response.status_code,
                    "query": payload["query"],
                    "test_header": payload["test_header"]
                }
      - id: answer
        data:
          title: Answer
          type: answer
          answer: "{{#probe.has_get#}}"
    edges:
      - source: start
        sourceHandle: source
        target: probe
      - source: probe
        sourceHandle: source
        target: answer
"#;
    let runtime = WorkflowRuntime {
        provider: Arc::new(WorkflowTestProvider),
        knowledge_provider: None,
        document_extractor: None,
        tool_provider: None,
        agent_provider: None,
        pause_store: None,
        model: "test-model".to_string(),
        temperature: 0.0,
    };
    let request = WorkflowRunRequest {
        workflow_uuid: None,
        workflow_yaml: Some(workflow_yaml.to_string()),
        query: None,
        inputs: BTreeMap::from([("api_url".to_string(), Value::String(api_url))]),
        max_steps: 10,
    };

    let response = run_workflow(runtime, request).await.expect("workflow response");

    assert_eq!(response.status, WorkflowRunStatus::Succeeded);
    assert_eq!(response.answer.as_deref(), Some("true"));
    let probe = response.nodes.iter().find(|node| node.node_id == "probe").expect("probe node");
    assert_eq!(probe.outputs.get("has_timeout"), Some(&Value::Bool(true)));
    assert_eq!(probe.outputs.get("has_request_exception"), Some(&Value::Bool(true)));
    assert_eq!(probe.outputs.get("http_error_is_raised"), Some(&Value::Bool(true)));
    assert_eq!(probe.outputs.get("status_code"), Some(&Value::Number(200.into())));
    assert_eq!(probe.outputs.get("query"), Some(&Value::String("hello".to_string())));
    assert_eq!(probe.outputs.get("test_header"), Some(&Value::String("bridge".to_string())));
}

async fn start_workflow_http_probe_server() -> String {
    async fn probe(
        Query(query): Query<BTreeMap<String, String>>,
        headers: HeaderMap,
    ) -> Json<Value> {
        let test_header =
            headers.get("x-vw-test").and_then(|value| value.to_str().ok()).unwrap_or_default();
        Json(json!({
            "query": query.get("q").cloned().unwrap_or_default(),
            "test_header": test_header,
        }))
    }

    let app = Router::new().route("/probe", get(probe));
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .expect("bind workflow HTTP probe server");
    let addr = listener.local_addr().expect("workflow HTTP probe addr");
    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("workflow HTTP probe server");
    });
    format!("http://{addr}/probe")
}

async fn start_large_workflow_http_server() -> String {
    async fn large() -> String {
        "x".repeat(2 * 1024 * 1024 + 1)
    }

    let app = Router::new().route("/large", get(large));
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .expect("bind workflow large HTTP server");
    let addr = listener.local_addr().expect("workflow large HTTP addr");
    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("workflow large HTTP server");
    });
    format!("http://{addr}/large")
}

#[tokio::test]
async fn run_workflow_falls_back_to_runtime_model_when_dify_model_is_unavailable() {
    let workflow_yaml = r#"
workflow:
  graph:
    nodes:
      - id: start
        data:
          title: Start
          type: start
      - id: llm
        data:
          title: LLM
          type: llm
          model:
            name: dify-only-model
          prompt_template:
            - role: user
              text: hello
      - id: answer
        data:
          title: Answer
          type: answer
          answer: "{{#llm.text#}}"
    edges:
      - source: start
        sourceHandle: source
        target: llm
      - source: llm
        sourceHandle: source
        target: answer
"#;
    let provider = Arc::new(ModelFallbackProvider { calls: Mutex::new(Vec::new()) });
    let runtime = WorkflowRuntime {
        provider: provider.clone(),
        knowledge_provider: None,
        document_extractor: None,
        tool_provider: None,
        agent_provider: None,
        pause_store: None,
        model: "runtime-model".to_string(),
        temperature: 0.0,
    };
    let request = WorkflowRunRequest {
        workflow_uuid: None,
        workflow_yaml: Some(workflow_yaml.to_string()),
        query: Some("hello".to_string()),
        inputs: Default::default(),
        max_steps: 10,
    };

    let response = run_workflow(runtime, request).await.expect("workflow response");

    assert_eq!(response.status, WorkflowRunStatus::Succeeded);
    assert_eq!(response.answer.as_deref(), Some("fallback model response"));
    assert_eq!(
        provider.calls.lock().expect("calls lock").as_slice(),
        ["dify-only-model".to_string(), "runtime-model".to_string()].as_slice()
    );
}

#[tokio::test]
async fn run_workflow_output_node_maps_configured_outputs() {
    let workflow_yaml = r#"
workflow:
  graph:
    nodes:
      - id: start
        data:
          title: Start
          type: start
          variables:
            - variable: name
      - id: out
        data:
          title: Output
          type: output
          outputs:
            - variable: greeting
              value_selector:
                - start
                - name
    edges:
      - source: start
        sourceHandle: source
        target: out
"#;
    let runtime = WorkflowRuntime {
        provider: Arc::new(WorkflowTestProvider),
        knowledge_provider: None,
        document_extractor: None,
        tool_provider: None,
        agent_provider: None,
        pause_store: None,
        model: "test-model".to_string(),
        temperature: 0.0,
    };
    let request = WorkflowRunRequest {
        workflow_uuid: None,
        workflow_yaml: Some(workflow_yaml.to_string()),
        query: None,
        inputs: BTreeMap::from([("name".to_string(), Value::String("Alice".to_string()))]),
        max_steps: 10,
    };

    let response = run_workflow(runtime, request).await.expect("workflow response");

    assert_eq!(response.status, WorkflowRunStatus::Succeeded);
    assert_eq!(response.outputs.get("greeting"), Some(&Value::String("Alice".to_string())));
    let output = response.nodes.iter().find(|node| node.node_id == "out").expect("output node");
    assert_eq!(output.outputs.get("greeting"), Some(&Value::String("Alice".to_string())));
}

#[tokio::test]
async fn run_workflow_end_node_maps_legacy_output_variables() {
    let workflow_yaml = r#"
workflow:
  graph:
    nodes:
      - id: start
        data:
          title: Start
          type: start
          variables:
            - variable: city
      - id: done
        data:
          title: End
          type: end
          output_variables:
            - key: location
              value_selector:
                - start
                - city
    edges:
      - source: start
        sourceHandle: source
        target: done
"#;
    let runtime = WorkflowRuntime {
        provider: Arc::new(WorkflowTestProvider),
        knowledge_provider: None,
        document_extractor: None,
        tool_provider: None,
        agent_provider: None,
        pause_store: None,
        model: "test-model".to_string(),
        temperature: 0.0,
    };
    let request = WorkflowRunRequest {
        workflow_uuid: None,
        workflow_yaml: Some(workflow_yaml.to_string()),
        query: None,
        inputs: BTreeMap::from([("city".to_string(), Value::String("Hangzhou".to_string()))]),
        max_steps: 10,
    };

    let response = run_workflow(runtime, request).await.expect("workflow response");

    assert_eq!(response.status, WorkflowRunStatus::Succeeded);
    assert_eq!(response.outputs.get("location"), Some(&Value::String("Hangzhou".to_string())));
    let end = response.nodes.iter().find(|node| node.node_id == "done").expect("end node");
    assert_eq!(end.outputs.get("location"), Some(&Value::String("Hangzhou".to_string())));
}

#[tokio::test]
async fn run_workflow_template_node_renders_configured_variables() {
    let workflow_yaml = r#"
workflow:
  graph:
    nodes:
      - id: start
        data:
          title: Start
          type: start
          variables:
            - variable: name
            - variable: count
      - id: tpl
        data:
          title: Template
          type: template
          template: "客户 {{ name }} 有 {{ count }} 条订单"
          variables:
            - variable: name
              value_selector:
                - start
                - name
            - variable: count
              value_selector:
                - start
                - count
      - id: answer
        data:
          title: Answer
          type: answer
          answer: "{{#tpl.output#}}"
    edges:
      - source: start
        sourceHandle: source
        target: tpl
      - source: tpl
        sourceHandle: source
        target: answer
"#;
    let runtime = WorkflowRuntime {
        provider: Arc::new(WorkflowTestProvider),
        knowledge_provider: None,
        document_extractor: None,
        tool_provider: None,
        agent_provider: None,
        pause_store: None,
        model: "test-model".to_string(),
        temperature: 0.0,
    };
    let request = WorkflowRunRequest {
        workflow_uuid: None,
        workflow_yaml: Some(workflow_yaml.to_string()),
        query: None,
        inputs: BTreeMap::from([
            ("name".to_string(), Value::String("Alice".to_string())),
            ("count".to_string(), Value::Number(3.into())),
        ]),
        max_steps: 10,
    };

    let response = run_workflow(runtime, request).await.expect("workflow response");

    assert_eq!(response.status, WorkflowRunStatus::Succeeded);
    assert_eq!(response.answer.as_deref(), Some("客户 Alice 有 3 条订单"));
    let template = response.nodes.iter().find(|node| node.node_id == "tpl").expect("template node");
    assert_eq!(
        template.outputs.get("result"),
        Some(&Value::String("客户 Alice 有 3 条订单".to_string()))
    );
}

#[tokio::test]
async fn run_workflow_template_node_supports_control_syntax() {
    let workflow_yaml = r#"
workflow:
  graph:
    nodes:
      - id: start
        data:
          title: Start
          type: start
          variables:
            - variable: items
      - id: tpl
        data:
          title: Template
          type: template-transform
          template: "{% for item in items %}{{ item }}{% endfor %}"
          variables:
            - variable: items
              value_selector:
                - start
                - items
      - id: answer
        data:
          title: Answer
          type: answer
          answer: "{{#tpl.output#}}"
    edges:
      - source: start
        sourceHandle: source
        target: tpl
      - source: tpl
        sourceHandle: source
        target: answer
"#;
    let runtime = WorkflowRuntime {
        provider: Arc::new(WorkflowTestProvider),
        knowledge_provider: None,
        document_extractor: None,
        tool_provider: None,
        agent_provider: None,
        pause_store: None,
        model: "test-model".to_string(),
        temperature: 0.0,
    };
    let request = WorkflowRunRequest {
        workflow_uuid: None,
        workflow_yaml: Some(workflow_yaml.to_string()),
        query: None,
        inputs: BTreeMap::from([("items".to_string(), json!(["paid", "-", "shipped"]))]),
        max_steps: 10,
    };

    let response = run_workflow(runtime, request).await.expect("workflow response");

    assert_eq!(response.status, WorkflowRunStatus::Succeeded);
    assert_eq!(response.answer.as_deref(), Some("paid-shipped"));
}

#[tokio::test]
async fn run_workflow_variable_aggregator_uses_executed_branch_value() {
    let response = run_variable_aggregator_workflow("A").await;

    assert_eq!(response.status, WorkflowRunStatus::Succeeded);
    assert_eq!(response.answer.as_deref(), Some("from-a"));
    assert!(response.nodes.iter().any(|node| node.node_id == "a"));
    assert!(!response.nodes.iter().any(|node| node.node_id == "b"));
}

#[tokio::test]
async fn run_workflow_variable_aggregator_ignores_unexecuted_branch() {
    let response = run_variable_aggregator_workflow("B").await;

    assert_eq!(response.status, WorkflowRunStatus::Succeeded);
    assert_eq!(response.answer.as_deref(), Some("from-b"));
    assert!(!response.nodes.iter().any(|node| node.node_id == "a"));
    assert!(response.nodes.iter().any(|node| node.node_id == "b"));
}

async fn run_variable_aggregator_workflow(
    query: &str,
) -> vw_api_types::workflow::WorkflowRunResponse {
    let workflow_yaml = r#"
workflow:
  graph:
    nodes:
      - id: start
        data:
          title: Start
          type: start
          variables:
            - variable: query
      - id: branch
        data:
          title: Branch
          type: if-else
          cases:
            - case_id: a
              conditions:
                - variable_selector:
                    - start
                    - query
                  comparison_operator: contains
                  value: A
      - id: a
        data:
          title: A
          type: code
          code_language: python3
          code: |
            def main():
                return {"text": "from-a"}
      - id: b
        data:
          title: B
          type: code
          code_language: python3
          code: |
            def main():
                return {"text": "from-b"}
      - id: agg
        data:
          title: Aggregate
          type: variable-aggregator
          variables:
            - variable: text
              selectors:
                - [a, text]
                - [b, text]
      - id: answer
        data:
          title: Answer
          type: answer
          answer: "{{#agg.text#}}"
    edges:
      - source: start
        sourceHandle: source
        target: branch
      - source: branch
        sourceHandle: a
        target: a
      - source: branch
        sourceHandle: false
        target: b
      - source: a
        sourceHandle: source
        target: agg
      - source: b
        sourceHandle: source
        target: agg
      - source: agg
        sourceHandle: source
        target: answer
"#;
    let runtime = WorkflowRuntime {
        provider: Arc::new(WorkflowTestProvider),
        knowledge_provider: None,
        document_extractor: None,
        tool_provider: None,
        agent_provider: None,
        pause_store: None,
        model: "test-model".to_string(),
        temperature: 0.0,
    };
    let request = WorkflowRunRequest {
        workflow_uuid: None,
        workflow_yaml: Some(workflow_yaml.to_string()),
        query: None,
        inputs: BTreeMap::from([("query".to_string(), Value::String(query.to_string()))]),
        max_steps: 20,
    };

    run_workflow(runtime, request).await.expect("workflow response")
}

#[tokio::test]
async fn run_workflow_variable_aggregator_rejects_type_mismatch() {
    let workflow_yaml = r#"
workflow:
  graph:
    nodes:
      - id: start
        data:
          title: Start
          type: start
      - id: text
        data:
          title: Text
          type: code
          code_language: python3
          code: |
            def main():
                return {"value": "one"}
      - id: number
        data:
          title: Number
          type: code
          code_language: python3
          code: |
            def main():
                return {"value": 2}
      - id: agg
        data:
          title: Aggregate
          type: variable-aggregator
          groups:
            - name: value
              selectors:
                - [text, value]
                - [number, value]
    edges:
      - source: start
        sourceHandle: source
        target: text
      - source: start
        sourceHandle: source
        target: number
      - source: text
        sourceHandle: source
        target: agg
      - source: number
        sourceHandle: source
        target: agg
"#;
    let runtime = WorkflowRuntime {
        provider: Arc::new(WorkflowTestProvider),
        knowledge_provider: None,
        document_extractor: None,
        tool_provider: None,
        agent_provider: None,
        pause_store: None,
        model: "test-model".to_string(),
        temperature: 0.0,
    };
    let request = WorkflowRunRequest {
        workflow_uuid: None,
        workflow_yaml: Some(workflow_yaml.to_string()),
        query: None,
        inputs: Default::default(),
        max_steps: 20,
    };

    let response = run_workflow(runtime, request).await.expect("workflow response");

    assert_eq!(response.status, WorkflowRunStatus::Failed);
    assert!(response.error.expect("error").contains("候选值类型不一致"));
}

#[tokio::test]
async fn run_workflow_list_operator_filters_array_items() {
    let workflow_yaml = r#"
workflow:
  graph:
    nodes:
      - id: start
        data:
          title: Start
          type: start
          variables:
            - variable: files
      - id: docs
        data:
          title: Docs
          type: list-operator
          input_selector:
            - start
            - files
          filter:
            field: type
            operator: in
            value:
              - document
      - id: answer
        data:
          title: Answer
          type: answer
          answer: "{{#docs.first_record.name#}}"
    edges:
      - source: start
        sourceHandle: source
        target: docs
      - source: docs
        sourceHandle: source
        target: answer
"#;
    let runtime = WorkflowRuntime {
        provider: Arc::new(WorkflowTestProvider),
        knowledge_provider: None,
        document_extractor: None,
        tool_provider: None,
        agent_provider: None,
        pause_store: None,
        model: "test-model".to_string(),
        temperature: 0.0,
    };
    let request = WorkflowRunRequest {
        workflow_uuid: None,
        workflow_yaml: Some(workflow_yaml.to_string()),
        query: None,
        inputs: BTreeMap::from([(
            "files".to_string(),
            json!([
                { "type": "image", "name": "a.png" },
                { "type": "document", "name": "b.pdf" }
            ]),
        )]),
        max_steps: 10,
    };

    let response = run_workflow(runtime, request).await.expect("workflow response");

    assert_eq!(response.status, WorkflowRunStatus::Succeeded);
    assert_eq!(response.answer.as_deref(), Some("b.pdf"));
    let docs = response.nodes.iter().find(|node| node.node_id == "docs").expect("docs node");
    assert_eq!(docs.outputs.get("result"), Some(&json!([{ "type": "document", "name": "b.pdf" }])));
    assert_eq!(
        docs.outputs.get("first_record").and_then(|value| value.get("name")),
        Some(&Value::String("b.pdf".to_string()))
    );
}

#[tokio::test]
async fn run_workflow_list_operator_returns_null_records_for_empty_array() {
    let workflow_yaml = r#"
workflow:
  graph:
    nodes:
      - id: start
        data:
          title: Start
          type: start
          variables:
            - variable: files
      - id: docs
        data:
          title: Docs
          type: list-operator
          input_selector:
            - start
            - files
      - id: answer
        data:
          title: Answer
          type: answer
          answer: "{{#docs.first_record#}}"
    edges:
      - source: start
        sourceHandle: source
        target: docs
      - source: docs
        sourceHandle: source
        target: answer
"#;
    let runtime = WorkflowRuntime {
        provider: Arc::new(WorkflowTestProvider),
        knowledge_provider: None,
        document_extractor: None,
        tool_provider: None,
        agent_provider: None,
        pause_store: None,
        model: "test-model".to_string(),
        temperature: 0.0,
    };
    let request = WorkflowRunRequest {
        workflow_uuid: None,
        workflow_yaml: Some(workflow_yaml.to_string()),
        query: None,
        inputs: BTreeMap::from([("files".to_string(), Value::Array(Vec::new()))]),
        max_steps: 10,
    };

    let response = run_workflow(runtime, request).await.expect("workflow response");

    assert_eq!(response.status, WorkflowRunStatus::Succeeded);
    let docs = response.nodes.iter().find(|node| node.node_id == "docs").expect("docs node");
    assert_eq!(docs.outputs.get("first_record"), Some(&Value::Null));
    assert_eq!(docs.outputs.get("last_record"), Some(&Value::Null));
}

#[tokio::test]
async fn run_workflow_list_operator_sorts_array_items() {
    let workflow_yaml = r#"
workflow:
  graph:
    nodes:
      - id: start
        data:
          title: Start
          type: start
          variables:
            - variable: files
      - id: sorted
        data:
          title: Sorted
          type: list-operator
          input_selector:
            - start
            - files
          sort:
            field: size
            order: desc
    edges:
      - source: start
        sourceHandle: source
        target: sorted
"#;
    let runtime = WorkflowRuntime {
        provider: Arc::new(WorkflowTestProvider),
        knowledge_provider: None,
        document_extractor: None,
        tool_provider: None,
        agent_provider: None,
        pause_store: None,
        model: "test-model".to_string(),
        temperature: 0.0,
    };
    let request = WorkflowRunRequest {
        workflow_uuid: None,
        workflow_yaml: Some(workflow_yaml.to_string()),
        query: None,
        inputs: BTreeMap::from([(
            "files".to_string(),
            json!([
                { "name": "small.txt", "size": 1 },
                { "name": "large.txt", "size": 10 }
            ]),
        )]),
        max_steps: 10,
    };

    let response = run_workflow(runtime, request).await.expect("workflow response");

    assert_eq!(response.status, WorkflowRunStatus::Succeeded);
    let sorted = response.nodes.iter().find(|node| node.node_id == "sorted").expect("sorted node");
    assert_eq!(
        sorted.outputs.get("first_record").and_then(|value| value.get("name")),
        Some(&Value::String("large.txt".to_string()))
    );
}

#[tokio::test]
async fn run_workflow_list_operator_rejects_unsupported_filter_operator() {
    let workflow_yaml = r#"
workflow:
  graph:
    nodes:
      - id: start
        data:
          title: Start
          type: start
          variables:
            - variable: files
      - id: docs
        data:
          title: Docs
          type: list-operator
          input_selector:
            - start
            - files
          filter:
            field: type
            operator: file-type
            value: document
    edges:
      - source: start
        sourceHandle: source
        target: docs
"#;
    let runtime = WorkflowRuntime {
        provider: Arc::new(WorkflowTestProvider),
        knowledge_provider: None,
        document_extractor: None,
        tool_provider: None,
        agent_provider: None,
        pause_store: None,
        model: "test-model".to_string(),
        temperature: 0.0,
    };
    let request = WorkflowRunRequest {
        workflow_uuid: None,
        workflow_yaml: Some(workflow_yaml.to_string()),
        query: None,
        inputs: BTreeMap::from([(
            "files".to_string(),
            json!([{ "type": "document", "name": "b.pdf" }]),
        )]),
        max_steps: 10,
    };

    let response = run_workflow(runtime, request).await.expect("workflow response");

    assert_eq!(response.status, WorkflowRunStatus::Failed);
    assert!(response.error.expect("error").contains("不支持的 list-operator"));
}

#[tokio::test]
async fn run_workflow_http_request_node_returns_status_body_and_json() {
    let api_url = start_workflow_http_probe_server().await;
    let workflow_yaml = r#"
workflow:
  graph:
    nodes:
      - id: start
        data:
          title: Start
          type: start
          variables:
            - variable: api_url
      - id: http
        data:
          title: HTTP
          type: http-request
          method: GET
          url: "{{ api_url }}"
          timeout: 10
          variables:
            - variable: api_url
              value_selector:
                - start
                - api_url
          params:
            q: hello
          headers:
            X-VW-Test: workflow-http
            Authorization: Bearer secret-token
      - id: answer
        data:
          title: Answer
          type: answer
          answer: "{{#http.status_code#}} {{#http.json.query#}} {{#http.json.test_header#}}"
    edges:
      - source: start
        sourceHandle: source
        target: http
      - source: http
        sourceHandle: source
        target: answer
"#;
    let runtime = WorkflowRuntime {
        provider: Arc::new(WorkflowTestProvider),
        knowledge_provider: None,
        document_extractor: None,
        tool_provider: None,
        agent_provider: None,
        pause_store: None,
        model: "test-model".to_string(),
        temperature: 0.0,
    };
    let request = WorkflowRunRequest {
        workflow_uuid: None,
        workflow_yaml: Some(workflow_yaml.to_string()),
        query: None,
        inputs: BTreeMap::from([("api_url".to_string(), Value::String(api_url))]),
        max_steps: 10,
    };

    let response = run_workflow(runtime, request).await.expect("workflow response");

    assert_eq!(response.status, WorkflowRunStatus::Succeeded);
    assert_eq!(response.answer.as_deref(), Some("200 hello workflow-http"));
    let http = response.nodes.iter().find(|node| node.node_id == "http").expect("http node");
    assert_eq!(http.outputs.get("status_code"), Some(&Value::Number(200.into())));
    assert_eq!(
        http.outputs.get("json").and_then(|value| value.get("query")),
        Some(&Value::String("hello".to_string()))
    );
    assert_eq!(
        http.inputs.get("headers").and_then(|headers| headers.get("Authorization")),
        Some(&Value::String("[REDACTED]".to_string()))
    );
}

#[tokio::test]
async fn run_workflow_http_request_node_rejects_non_http_url() {
    let workflow_yaml = r#"
workflow:
  graph:
    nodes:
      - id: start
        data:
          title: Start
          type: start
      - id: http
        data:
          title: HTTP
          type: http-request
          method: GET
          url: "file:///tmp/secret"
    edges:
      - source: start
        sourceHandle: source
        target: http
"#;
    let runtime = WorkflowRuntime {
        provider: Arc::new(WorkflowTestProvider),
        knowledge_provider: None,
        document_extractor: None,
        tool_provider: None,
        agent_provider: None,
        pause_store: None,
        model: "test-model".to_string(),
        temperature: 0.0,
    };
    let request = WorkflowRunRequest {
        workflow_uuid: None,
        workflow_yaml: Some(workflow_yaml.to_string()),
        query: None,
        inputs: Default::default(),
        max_steps: 10,
    };

    let response = run_workflow(runtime, request).await.expect("workflow response");

    assert_eq!(response.status, WorkflowRunStatus::Failed);
    assert!(response.error.expect("error").contains("仅支持 http/https"));
}

#[tokio::test]
async fn run_workflow_http_request_node_rejects_large_response() {
    let api_url = start_large_workflow_http_server().await;
    let workflow_yaml = r#"
workflow:
  graph:
    nodes:
      - id: start
        data:
          title: Start
          type: start
          variables:
            - variable: api_url
      - id: http
        data:
          title: HTTP
          type: http-request
          method: GET
          url: "{{ api_url }}"
          variables:
            - variable: api_url
              value_selector:
                - start
                - api_url
    edges:
      - source: start
        sourceHandle: source
        target: http
"#;
    let runtime = WorkflowRuntime {
        provider: Arc::new(WorkflowTestProvider),
        knowledge_provider: None,
        document_extractor: None,
        tool_provider: None,
        agent_provider: None,
        pause_store: None,
        model: "test-model".to_string(),
        temperature: 0.0,
    };
    let request = WorkflowRunRequest {
        workflow_uuid: None,
        workflow_yaml: Some(workflow_yaml.to_string()),
        query: None,
        inputs: BTreeMap::from([("api_url".to_string(), Value::String(api_url))]),
        max_steps: 10,
    };

    let response = run_workflow(runtime, request).await.expect("workflow response");

    assert_eq!(response.status, WorkflowRunStatus::Failed);
    assert!(response.error.expect("error").contains("响应体超过限制"));
}

#[tokio::test]
async fn run_workflow_parameter_extractor_extracts_parameters() {
    let workflow_yaml = parameter_extractor_workflow(None);
    let provider = Arc::new(ParameterExtractorProvider {
        response: r#"{"city":"上海","days":3}"#.to_string(),
        unavailable_model: None,
        calls: Mutex::new(Vec::new()),
    });
    let runtime = WorkflowRuntime {
        provider,
        knowledge_provider: None,
        document_extractor: None,
        tool_provider: None,
        agent_provider: None,
        pause_store: None,
        model: "runtime-model".to_string(),
        temperature: 0.0,
    };
    let request = WorkflowRunRequest {
        workflow_uuid: None,
        workflow_yaml: Some(workflow_yaml),
        query: None,
        inputs: BTreeMap::from([(
            "query".to_string(),
            Value::String("查上海未来3天天气".to_string()),
        )]),
        max_steps: 10,
    };

    let response = run_workflow(runtime, request).await.expect("workflow response");

    assert_eq!(response.status, WorkflowRunStatus::Succeeded);
    assert_eq!(response.answer.as_deref(), Some("上海/3/1"));
    let extractor =
        response.nodes.iter().find(|node| node.node_id == "extractor").expect("extractor node");
    assert_eq!(extractor.outputs.get("city"), Some(&Value::String("上海".to_string())));
    assert_eq!(extractor.outputs.get("__reason"), Some(&Value::String(String::new())));
}

#[tokio::test]
async fn run_workflow_parameter_extractor_marks_missing_required_parameter() {
    let workflow_yaml = parameter_extractor_workflow(None);
    let provider = Arc::new(ParameterExtractorProvider {
        response: r#"{"days":3}"#.to_string(),
        unavailable_model: None,
        calls: Mutex::new(Vec::new()),
    });
    let runtime = WorkflowRuntime {
        provider,
        knowledge_provider: None,
        document_extractor: None,
        tool_provider: None,
        agent_provider: None,
        pause_store: None,
        model: "runtime-model".to_string(),
        temperature: 0.0,
    };
    let request = WorkflowRunRequest {
        workflow_uuid: None,
        workflow_yaml: Some(workflow_yaml),
        query: None,
        inputs: BTreeMap::from([("query".to_string(), Value::String("查未来3天天气".to_string()))]),
        max_steps: 10,
    };

    let response = run_workflow(runtime, request).await.expect("workflow response");

    assert_eq!(response.status, WorkflowRunStatus::Succeeded);
    assert_eq!(response.answer.as_deref(), Some("/3/0"));
    let extractor =
        response.nodes.iter().find(|node| node.node_id == "extractor").expect("extractor node");
    assert!(
        extractor
            .outputs
            .get("__reason")
            .and_then(Value::as_str)
            .is_some_and(|reason| reason.contains("city"))
    );
}

#[tokio::test]
async fn run_workflow_parameter_extractor_rejects_non_json_response() {
    let workflow_yaml = parameter_extractor_workflow(None);
    let provider = Arc::new(ParameterExtractorProvider {
        response: "city is 上海".to_string(),
        unavailable_model: None,
        calls: Mutex::new(Vec::new()),
    });
    let runtime = WorkflowRuntime {
        provider,
        knowledge_provider: None,
        document_extractor: None,
        tool_provider: None,
        agent_provider: None,
        pause_store: None,
        model: "runtime-model".to_string(),
        temperature: 0.0,
    };
    let request = WorkflowRunRequest {
        workflow_uuid: None,
        workflow_yaml: Some(workflow_yaml),
        query: None,
        inputs: BTreeMap::from([("query".to_string(), Value::String("查上海天气".to_string()))]),
        max_steps: 10,
    };

    let response = run_workflow(runtime, request).await.expect("workflow response");

    assert_eq!(response.status, WorkflowRunStatus::Failed);
    assert!(response.error.expect("error").contains("返回非 JSON"));
}

#[tokio::test]
async fn run_workflow_parameter_extractor_uses_model_fallback() {
    let workflow_yaml = parameter_extractor_workflow(Some("dify-only-model"));
    let provider = Arc::new(ParameterExtractorProvider {
        response: r#"{"city":"上海","days":3}"#.to_string(),
        unavailable_model: Some("dify-only-model".to_string()),
        calls: Mutex::new(Vec::new()),
    });
    let runtime = WorkflowRuntime {
        provider: provider.clone(),
        knowledge_provider: None,
        document_extractor: None,
        tool_provider: None,
        agent_provider: None,
        pause_store: None,
        model: "runtime-model".to_string(),
        temperature: 0.0,
    };
    let request = WorkflowRunRequest {
        workflow_uuid: None,
        workflow_yaml: Some(workflow_yaml),
        query: None,
        inputs: BTreeMap::from([(
            "query".to_string(),
            Value::String("查上海未来3天天气".to_string()),
        )]),
        max_steps: 10,
    };

    let response = run_workflow(runtime, request).await.expect("workflow response");

    assert_eq!(response.status, WorkflowRunStatus::Succeeded);
    assert_eq!(
        provider.calls.lock().expect("calls lock").as_slice(),
        ["dify-only-model".to_string(), "runtime-model".to_string()].as_slice()
    );
}

fn parameter_extractor_workflow(model: Option<&str>) -> String {
    let model_yaml = model
        .map(|model| format!("          model:\n            name: {model}\n"))
        .unwrap_or_default();
    format!(
        r#"
workflow:
  graph:
    nodes:
      - id: start
        data:
          title: Start
          type: start
          variables:
            - variable: query
      - id: extractor
        data:
          title: Extractor
          type: parameter-extractor
{model_yaml}          input_selector:
            - start
            - query
          parameters:
            - name: city
              type: string
              required: true
              description: 城市名称
            - name: days
              type: number
              required: false
              description: 查询天数
      - id: answer
        data:
          title: Answer
          type: answer
          answer: "{{{{#extractor.city#}}}}/{{{{#extractor.days#}}}}/{{{{#extractor.__is_success#}}}}"
    edges:
      - source: start
        sourceHandle: source
        target: extractor
      - source: extractor
        sourceHandle: source
        target: answer
"#
    )
}

#[tokio::test]
async fn run_workflow_question_classifier_activates_matched_class_edge() {
    let provider = Arc::new(ParameterExtractorProvider {
        response: "order".to_string(),
        unavailable_model: None,
        calls: Mutex::new(Vec::new()),
    });
    let runtime = WorkflowRuntime {
        provider,
        knowledge_provider: None,
        document_extractor: None,
        tool_provider: None,
        agent_provider: None,
        pause_store: None,
        model: "runtime-model".to_string(),
        temperature: 0.0,
    };
    let request = WorkflowRunRequest {
        workflow_uuid: None,
        workflow_yaml: Some(question_classifier_workflow(false)),
        query: None,
        inputs: BTreeMap::from([(
            "query".to_string(),
            Value::String("查一下昨天订单".to_string()),
        )]),
        max_steps: 10,
    };

    let response = run_workflow(runtime, request).await.expect("workflow response");

    assert_eq!(response.status, WorkflowRunStatus::Succeeded);
    assert_eq!(response.answer.as_deref(), Some("order"));
    let classifier =
        response.nodes.iter().find(|node| node.node_id == "classifier").expect("classifier node");
    assert_eq!(classifier.selected_handle.as_deref(), Some("order"));
    assert_eq!(classifier.outputs.get("class_name"), Some(&Value::String("订单".to_string())));
}

#[tokio::test]
async fn run_workflow_question_classifier_uses_default_for_unknown_class() {
    let provider = Arc::new(ParameterExtractorProvider {
        response: "billing".to_string(),
        unavailable_model: None,
        calls: Mutex::new(Vec::new()),
    });
    let runtime = WorkflowRuntime {
        provider,
        knowledge_provider: None,
        document_extractor: None,
        tool_provider: None,
        agent_provider: None,
        pause_store: None,
        model: "runtime-model".to_string(),
        temperature: 0.0,
    };
    let request = WorkflowRunRequest {
        workflow_uuid: None,
        workflow_yaml: Some(question_classifier_workflow(true)),
        query: None,
        inputs: BTreeMap::from([("query".to_string(), Value::String("这是什么问题".to_string()))]),
        max_steps: 10,
    };

    let response = run_workflow(runtime, request).await.expect("workflow response");

    assert_eq!(response.status, WorkflowRunStatus::Succeeded);
    assert_eq!(response.answer.as_deref(), Some("default"));
    let classifier =
        response.nodes.iter().find(|node| node.node_id == "classifier").expect("classifier node");
    assert_eq!(classifier.selected_handle.as_deref(), Some("default"));
}

#[tokio::test]
async fn run_workflow_question_classifier_rejects_unknown_class_without_fallback() {
    let provider = Arc::new(ParameterExtractorProvider {
        response: "billing".to_string(),
        unavailable_model: None,
        calls: Mutex::new(Vec::new()),
    });
    let runtime = WorkflowRuntime {
        provider,
        knowledge_provider: None,
        document_extractor: None,
        tool_provider: None,
        agent_provider: None,
        pause_store: None,
        model: "runtime-model".to_string(),
        temperature: 0.0,
    };
    let request = WorkflowRunRequest {
        workflow_uuid: None,
        workflow_yaml: Some(question_classifier_workflow(false)),
        query: None,
        inputs: BTreeMap::from([("query".to_string(), Value::String("这是什么问题".to_string()))]),
        max_steps: 10,
    };

    let response = run_workflow(runtime, request).await.expect("workflow response");

    assert_eq!(response.status, WorkflowRunStatus::Failed);
    assert!(response.error.expect("error").contains("未知分类 id"));
}

fn question_classifier_workflow(include_default: bool) -> String {
    let default_node = if include_default {
        r#"
      - id: default_answer
        data:
          title: Default Answer
          type: answer
          answer: default
"#
    } else {
        ""
    };
    let default_edge = if include_default {
        r#"
      - source: classifier
        sourceHandle: default
        target: default_answer
"#
    } else {
        ""
    };
    format!(
        r#"
workflow:
  graph:
    nodes:
      - id: start
        data:
          title: Start
          type: start
          variables:
            - variable: query
      - id: classifier
        data:
          title: Classifier
          type: question-classifier
          input_selector:
            - start
            - query
          classes:
            - id: order
              name: 订单
              description: 订单查询、订单状态、订单金额
            - id: other
              name: 其他
              description: 不属于订单的问题
      - id: order_answer
        data:
          title: Order Answer
          type: answer
          answer: order
      - id: other_answer
        data:
          title: Other Answer
          type: answer
          answer: other
{default_node}    edges:
      - source: start
        sourceHandle: source
        target: classifier
      - source: classifier
        sourceHandle: order
        target: order_answer
      - source: classifier
        sourceHandle: other
        target: other_answer
{default_edge}"#
    )
}

#[tokio::test]
async fn run_workflow_knowledge_retrieval_returns_provider_chunks() {
    let knowledge_provider = Arc::new(FakeKnowledgeProvider { requests: Mutex::new(Vec::new()) });
    let runtime = WorkflowRuntime {
        provider: Arc::new(WorkflowTestProvider),
        knowledge_provider: Some(knowledge_provider.clone()),
        document_extractor: None,
        tool_provider: None,
        agent_provider: None,
        pause_store: None,
        model: "test-model".to_string(),
        temperature: 0.0,
    };
    let request = WorkflowRunRequest {
        workflow_uuid: None,
        workflow_yaml: Some(knowledge_retrieval_workflow()),
        query: None,
        inputs: BTreeMap::from([("query".to_string(), Value::String("退货规则".to_string()))]),
        max_steps: 10,
    };

    let response = run_workflow(runtime, request).await.expect("workflow response");

    assert_eq!(response.status, WorkflowRunStatus::Succeeded);
    assert!(response.answer.as_deref().is_some_and(|answer| answer.contains("退货需在七天内申请")));
    let kb = response.nodes.iter().find(|node| node.node_id == "kb").expect("kb node");
    assert_eq!(
        kb.outputs
            .get("result")
            .and_then(|result| result.get(0))
            .and_then(|chunk| chunk.get("title")),
        Some(&Value::String("退货规则".to_string()))
    );
    let requests = knowledge_provider.requests.lock().expect("requests lock");
    assert_eq!(requests[0].query, "退货规则");
    assert_eq!(requests[0].dataset_ids, ["demo".to_string()]);
    assert_eq!(requests[0].top_k, 3);
}

#[tokio::test]
async fn run_workflow_knowledge_retrieval_requires_provider() {
    let runtime = WorkflowRuntime {
        provider: Arc::new(WorkflowTestProvider),
        knowledge_provider: None,
        document_extractor: None,
        tool_provider: None,
        agent_provider: None,
        pause_store: None,
        model: "test-model".to_string(),
        temperature: 0.0,
    };
    let request = WorkflowRunRequest {
        workflow_uuid: None,
        workflow_yaml: Some(knowledge_retrieval_workflow()),
        query: None,
        inputs: BTreeMap::from([("query".to_string(), Value::String("退货规则".to_string()))]),
        max_steps: 10,
    };

    let response = run_workflow(runtime, request).await.expect("workflow response");

    assert_eq!(response.status, WorkflowRunStatus::Failed);
    assert!(response.error.expect("error").contains("knowledge provider 未配置"));
}

fn knowledge_retrieval_workflow() -> String {
    r#"
workflow:
  graph:
    nodes:
      - id: start
        data:
          title: Start
          type: start
          variables:
            - variable: query
      - id: kb
        data:
          title: Knowledge
          type: knowledge-retrieval
          query_selector:
            - start
            - query
          dataset_ids:
            - demo
          top_k: 3
      - id: answer
        data:
          title: Answer
          type: answer
          answer: "{{#kb.result#}}"
    edges:
      - source: start
        sourceHandle: source
        target: kb
      - source: kb
        sourceHandle: source
        target: answer
"#
    .to_string()
}

#[tokio::test]
async fn run_workflow_knowledge_retrieval_reads_dify_native_fields() {
    let knowledge_provider = Arc::new(FakeKnowledgeProvider { requests: Mutex::new(Vec::new()) });
    let runtime = WorkflowRuntime {
        provider: Arc::new(WorkflowTestProvider),
        knowledge_provider: Some(knowledge_provider.clone()),
        document_extractor: None,
        tool_provider: None,
        agent_provider: None,
        pause_store: None,
        model: "test-model".to_string(),
        temperature: 0.0,
    };
    let request = WorkflowRunRequest {
        workflow_uuid: None,
        workflow_yaml: Some(dify_native_knowledge_retrieval_workflow()),
        query: None,
        inputs: BTreeMap::from([("query".to_string(), Value::String("退货规则".to_string()))]),
        max_steps: 10,
    };

    let response = run_workflow(runtime, request).await.expect("workflow response");

    assert_eq!(response.status, WorkflowRunStatus::Succeeded);
    let requests = knowledge_provider.requests.lock().expect("requests lock");
    assert_eq!(requests[0].query, "退货规则");
    assert_eq!(requests[0].top_k, 10);
    assert_eq!(requests[0].dataset_ids, ["demo".to_string()]);
}

fn dify_native_knowledge_retrieval_workflow() -> String {
    r#"
workflow:
  graph:
    nodes:
      - id: start
        data:
          title: Start
          type: start
          variables:
            - variable: query
      - id: kb
        data:
          title: Knowledge
          type: knowledge-retrieval
          query_variable_selector:
            - start
            - query
          dataset_ids:
            - demo
          multiple_retrieval_config:
            top_k: 10
      - id: answer
        data:
          title: Answer
          type: answer
          answer: "{{#kb.result#}}"
    edges:
      - source: start
        sourceHandle: source
        target: kb
      - source: kb
        sourceHandle: source
        target: answer
"#
    .to_string()
}

#[tokio::test]
async fn run_workflow_dhb_demo_yaml_runs_end_to_end() {
    let knowledge_provider = Arc::new(FakeKnowledgeProvider { requests: Mutex::new(Vec::new()) });
    let runtime = WorkflowRuntime {
        provider: Arc::new(WorkflowTestProvider),
        knowledge_provider: Some(knowledge_provider.clone()),
        document_extractor: None,
        tool_provider: None,
        agent_provider: None,
        pause_store: None,
        model: "test-model".to_string(),
        temperature: 0.0,
    };
    let request = WorkflowRunRequest {
        workflow_uuid: None,
        workflow_yaml: Some(dhb_demo_workflow()),
        query: Some("如何批量导入商品？".to_string()),
        inputs: BTreeMap::new(),
        max_steps: 20,
    };

    let response = run_workflow(runtime, request).await.expect("workflow response");

    assert_eq!(response.status, WorkflowRunStatus::Succeeded);
    let requests = knowledge_provider.requests.lock().expect("requests lock");
    assert_eq!(requests[0].query, "如何批量导入商品？");
    assert_eq!(requests[0].dataset_ids, ["ff2277f0-f858-4801-80cb-2545e3aa9d26".to_string()]);
    assert_eq!(requests[0].top_k, 10);
}

/// 精简等价于 `dhb-zhishiku.yml`：start → knowledge-retrieval → llm → answer，
/// 保留检索节点原生 `query_variable_selector` 与 `multiple_retrieval_config.top_k:10`，
/// dataset_ids 为本地 UUID，LLM 节点模型为 `deepseek/deepseek-v4-pro`。
fn dhb_demo_workflow() -> String {
    r#"
workflow:
  graph:
    nodes:
      - id: start
        data:
          title: 开始
          type: start
          variables:
            - variable: query
      - id: kb
        data:
          title: 问答_帮助文档
          type: knowledge-retrieval
          query_variable_selector:
            - start
            - sys.query
          dataset_ids:
            - ff2277f0-f858-4801-80cb-2545e3aa9d26
          retrieval_mode: multiple
          multiple_retrieval_config:
            top_k: 10
            score_threshold: null
      - id: llm
        data:
          title: 客户问答助手
          type: llm
          model:
            name: deepseek/deepseek-v4-pro
            provider: langgenius/tongyi/tongyi
          context:
            enabled: true
            variable_selector:
              - kb
              - result
          prompt_template:
            - role: system
              text: |
                <context>
                {{#context#}}
                </context>
            - role: user
              text: '{{#sys.query#}}'
      - id: answer
        data:
          title: 直接回复
          type: answer
          answer: '{{#llm.text#}}'
    edges:
      - source: start
        sourceHandle: source
        target: kb
      - source: kb
        sourceHandle: source
        target: llm
      - source: llm
        sourceHandle: source
        target: answer
"#
    .to_string()
}

#[tokio::test]
async fn run_workflow_document_extractor_reads_inline_text_file() {
    let runtime = WorkflowRuntime {
        provider: Arc::new(WorkflowTestProvider),
        knowledge_provider: None,
        document_extractor: None,
        tool_provider: None,
        agent_provider: None,
        pause_store: None,
        model: "test-model".to_string(),
        temperature: 0.0,
    };
    let request = WorkflowRunRequest {
        workflow_uuid: None,
        workflow_yaml: Some(document_extractor_workflow()),
        query: None,
        inputs: BTreeMap::from([(
            "file".to_string(),
            json!({
                "name": "note.txt",
                "mime_type": "text/plain",
                "text": "hello doc"
            }),
        )]),
        max_steps: 10,
    };

    let response = run_workflow(runtime, request).await.expect("workflow response");

    assert_eq!(response.status, WorkflowRunStatus::Succeeded);
    assert_eq!(response.answer.as_deref(), Some("hello doc"));
    let doc = response.nodes.iter().find(|node| node.node_id == "doc").expect("doc node");
    assert_eq!(doc.outputs.get("text"), Some(&Value::String("hello doc".to_string())));
    assert_eq!(
        doc.outputs.get("files").and_then(|files| files.get(0)).and_then(|file| file.get("name")),
        Some(&Value::String("note.txt".to_string()))
    );
}

#[tokio::test]
async fn run_workflow_document_extractor_rejects_unsupported_inline_format() {
    let runtime = WorkflowRuntime {
        provider: Arc::new(WorkflowTestProvider),
        knowledge_provider: None,
        document_extractor: None,
        tool_provider: None,
        agent_provider: None,
        pause_store: None,
        model: "test-model".to_string(),
        temperature: 0.0,
    };
    let request = WorkflowRunRequest {
        workflow_uuid: None,
        workflow_yaml: Some(document_extractor_workflow()),
        query: None,
        inputs: BTreeMap::from([(
            "file".to_string(),
            json!({
                "name": "paper.pdf",
                "mime_type": "application/pdf",
                "text": "pdf text"
            }),
        )]),
        max_steps: 10,
    };

    let response = run_workflow(runtime, request).await.expect("workflow response");

    assert_eq!(response.status, WorkflowRunStatus::Failed);
    assert!(response.error.expect("error").contains("不支持的文件格式"));
}

#[tokio::test]
async fn run_workflow_document_extractor_does_not_read_path_without_provider() {
    let runtime = WorkflowRuntime {
        provider: Arc::new(WorkflowTestProvider),
        knowledge_provider: None,
        document_extractor: None,
        tool_provider: None,
        agent_provider: None,
        pause_store: None,
        model: "test-model".to_string(),
        temperature: 0.0,
    };
    let request = WorkflowRunRequest {
        workflow_uuid: None,
        workflow_yaml: Some(document_extractor_workflow()),
        query: None,
        inputs: BTreeMap::from([(
            "file".to_string(),
            json!({
                "name": "secret.txt",
                "mime_type": "text/plain",
                "path": "/etc/passwd",
                "size": 128
            }),
        )]),
        max_steps: 10,
    };

    let response = run_workflow(runtime, request).await.expect("workflow response");

    assert_eq!(response.status, WorkflowRunStatus::Failed);
    assert!(response.error.expect("error").contains("document extractor provider 未配置"));
}

fn document_extractor_workflow() -> String {
    r#"
workflow:
  graph:
    nodes:
      - id: start
        data:
          title: Start
          type: start
          variables:
            - variable: file
      - id: doc
        data:
          title: Document
          type: document-extractor
          input_selector:
            - start
            - file
      - id: answer
        data:
          title: Answer
          type: answer
          answer: "{{#doc.text#}}"
    edges:
      - source: start
        sourceHandle: source
        target: doc
      - source: doc
        sourceHandle: source
        target: answer
"#
    .to_string()
}

#[tokio::test]
async fn run_workflow_variable_assigner_appends_runtime_conversation_value() {
    let runtime = WorkflowRuntime {
        provider: Arc::new(WorkflowTestProvider),
        knowledge_provider: None,
        document_extractor: None,
        tool_provider: None,
        agent_provider: None,
        pause_store: None,
        model: "test-model".to_string(),
        temperature: 0.0,
    };
    let request = WorkflowRunRequest {
        workflow_uuid: None,
        workflow_yaml: Some(variable_assigner_append_workflow()),
        query: None,
        inputs: BTreeMap::from([("item".to_string(), json!("tea"))]),
        max_steps: 10,
    };

    let response = run_workflow(runtime, request).await.expect("workflow response");

    assert_eq!(response.status, WorkflowRunStatus::Succeeded);
    assert_eq!(response.answer.as_deref(), Some(r#"["tea"]"#));
    let assign = response.nodes.iter().find(|node| node.node_id == "assign").expect("assign node");
    assert_eq!(assign.outputs.get("favorites"), Some(&json!(["tea"])));
}

#[tokio::test]
async fn run_workflow_variable_assigner_applies_number_assignment() {
    let runtime = WorkflowRuntime {
        provider: Arc::new(WorkflowTestProvider),
        knowledge_provider: None,
        document_extractor: None,
        tool_provider: None,
        agent_provider: None,
        pause_store: None,
        model: "test-model".to_string(),
        temperature: 0.0,
    };
    let request = WorkflowRunRequest {
        workflow_uuid: None,
        workflow_yaml: Some(variable_assigner_number_workflow()),
        query: None,
        inputs: BTreeMap::new(),
        max_steps: 10,
    };

    let response = run_workflow(runtime, request).await.expect("workflow response");

    assert_eq!(response.status, WorkflowRunStatus::Succeeded);
    let assign = response.nodes.iter().find(|node| node.node_id == "assign").expect("assign node");
    assert_eq!(assign.outputs.get("count").and_then(Value::as_f64), Some(5.0));
}

#[tokio::test]
async fn run_workflow_variable_assigner_rejects_persistent_conversation_value() {
    let runtime = WorkflowRuntime {
        provider: Arc::new(WorkflowTestProvider),
        knowledge_provider: None,
        document_extractor: None,
        tool_provider: None,
        agent_provider: None,
        pause_store: None,
        model: "test-model".to_string(),
        temperature: 0.0,
    };
    let request = WorkflowRunRequest {
        workflow_uuid: None,
        workflow_yaml: Some(variable_assigner_persistent_workflow()),
        query: None,
        inputs: BTreeMap::new(),
        max_steps: 10,
    };

    let response = run_workflow(runtime, request).await.expect("workflow response");

    assert_eq!(response.status, WorkflowRunStatus::Failed);
    assert!(response.error.expect("error").contains("持久化会话变量未支持"));
}

fn variable_assigner_append_workflow() -> String {
    r#"
workflow:
  graph:
    nodes:
      - id: start
        data:
          title: Start
          type: start
          variables:
            - variable: item
      - id: assign
        data:
          title: Assign
          type: variable-assigner
          assignments:
            - variable: favorites
              operation: append
              value_selector:
                - start
                - item
      - id: answer
        data:
          title: Answer
          type: answer
          answer: "{{#assign.favorites#}}"
    edges:
      - source: start
        sourceHandle: source
        target: assign
      - source: assign
        sourceHandle: source
        target: answer
"#
    .to_string()
}

fn variable_assigner_number_workflow() -> String {
    r#"
workflow:
  graph:
    nodes:
      - id: start
        data:
          title: Start
          type: start
      - id: assign
        data:
          title: Assign
          type: variable-assigner
          assignments:
            - variable: count
              operation: set
              value: 2
            - variable: count
              operation: add
              value: 3
    edges:
      - source: start
        sourceHandle: source
        target: assign
"#
    .to_string()
}

fn variable_assigner_persistent_workflow() -> String {
    r#"
workflow:
  graph:
    nodes:
      - id: start
        data:
          title: Start
          type: start
      - id: assign
        data:
          title: Assign
          type: variable-assigner
          persistent: true
          assignments:
            - variable: favorites
              operation: set
              value: []
    edges:
      - source: start
        sourceHandle: source
        target: assign
"#
    .to_string()
}

#[tokio::test]
async fn run_workflow_tool_node_returns_provider_text() {
    let tool_provider = Arc::new(FakeWorkflowToolProvider { requests: Mutex::new(Vec::new()) });
    let runtime = WorkflowRuntime {
        provider: Arc::new(WorkflowTestProvider),
        knowledge_provider: None,
        document_extractor: None,
        tool_provider: Some(tool_provider.clone()),
        agent_provider: None,
        pause_store: None,
        model: "test-model".to_string(),
        temperature: 0.0,
    };
    let request = WorkflowRunRequest {
        workflow_uuid: None,
        workflow_yaml: Some(tool_echo_workflow("string", "echo")),
        query: None,
        inputs: BTreeMap::from([("q".to_string(), json!("hello"))]),
        max_steps: 10,
    };

    let response = run_workflow(runtime, request).await.expect("workflow response");

    assert_eq!(response.status, WorkflowRunStatus::Succeeded);
    assert_eq!(response.answer.as_deref(), Some("hello"));
    let tool = response.nodes.iter().find(|node| node.node_id == "tool").expect("tool node");
    assert_eq!(tool.outputs.get("text"), Some(&Value::String("hello".to_string())));
    assert_eq!(tool.outputs.get("json"), Some(&json!({"ok": true})));
    let requests = tool_provider.requests.lock().expect("requests lock");
    assert_eq!(requests[0].provider, "demo");
    assert_eq!(requests[0].tool_name, "echo");
    assert_eq!(requests[0].inputs.get("q"), Some(&Value::String("hello".to_string())));
}

#[tokio::test]
async fn run_workflow_tool_node_rejects_unauthorized_tool_without_secret_leak() {
    let runtime = WorkflowRuntime {
        provider: Arc::new(WorkflowTestProvider),
        knowledge_provider: None,
        document_extractor: None,
        tool_provider: Some(Arc::new(DeniedWorkflowToolProvider)),
        agent_provider: None,
        pause_store: None,
        model: "test-model".to_string(),
        temperature: 0.0,
    };
    let request = WorkflowRunRequest {
        workflow_uuid: None,
        workflow_yaml: Some(tool_denied_workflow()),
        query: None,
        inputs: BTreeMap::from([("q".to_string(), json!("hello"))]),
        max_steps: 10,
    };

    let response = run_workflow(runtime, request).await.expect("workflow response");

    assert_eq!(response.status, WorkflowRunStatus::Failed);
    let error = response.error.expect("error");
    assert!(error.contains("未授权"));
    assert!(!error.contains("secret-token"));
}

#[tokio::test]
async fn run_workflow_tool_node_rejects_input_type_mismatch() {
    let runtime = WorkflowRuntime {
        provider: Arc::new(WorkflowTestProvider),
        knowledge_provider: None,
        document_extractor: None,
        tool_provider: Some(Arc::new(FakeWorkflowToolProvider {
            requests: Mutex::new(Vec::new()),
        })),
        agent_provider: None,
        pause_store: None,
        model: "test-model".to_string(),
        temperature: 0.0,
    };
    let request = WorkflowRunRequest {
        workflow_uuid: None,
        workflow_yaml: Some(tool_echo_workflow("number", "echo")),
        query: None,
        inputs: BTreeMap::from([("q".to_string(), json!("hello"))]),
        max_steps: 10,
    };

    let response = run_workflow(runtime, request).await.expect("workflow response");

    assert_eq!(response.status, WorkflowRunStatus::Failed);
    assert!(response.error.expect("error").contains("类型不匹配"));
}

fn tool_echo_workflow(input_type: &str, tool_name: &str) -> String {
    format!(
        r#"
workflow:
  graph:
    nodes:
      - id: start
        data:
          title: Start
          type: start
          variables:
            - variable: q
      - id: tool
        data:
          title: Tool
          type: tool
          provider: demo
          tool_name: {tool_name}
          inputs:
            q:
              type: {input_type}
              value_selector:
                - start
                - q
      - id: answer
        data:
          title: Answer
          type: answer
          answer: "{{{{#tool.text#}}}}"
    edges:
      - source: start
        sourceHandle: source
        target: tool
      - source: tool
        sourceHandle: source
        target: answer
"#
    )
}

fn tool_denied_workflow() -> String {
    r#"
workflow:
  graph:
    nodes:
      - id: start
        data:
          title: Start
          type: start
          variables:
            - variable: q
      - id: tool
        data:
          title: Tool
          type: tool
          provider: demo
          tool_name: blocked
          credential:
            id: demo-credential
            token: secret-token
          inputs:
            q:
              type: string
              value_selector:
                - start
                - q
    edges:
      - source: start
        sourceHandle: source
        target: tool
"#
    .to_string()
}

#[tokio::test]
async fn run_workflow_agent_node_calls_tool_and_returns_answer() {
    let agent_provider = Arc::new(FakeWorkflowAgentProvider { requests: Mutex::new(Vec::new()) });
    let tool_provider = Arc::new(FakeWorkflowToolProvider { requests: Mutex::new(Vec::new()) });
    let runtime = WorkflowRuntime {
        provider: Arc::new(WorkflowTestProvider),
        knowledge_provider: None,
        document_extractor: None,
        tool_provider: Some(tool_provider.clone()),
        agent_provider: Some(agent_provider.clone()),
        pause_store: None,
        model: "test-model".to_string(),
        temperature: 0.0,
    };
    let request = WorkflowRunRequest {
        workflow_uuid: None,
        workflow_yaml: Some(agent_echo_workflow(3)),
        query: None,
        inputs: BTreeMap::from([("query".to_string(), json!("echo hello"))]),
        max_steps: 20,
    };

    let response = run_workflow(runtime, request).await.expect("workflow response");

    assert_eq!(response.status, WorkflowRunStatus::Succeeded);
    assert_eq!(response.answer.as_deref(), Some("hello"));
    let agent = response.nodes.iter().find(|node| node.node_id == "agent").expect("agent node");
    assert_eq!(agent.outputs.get("answer"), Some(&Value::String("hello".to_string())));
    assert_eq!(agent.outputs.get("iterations").and_then(Value::as_u64), Some(2));
    let agent_requests = agent_provider.requests.lock().expect("requests lock");
    assert_eq!(agent_requests[0].tools[0].tool_name, "echo");
    let tool_requests = tool_provider.requests.lock().expect("requests lock");
    assert_eq!(tool_requests[0].inputs.get("q"), Some(&Value::String("hello".to_string())));
}

#[tokio::test]
async fn run_workflow_agent_node_fails_when_max_iterations_exceeded() {
    let runtime = WorkflowRuntime {
        provider: Arc::new(WorkflowTestProvider),
        knowledge_provider: None,
        document_extractor: None,
        tool_provider: Some(Arc::new(FakeWorkflowToolProvider {
            requests: Mutex::new(Vec::new()),
        })),
        agent_provider: Some(Arc::new(FakeWorkflowAgentProvider {
            requests: Mutex::new(Vec::new()),
        })),
        pause_store: None,
        model: "test-model".to_string(),
        temperature: 0.0,
    };
    let request = WorkflowRunRequest {
        workflow_uuid: None,
        workflow_yaml: Some(agent_echo_workflow(1)),
        query: None,
        inputs: BTreeMap::from([("query".to_string(), json!("too-many"))]),
        max_steps: 20,
    };

    let response = run_workflow(runtime, request).await.expect("workflow response");

    assert_eq!(response.status, WorkflowRunStatus::Failed);
    assert!(response.error.expect("error").contains("超过最大迭代次数"));
}

#[tokio::test]
async fn run_workflow_agent_node_requires_tool_provider() {
    let runtime = WorkflowRuntime {
        provider: Arc::new(WorkflowTestProvider),
        knowledge_provider: None,
        document_extractor: None,
        tool_provider: None,
        agent_provider: Some(Arc::new(FakeWorkflowAgentProvider {
            requests: Mutex::new(Vec::new()),
        })),
        pause_store: None,
        model: "test-model".to_string(),
        temperature: 0.0,
    };
    let request = WorkflowRunRequest {
        workflow_uuid: None,
        workflow_yaml: Some(agent_echo_workflow(3)),
        query: None,
        inputs: BTreeMap::from([("query".to_string(), json!("echo hello"))]),
        max_steps: 20,
    };

    let response = run_workflow(runtime, request).await.expect("workflow response");

    assert_eq!(response.status, WorkflowRunStatus::Failed);
    assert!(response.error.expect("error").contains("workflow tool provider 未配置"));
}

fn agent_echo_workflow(max_iterations: u32) -> String {
    format!(
        r#"
workflow:
  graph:
    nodes:
      - id: start
        data:
          title: Start
          type: start
          variables:
            - variable: query
      - id: agent
        data:
          title: Agent
          type: agent
          strategy: function_calling
          max_iterations: {max_iterations}
          prompt_template:
            - role: user
              text: "{{{{#start.query#}}}}"
          tools:
            - provider: demo
              tool_name: echo
      - id: answer
        data:
          title: Answer
          type: answer
          answer: "{{{{#agent.answer#}}}}"
    edges:
      - source: start
        sourceHandle: source
        target: agent
      - source: agent
        sourceHandle: source
        target: answer
"#
    )
}

#[tokio::test]
async fn run_workflow_loop_node_runs_until_max_count() {
    let runtime = WorkflowRuntime {
        provider: Arc::new(WorkflowTestProvider),
        knowledge_provider: None,
        document_extractor: None,
        tool_provider: None,
        agent_provider: None,
        pause_store: None,
        model: "test-model".to_string(),
        temperature: 0.0,
    };
    let request = WorkflowRunRequest {
        workflow_uuid: None,
        workflow_yaml: Some(loop_workflow(None, 3)),
        query: None,
        inputs: BTreeMap::new(),
        max_steps: 20,
    };

    let response = run_workflow(runtime, request).await.expect("workflow response");

    assert_eq!(response.status, WorkflowRunStatus::Succeeded);
    assert_eq!(response.answer.as_deref(), Some("3/3"));
    let loop_node = response.nodes.iter().find(|node| node.node_id == "loop").expect("loop node");
    assert_eq!(loop_node.outputs.get("last_output"), Some(&json!(3)));
    assert_eq!(loop_node.outputs.get("iterations").and_then(Value::as_u64), Some(3));
}

#[tokio::test]
async fn run_workflow_loop_node_stops_on_condition() {
    let runtime = WorkflowRuntime {
        provider: Arc::new(WorkflowTestProvider),
        knowledge_provider: None,
        document_extractor: None,
        tool_provider: None,
        agent_provider: None,
        pause_store: None,
        model: "test-model".to_string(),
        temperature: 0.0,
    };
    let request = WorkflowRunRequest {
        workflow_uuid: None,
        workflow_yaml: Some(loop_workflow(Some(2), 5)),
        query: None,
        inputs: BTreeMap::new(),
        max_steps: 20,
    };

    let response = run_workflow(runtime, request).await.expect("workflow response");

    assert_eq!(response.status, WorkflowRunStatus::Succeeded);
    assert_eq!(response.answer.as_deref(), Some("2/2"));
    let loop_node = response.nodes.iter().find(|node| node.node_id == "loop").expect("loop node");
    assert_eq!(loop_node.outputs.get("last_output"), Some(&json!(2)));
    assert_eq!(loop_node.outputs.get("iterations").and_then(Value::as_u64), Some(2));
}

#[tokio::test]
async fn run_workflow_loop_node_fails_when_condition_never_stops() {
    let runtime = WorkflowRuntime {
        provider: Arc::new(WorkflowTestProvider),
        knowledge_provider: None,
        document_extractor: None,
        tool_provider: None,
        agent_provider: None,
        pause_store: None,
        model: "test-model".to_string(),
        temperature: 0.0,
    };
    let request = WorkflowRunRequest {
        workflow_uuid: None,
        workflow_yaml: Some(loop_workflow(Some(99), 2)),
        query: None,
        inputs: BTreeMap::new(),
        max_steps: 20,
    };

    let response = run_workflow(runtime, request).await.expect("workflow response");

    assert_eq!(response.status, WorkflowRunStatus::Failed);
    assert!(response.error.expect("error").contains("达到最大次数"));
}

fn loop_workflow(stop_at: Option<u32>, max_count: u32) -> String {
    let condition = stop_at
        .map(|value| {
            format!(
                r#"
          conditions:
            - variable_selector:
                - step
                - value
              comparison_operator: ">="
              value: {value}
"#
            )
        })
        .unwrap_or_default();
    format!(
        r#"
workflow:
  graph:
    nodes:
      - id: start
        data:
          title: Start
          type: start
      - id: loop
        data:
          title: Loop
          type: loop
          max_count: {max_count}
          output_selector:
            - step
            - value
{condition}          graph:
            nodes:
              - id: step
                data:
                  title: Step
                  type: code
                  code_language: python3
                  variables:
                    - variable: index
                      value_selector:
                        - loop
                        - index
                  code: |
                    def main(index):
                        return {{"value": index + 1}}
            edges: []
      - id: answer
        data:
          title: Answer
          type: answer
          answer: "{{{{#loop.last_output#}}}}/{{{{#loop.iterations#}}}}"
    edges:
      - source: start
        sourceHandle: source
        target: loop
      - source: loop
        sourceHandle: source
        target: answer
"#
    )
}

#[tokio::test]
async fn run_workflow_human_input_pauses_with_form() {
    let pause_store = Arc::new(FakeWorkflowPauseStore { states: Mutex::new(BTreeMap::new()) });
    let runtime = WorkflowRuntime {
        provider: Arc::new(WorkflowTestProvider),
        knowledge_provider: None,
        document_extractor: None,
        tool_provider: None,
        agent_provider: None,
        pause_store: Some(pause_store.clone()),
        model: "test-model".to_string(),
        temperature: 0.0,
    };
    let request = WorkflowRunRequest {
        workflow_uuid: None,
        workflow_yaml: Some(human_input_workflow()),
        query: None,
        inputs: BTreeMap::new(),
        max_steps: 20,
    };

    let response = run_workflow(runtime, request).await.expect("workflow response");

    assert_eq!(response.status, WorkflowRunStatus::Paused);
    let pause = response.pause.expect("pause");
    assert_eq!(pause.node_id, "review");
    assert_eq!(pause.actions[0].id, "approve");
    assert!(pause.form.get("fields").is_some());
    assert!(pause_store.states.lock().expect("states lock").contains_key(&response.run_id));
}

#[tokio::test]
async fn resume_workflow_human_input_activates_action_branch() {
    let pause_store = Arc::new(FakeWorkflowPauseStore { states: Mutex::new(BTreeMap::new()) });
    let runtime = WorkflowRuntime {
        provider: Arc::new(WorkflowTestProvider),
        knowledge_provider: None,
        document_extractor: None,
        tool_provider: None,
        agent_provider: None,
        pause_store: Some(pause_store.clone()),
        model: "test-model".to_string(),
        temperature: 0.0,
    };
    let request = WorkflowRunRequest {
        workflow_uuid: None,
        workflow_yaml: Some(human_input_workflow()),
        query: None,
        inputs: BTreeMap::new(),
        max_steps: 20,
    };
    let paused = run_workflow(runtime.clone(), request).await.expect("paused response");
    let pause = paused.pause.expect("pause");

    let response = resume_workflow(
        runtime,
        WorkflowResumeRequest {
            run_id: paused.run_id.clone(),
            form_token: pause.form_token,
            form_values: BTreeMap::from([("comment".to_string(), json!("ok"))]),
            action: Some("approve".to_string()),
        },
    )
    .await
    .expect("resume response");

    assert_eq!(response.status, WorkflowRunStatus::Succeeded);
    assert_eq!(response.answer.as_deref(), Some("approved ok"));
    assert!(!pause_store.states.lock().expect("states lock").contains_key(&paused.run_id));
}

#[tokio::test]
async fn resume_workflow_human_input_rejects_bad_token() {
    let pause_store = Arc::new(FakeWorkflowPauseStore { states: Mutex::new(BTreeMap::new()) });
    let runtime = WorkflowRuntime {
        provider: Arc::new(WorkflowTestProvider),
        knowledge_provider: None,
        document_extractor: None,
        tool_provider: None,
        agent_provider: None,
        pause_store: Some(pause_store),
        model: "test-model".to_string(),
        temperature: 0.0,
    };
    let request = WorkflowRunRequest {
        workflow_uuid: None,
        workflow_yaml: Some(human_input_workflow()),
        query: None,
        inputs: BTreeMap::new(),
        max_steps: 20,
    };
    let paused = run_workflow(runtime.clone(), request).await.expect("paused response");

    let error = resume_workflow(
        runtime,
        WorkflowResumeRequest {
            run_id: paused.run_id,
            form_token: "bad-token".to_string(),
            form_values: BTreeMap::from([("comment".to_string(), json!("ok"))]),
            action: Some("approve".to_string()),
        },
    )
    .await
    .expect_err("bad token");

    assert!(error.contains("form_token 不匹配"));
}

#[tokio::test]
async fn run_workflow_human_input_requires_pause_store() {
    let runtime = WorkflowRuntime {
        provider: Arc::new(WorkflowTestProvider),
        knowledge_provider: None,
        document_extractor: None,
        tool_provider: None,
        agent_provider: None,
        pause_store: None,
        model: "test-model".to_string(),
        temperature: 0.0,
    };
    let request = WorkflowRunRequest {
        workflow_uuid: None,
        workflow_yaml: Some(human_input_workflow()),
        query: None,
        inputs: BTreeMap::new(),
        max_steps: 20,
    };

    let response = run_workflow(runtime, request).await.expect("workflow response");

    assert_eq!(response.status, WorkflowRunStatus::Failed);
    assert!(response.error.expect("error").contains("pause store 未配置"));
}

fn human_input_workflow() -> String {
    r#"
workflow:
  graph:
    nodes:
      - id: start
        data:
          title: Start
          type: start
      - id: review
        data:
          title: Review
          type: human-input
          form:
            fields:
              - name: comment
                type: text
                required: true
          actions:
            - id: approve
              label: Approve
            - id: reject
              label: Reject
      - id: ok
        data:
          title: OK
          type: answer
          answer: "approved {{#review.comment#}}"
      - id: no
        data:
          title: No
          type: answer
          answer: "rejected"
    edges:
      - source: start
        sourceHandle: source
        target: review
      - source: review
        sourceHandle: approve
        target: ok
      - source: review
        sourceHandle: reject
        target: no
"#
    .to_string()
}

#[tokio::test]
async fn run_workflow_iteration_node_collects_subgraph_results() {
    let workflow_yaml = iteration_workflow("terminate", false);
    let runtime = WorkflowRuntime {
        provider: Arc::new(WorkflowTestProvider),
        knowledge_provider: None,
        document_extractor: None,
        tool_provider: None,
        agent_provider: None,
        pause_store: None,
        model: "test-model".to_string(),
        temperature: 0.0,
    };
    let request = WorkflowRunRequest {
        workflow_uuid: None,
        workflow_yaml: Some(workflow_yaml),
        query: None,
        inputs: BTreeMap::from([("nums".to_string(), json!([1, 2, 3]))]),
        max_steps: 20,
    };

    let response = run_workflow(runtime, request).await.expect("workflow response");

    assert_eq!(response.status, WorkflowRunStatus::Succeeded);
    assert_eq!(response.answer.as_deref(), Some("[2,4,6]"));
    let iter = response.nodes.iter().find(|node| node.node_id == "iter").expect("iteration node");
    assert_eq!(iter.outputs.get("result"), Some(&json!([2, 4, 6])));
}

#[tokio::test]
async fn run_workflow_iteration_node_returns_empty_result_for_empty_array() {
    let workflow_yaml = iteration_workflow("terminate", false);
    let runtime = WorkflowRuntime {
        provider: Arc::new(WorkflowTestProvider),
        knowledge_provider: None,
        document_extractor: None,
        tool_provider: None,
        agent_provider: None,
        pause_store: None,
        model: "test-model".to_string(),
        temperature: 0.0,
    };
    let request = WorkflowRunRequest {
        workflow_uuid: None,
        workflow_yaml: Some(workflow_yaml),
        query: None,
        inputs: BTreeMap::from([("nums".to_string(), json!([]))]),
        max_steps: 20,
    };

    let response = run_workflow(runtime, request).await.expect("workflow response");

    assert_eq!(response.status, WorkflowRunStatus::Succeeded);
    let iter = response.nodes.iter().find(|node| node.node_id == "iter").expect("iteration node");
    assert_eq!(iter.outputs.get("result"), Some(&json!([])));
}

#[tokio::test]
async fn run_workflow_iteration_node_continues_on_error_with_null() {
    let workflow_yaml = iteration_workflow("continue_on_error", true);
    let runtime = WorkflowRuntime {
        provider: Arc::new(WorkflowTestProvider),
        knowledge_provider: None,
        document_extractor: None,
        tool_provider: None,
        agent_provider: None,
        pause_store: None,
        model: "test-model".to_string(),
        temperature: 0.0,
    };
    let request = WorkflowRunRequest {
        workflow_uuid: None,
        workflow_yaml: Some(workflow_yaml),
        query: None,
        inputs: BTreeMap::from([("nums".to_string(), json!([1, 2, 3]))]),
        max_steps: 20,
    };

    let response = run_workflow(runtime, request).await.expect("workflow response");

    assert_eq!(response.status, WorkflowRunStatus::Succeeded);
    let iter = response.nodes.iter().find(|node| node.node_id == "iter").expect("iteration node");
    assert_eq!(iter.outputs.get("result"), Some(&json!([2, null, 6])));
}

#[tokio::test]
async fn run_workflow_iteration_node_rejects_parallel_mode() {
    let workflow_yaml = iteration_parallel_workflow();
    let runtime = WorkflowRuntime {
        provider: Arc::new(WorkflowTestProvider),
        knowledge_provider: None,
        document_extractor: None,
        tool_provider: None,
        agent_provider: None,
        pause_store: None,
        model: "test-model".to_string(),
        temperature: 0.0,
    };
    let request = WorkflowRunRequest {
        workflow_uuid: None,
        workflow_yaml: Some(workflow_yaml),
        query: None,
        inputs: BTreeMap::from([("nums".to_string(), json!([1]))]),
        max_steps: 20,
    };

    let response = run_workflow(runtime, request).await.expect("workflow response");

    assert_eq!(response.status, WorkflowRunStatus::Failed);
    assert!(response.error.expect("error").contains("并行模式"));
}

fn iteration_workflow(error_strategy: &str, fail_on_two: bool) -> String {
    let guard = if fail_on_two {
        r#"
                        if item == 2:
                            raise Exception("bad item")
"#
    } else {
        ""
    };
    format!(
        r#"
workflow:
  graph:
    nodes:
      - id: start
        data:
          title: Start
          type: start
          variables:
            - variable: nums
      - id: iter
        data:
          title: Iteration
          type: iteration
          input_selector:
            - start
            - nums
          output_selector:
            - double
            - value
          error_strategy: {error_strategy}
          graph:
            nodes:
              - id: double
                data:
                  title: Double
                  type: code
                  code_language: python3
                  variables:
                    - variable: item
                      value_selector:
                        - iter
                        - item
                  code: |
                    def main(item):
{guard}                        return {{"value": item * 2}}
            edges: []
      - id: answer
        data:
          title: Answer
          type: answer
          answer: "{{{{#iter.result#}}}}"
    edges:
      - source: start
        sourceHandle: source
        target: iter
      - source: iter
        sourceHandle: source
        target: answer
"#
    )
}

fn iteration_parallel_workflow() -> String {
    r#"
workflow:
  graph:
    nodes:
      - id: start
        data:
          title: Start
          type: start
          variables:
            - variable: nums
      - id: iter
        data:
          title: Iteration
          type: iteration
          parallel: true
          input_selector:
            - start
            - nums
          output_selector:
            - double
            - value
          graph:
            nodes:
              - id: double
                data:
                  title: Double
                  type: code
                  code_language: python3
                  code: |
                    def main():
                        return {"value": 1}
            edges: []
    edges:
      - source: start
        sourceHandle: source
        target: iter
"#
    .to_string()
}
