//! Real processor, local MCP and storage regression; only the model is scripted.

use crate::llm_stream::StreamEvent as LlmEvent;
use model_types::ToolCall;
use crate::processor_events::StreamEvent;
use session_port::ui_types::{ChatMessage, ChatRole};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};


pub type Request<H> = crate::runner_types::Request<<H as AcceptanceHost>::Approval,<H as AcceptanceHost>::Skills>;
pub struct RecordedRequest { pub messages: Vec<Value> }
/// Real runtime/permission execution and trusted configuration remain host-owned.
pub trait AcceptanceHost {
    type Approval;
    type Skills: Clone + Default;
    fn enqueue(events: Vec<LlmEvent>);
    fn run(request: Request<Self>) -> Vec<StreamEvent> where Self: Sized;
    fn requests(session: &str) -> Vec<RecordedRequest>;
    fn tool_ids() -> Vec<Vec<String>>;
    fn limit_iterations(runtime: &tokio::runtime::Runtime);
}

const ECHO: &str = "mcp__alignment__echo";
const BODY: &str = "ALIGNMENT_PRIVATE_WORKFLOW";

fn stream<H: AcceptanceHost>(calls: &[(&str, &str, Value)]) {
    let event = if calls.is_empty() {
        LlmEvent::Delta("complete".into())
    } else {
        LlmEvent::ToolCalls(
            calls
                .iter()
                .map(|(id, name, args)| ToolCall {
                    kind: Default::default(),
                    namespace: None,
                    id: (*id).into(),
                    name: (*name).into(),
                    arguments: args.to_string(),
                })
                .collect(),
        )
    };
    H::enqueue(vec![
        event,
        LlmEvent::Done { usage: Default::default(), finish_reason: Some("stop".into()) },
    ]);
}

fn skill(root: &Path, prefix: &str, body: &str) -> PathBuf {
    let path = root.join(prefix).join("workflow/SKILL.md");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(
        &path,
        format!("---\nname: workflow\ndescription: Local workflow metadata\n---\n{body}\n"),
    )
    .unwrap();
    path
}

pub fn workspace(root: &Path) -> PathBuf {
    let workspace = root.join("workspace");
    std::fs::create_dir_all(&workspace).unwrap();
    let script = root.join("alignment_mcp.py");
    std::fs::write(
        &script,
        r#"import json, pathlib, sys
marker = pathlib.Path(sys.argv[1])
tool = {"name":"echo", "description":"Echo a local alignment message", "inputSchema":{"type":"object","properties":{"message":{"type":"string"}},"required":["message"],"additionalProperties":False},"annotations":{"readOnlyHint":True,"destructiveHint":False,"idempotentHint":True}}
for line in sys.stdin:
    msg = json.loads(line)
    method, ident = msg.get("method"), msg.get("id")
    if ident is None:
        continue
    if method == "initialize":
        result = {"protocolVersion":msg.get("params",{}).get("protocolVersion","2025-06-18"),"capabilities":{"tools":{}},"serverInfo":{"name":"alignment","version":"1"}}
    elif method == "tools/list":
        result = {"tools":[tool]}
    elif method == "tools/call":
        params = msg.get("params",{})
        with marker.open("a") as output:
            output.write(json.dumps(params) + "\n")
        result = {"content":[{"type":"text","text":"echo:" + params.get("arguments",{}).get("message","")}],"isError":False}
    else:
        result = {}
    print(json.dumps({"jsonrpc":"2.0","id":ident,"result":result}), flush=True)
"#,
    )
    .unwrap();
    std::fs::write(
        workspace.join(".vwacprc.json"),
        json!({"mcpServers":[{"name":"alignment","command":"python3","args":[script, root.join("mcp_calls")]}]}).to_string(),
    )
    .unwrap();
    skill(&workspace, "skills", BODY);
    workspace
}

fn request<H: AcceptanceHost>(workspace: &Path) -> Request<H> {
    crate::runner_types::Request {
        stream: 1,
        session: format!("alignment-{}", uuid::Uuid::new_v4()),
        query: "Complete the local action".into(),
        root: Some(workspace.display().to_string()),
        model: Some("model-recovery/original".into()),
        options: json!({"allowed_tools":["skill","tool_search",ECHO],"full_access":true}),
        approval: None,
        channel_name: None,
        non_cli_approval_context: None,
        assistant_message_id: None,
        history: Vec::new(),
        persist_app_session_artifacts: false,
        app_session_scope: None,
        abort_token: None,
        session_tools: Default::default(),
        session_skills: Default::default(),
        reliability: agent_config::runtime::ReliabilityConfig {
            provider_retries: 0,
            provider_backoff_ms: 0,
            ..Default::default()
        },
    }
}

fn body_count(messages: &[Value], marker: &str) -> usize {
    messages
        .iter()
        .filter(|message| {
            message["role"] == "user"
                && message["content"].as_str().is_some_and(|body| body.contains(marker))
        })
        .count()
}

fn history(query: &str, events: &[StreamEvent]) -> Vec<ChatMessage> {
    let mut history = vec![ChatMessage {
        role: ChatRole::User,
        content: query.into(),
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
    }];
    for event in events {
        if let StreamEvent::Tool(tool) = event
            && matches!(tool.status, session_port::ToolCallStatus::Completed)
        {
            history.push(ChatMessage {
                role: ChatRole::Tool,
                content: format!(
                    "tool {}\n{}\n",
                    tool.name,
                    json!({"status":"completed","input":tool.payload["input"],"output":tool.payload["output"]})
                ),
                model: None,
                think_timing: Vec::new(),
                processing_timing: None,
            });
        }
    }
    history
}

pub fn codex_alignment_skill_search_mcp_compaction_and_new_turn_preserve_boundaries<H: AcceptanceHost>(workspace: &Path, root: &Path, _runtime: &tokio::runtime::Runtime) {
        let mut request = request::<H>(workspace);
        let session = request.session.clone();
        stream::<H>(&[("activate", "skill", json!({"skill":"workflow"}))]);
        stream::<H>(&[("discover", "tool_search", json!({"query":"echo"}))]);
        stream::<H>(&[("invoke", ECHO, json!({"message":"local-result"}))]);
        H::enqueue(vec![LlmEvent::Error(
            message_types::AssistantError::APIError {
                message: "maximum context length exceeded".into(),
                status_code: Some(400),
                is_retryable: false,
                response_headers: None,
                response_body: None,
                metadata: None,
            },
        )]);
        stream::<H>(&[]); // Real internal summarizer consumes this model response.
        stream::<H>(&[]);
        let events = H::run(request.clone());
        let requests = H::requests(&session);
        let ids = H::tool_ids();
        assert_eq!(requests.len(), 6, "{events:?}");
        assert_eq!(ids.len(), 6);
        assert!(!ids[0].contains(&ECHO.into()));
        assert!(!ids[1].contains(&ECHO.into()));
        assert!(ids[2].contains(&ECHO.into()));
        assert!(ids[4].is_empty(), "internal summary cannot call tools");
        assert!(ids[5].contains(&ECHO.into()));
        let catalog = requests[0].messages.iter().find(|m| m["role"] == "developer").unwrap();
        assert!(catalog["content"].as_str().unwrap().contains("workflow/SKILL.md"));
        assert!(!catalog["content"].as_str().unwrap().contains(BODY));
        assert_eq!(body_count(&requests[0].messages, BODY), 0);
        for index in [1, 2, 3, 5] {
            assert_eq!(body_count(&requests[index].messages, BODY), 1);
        }
        let search = requests[2].messages.iter().find(|m| m["tool_call_id"] == "discover").unwrap();
        let result: Value = serde_json::from_str(search["content"].as_str().unwrap()).unwrap();
        assert_eq!(result["tools"][0]["id"], ECHO);
        assert_eq!(result["tools"][0]["input_schema"]["required"], json!(["message"]));
        assert!(requests[3].messages.iter().any(|m| m["tool_call_id"] == "invoke"
            && m["content"].as_str().is_some_and(|s| s.contains("echo:local-result"))));
        assert!(events.iter().any(|event| matches!(
            event,
            StreamEvent::Compaction { phase: session_port::CompactionPhase::Completed }
        )));
        assert!(!events.iter().any(|event| matches!(event, StreamEvent::Error(_))), "{events:?}");
        let calls = std::fs::read_to_string(root.join("mcp_calls")).unwrap();
        assert_eq!(calls.lines().count(), 1, "search never calls the remote tool");
        assert_eq!(
            serde_json::from_str::<Value>(calls.trim()).unwrap()["arguments"]["message"],
            "local-result"
        );

        request.history = history(&request.query, &events);
        assert!(!request.history.iter().any(|m| m.content.contains(BODY)));
        request.query = "Continue without selecting another workflow".into();
        stream::<H>(&[]);
        H::run(request);
        let requests = H::requests(&session);
        assert_eq!(requests.len(), 1);
        assert_eq!(body_count(&requests[0].messages, BODY), 1);
        let ids = H::tool_ids();
        assert!(!ids[0].contains(&ECHO.into()), "new turn must rediscover tools");
}

pub fn codex_alignment_explicit_tools_search_disabled_and_disable_tools_keep_authorization<H: AcceptanceHost>(workspace: &Path, root: &Path, _runtime: &tokio::runtime::Runtime) {
        for (allowed, disabled, visible) in [
            (json!([ECHO]), false, true),
            (json!(["tool_search", ECHO]), false, false),
            (json!(["tool_search"]), false, false),
            (json!(["tool_search", ECHO]), true, false),
        ] {
            let mut request = request::<H>(workspace);
            request.options =
                json!({"allowed_tools":allowed,"disable_tools":disabled,"full_access":true});
            stream::<H>(&[]);
            let events = H::run(request);
            assert!(
                !events.iter().any(|event| matches!(event, StreamEvent::Error(_))),
                "{events:?}"
            );
            let ids = H::tool_ids();
            assert_eq!(ids.len(), 1);
            assert_eq!(ids[0].contains(&ECHO.into()), visible);
            if disabled {
                assert!(ids[0].is_empty());
            }
        }
        assert!(!root.join("mcp_calls").exists());
}

pub fn codex_alignment_strict_skills_and_last_round_do_not_expand_capabilities<H: AcceptanceHost>(workspace: &Path, root: &Path, runtime: &tokio::runtime::Runtime) {
        let blocked = workspace.join("skills/blocked/SKILL.md");
        std::fs::create_dir_all(blocked.parent().unwrap()).unwrap();
        std::fs::write(
            &blocked,
            "---\nname: blocked\ndescription: Hidden metadata\n---\nBLOCKED_PRIVATE_BODY",
        )
        .unwrap();
        H::limit_iterations(runtime);
        let mut request = request::<H>(workspace);
        request.options = json!({"allowed_tools":["skill","tool_search",ECHO],"full_access":true,"delegate_skill_policy":"strict","delegate_default_allowed_skills":["workflow"]});
        let session = request.session.clone();
        stream::<H>(&[("denied", "skill", json!({"skill":"blocked"}))]);
        stream::<H>(&[("discover", "tool_search", json!({"query":"echo"}))]);
        stream::<H>(&[]);
        H::run(request);
        let requests = H::requests(&session);
        assert_eq!(requests.len(), 3);
        for request in requests {
            assert_eq!(body_count(&request.messages, BODY), 1);
            assert!(!request.messages.iter().any(|m| m["role"] == "developer"
                && m["content"].as_str().is_some_and(|s| s.contains("blocked"))));
            assert_eq!(body_count(&request.messages, "BLOCKED_PRIVATE_BODY"), 0);
        }
        let ids = H::tool_ids();
        assert!(ids[2].is_empty(), "last response has no tool schema");
        assert!(!root.join("mcp_calls").exists());
}

pub fn codex_alignment_same_name_replacement_and_disabled_sources_cannot_replay_old_body<H: AcceptanceHost>(workspace: &Path, _root: &Path, _runtime: &tokio::runtime::Runtime) {
        let mut request = request::<H>(workspace);
        let session = request.session.clone();
        stream::<H>(&[("activate", "skill", json!({"skill":"workflow"}))]);
        stream::<H>(&[]);
        let events = H::run(request.clone());
        request.history = history(&request.query, &events);
        request.query = "Continue the existing task".into();
        H::requests(&session);
        H::tool_ids();
        let original = workspace.join("skills/workflow/SKILL.md");
        std::fs::write(original.parent().unwrap().join("SKILL.disabled"), "disabled").unwrap();
        let events = H::run(request.clone());
        assert!(H::requests(&session).is_empty());
        assert!(
            events.iter().any(
                |event| matches!(event, StreamEvent::Error(error) if error.contains("workflow"))
            )
        );
        std::fs::remove_dir_all(original.parent().unwrap()).unwrap();
        skill(workspace, ".agents/skills", "REPLACEMENT_PRIVATE_BODY");
        let events = H::run(request);
        assert!(H::requests(&session).is_empty());
        assert!(
            events.iter().any(
                |event| matches!(event, StreamEvent::Error(error) if error.contains("workflow"))
            )
        );

        let mut fresh = self::request::<H>(workspace);
        fresh.query = "Use workflow".into();
        let fresh_id = fresh.session.clone();
        stream::<H>(&[]);
        H::run(fresh);
        let requests = H::requests(&fresh_id);
        assert_eq!(requests.len(), 1);
        assert_eq!(body_count(&requests[0].messages, "REPLACEMENT_PRIVATE_BODY"), 1);
        assert_eq!(body_count(&requests[0].messages, BODY), 0);
}

pub fn codex_alignment_new_session_does_not_inherit_skill_or_tool_exposure<H: AcceptanceHost>(workspace: &Path, _root: &Path, _runtime: &tokio::runtime::Runtime) {
        let first = request::<H>(workspace);
        stream::<H>(&[("activate", "skill", json!({"skill":"workflow"}))]);
        stream::<H>(&[("discover", "tool_search", json!({"query":"echo"}))]);
        stream::<H>(&[]);
        H::run(first);
        H::tool_ids();
        let second = request::<H>(workspace);
        let session = second.session.clone();
        stream::<H>(&[]);
        H::run(second);
        let requests = H::requests(&session);
        assert_eq!(requests.len(), 1);
        assert_eq!(body_count(&requests[0].messages, BODY), 0);
        assert!(!H::tool_ids()[0].contains(&ECHO.into()));
}
