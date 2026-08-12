//! 将桌面代理请求转换为网关流式消息。
//! 本模块隔离传输事件解析，让应用状态只接收明确的 Message。

use super::message;
use super::{AgentRequest, Message};
use vw_gateway_client::vw_api_types;

const GATEWAY_STREAM_INITIAL_RETRY_DELAY: std::time::Duration =
    std::time::Duration::from_millis(500);
const GATEWAY_STREAM_MAX_RETRY_DELAY: std::time::Duration =
    std::time::Duration::from_secs(3 * 60 * 60);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GatewayStreamStopReason {
    Backpressure,
    Cancelled,
    PostToolRoundHandoff,
}

#[derive(Debug, PartialEq, Eq)]
enum GatewayStreamNext {
    Terminal,
    Retry(std::time::Duration),
    Cancelled,
    PostToolRoundHandoff,
    Failed(String),
}

#[derive(Debug)]
struct GatewayReconnectState {
    request_id: String,
    last_event_id: Option<u64>,
    next_delay: std::time::Duration,
}

impl GatewayReconnectState {
    fn new(request_id: String) -> Self {
        Self { request_id, last_event_id: None, next_delay: GATEWAY_STREAM_INITIAL_RETRY_DELAY }
    }

    fn resume(&self) -> vw_gateway_client::GatewayStreamResume {
        vw_gateway_client::GatewayStreamResume {
            request_id: self.request_id.clone(),
            last_event_id: self.last_event_id,
        }
    }

    fn finish_attempt(
        &mut self,
        result: Result<vw_gateway_client::GatewayStreamAttemptResult, String>,
        saw_new_event: bool,
        last_forwarded_event_id: Option<u64>,
        stop_reason: Option<GatewayStreamStopReason>,
    ) -> GatewayStreamNext {
        if let Some(event_id) = last_forwarded_event_id {
            self.last_event_id =
                Some(self.last_event_id.map_or(event_id, |current| current.max(event_id)));
        }
        if saw_new_event {
            self.next_delay = GATEWAY_STREAM_INITIAL_RETRY_DELAY;
        }

        match stop_reason {
            Some(GatewayStreamStopReason::Cancelled) => return GatewayStreamNext::Cancelled,
            Some(GatewayStreamStopReason::PostToolRoundHandoff) => {
                return GatewayStreamNext::PostToolRoundHandoff;
            }
            Some(GatewayStreamStopReason::Backpressure) => {
                return GatewayStreamNext::Retry(self.take_retry_delay());
            }
            None => {}
        }

        match result {
            Ok(vw_gateway_client::GatewayStreamAttemptResult::Terminated) => {
                GatewayStreamNext::Terminal
            }
            Ok(vw_gateway_client::GatewayStreamAttemptResult::CallerStopped) => {
                GatewayStreamNext::Cancelled
            }
            Ok(vw_gateway_client::GatewayStreamAttemptResult::Incomplete) => {
                GatewayStreamNext::Retry(self.take_retry_delay())
            }
            Err(error) if gateway_stream_error_is_retryable(&error) => {
                GatewayStreamNext::Retry(self.take_retry_delay())
            }
            Err(error) => GatewayStreamNext::Failed(error),
        }
    }

    fn take_retry_delay(&mut self) -> std::time::Duration {
        let delay = self.next_delay;
        self.next_delay = self.next_delay.saturating_mul(2).min(GATEWAY_STREAM_MAX_RETRY_DELAY);
        delay
    }
}

fn gateway_stream_error_is_retryable(error: &str) -> bool {
    if error.starts_with("请求内容过大") {
        return false;
    }

    let lower = error.to_ascii_lowercase();
    for prefix in ["gateway stream failed: ", "gateway request failed: "] {
        let Some(status_text) = lower.strip_prefix(prefix) else {
            continue;
        };
        let Some(status) = status_text.get(..3).and_then(|value| value.parse::<u16>().ok()) else {
            continue;
        };
        return status == 408 || status == 429 || status >= 500;
    }

    true
}

fn desktop_stream_request_id(req: &AgentRequest) -> String {
    use std::hash::{Hash, Hasher};
    use std::sync::OnceLock;

    static CLIENT_NONCE: OnceLock<u128> = OnceLock::new();

    let nonce = CLIENT_NONCE.get_or_init(rand::random);
    let mut session_hasher = std::collections::hash_map::DefaultHasher::new();
    req.session.hash(&mut session_hasher);
    format!("desktop-{nonce:032x}-{}-{:016x}", req.id, session_hasher.finish())
}

async fn gateway_stream_retry_sleep(duration: std::time::Duration) {
    #[cfg(not(target_arch = "wasm32"))]
    tokio::time::sleep(duration).await;

    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::{JsCast, JsValue, closure::Closure};

        let millis = duration.as_millis().min(i32::MAX as u128) as i32;
        let Some(window) = web_sys::window() else {
            return;
        };
        let promise = js_sys::Promise::new(&mut |resolve, _reject| {
            let Ok(resolve) = resolve.dyn_into::<js_sys::Function>() else {
                return;
            };
            let callback = Closure::once(move || {
                let _ = resolve.call0(&JsValue::NULL);
            });
            let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
                callback.as_ref().unchecked_ref(),
                millis,
            );
            callback.forget();
        });
        let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
    }
}

/// 模块内可见函数，执行 agent_stream 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn agent_stream(req: &AgentRequest) -> AgentBoxStream<Message> {
    gateway_agent_stream(req.clone())
}

#[cfg(not(target_arch = "wasm32"))]
type AgentBoxStream<T> = iced::futures::stream::BoxStream<'static, T>;
#[cfg(target_arch = "wasm32")]
type AgentBoxStream<T> = iced::futures::stream::LocalBoxStream<'static, T>;

const WORKFLOW_HISTORY_MAX_CHARS: usize = 12_000;

#[allow(dead_code)]
fn workflow_role_label(role: crate::app::models::ChatRole) -> &'static str {
    match role {
        crate::app::models::ChatRole::User => "User",
        crate::app::models::ChatRole::Assistant => "Assistant",
        crate::app::models::ChatRole::System => "System",
        crate::app::models::ChatRole::Tool => "Tool",
    }
}

#[allow(dead_code)]
fn workflow_history_context(history: &[crate::app::models::ChatMessage]) -> String {
    let mut lines = history
        .iter()
        .filter(|message| !message.content.trim().is_empty())
        .rev()
        .map(|message| format!("{}: {}", workflow_role_label(message.role), message.content.trim()))
        .collect::<Vec<_>>();
    lines.reverse();

    let mut context = lines.join("\n\n");
    if context.len() > WORKFLOW_HISTORY_MAX_CHARS {
        let trim_start = context.len().saturating_sub(WORKFLOW_HISTORY_MAX_CHARS);
        context = format!("...[truncated]\n{}", &context[trim_start..]);
    }
    context
}

const TEMPORARY_WORKFLOW_MAX_TASKS: usize = 8;
const TEMPORARY_WORKFLOW_TITLE_MAX_CHARS: usize = 48;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TemporaryWorkflowTopology {
    Serial,
    Parallel,
}

impl TemporaryWorkflowTopology {
    fn key(self) -> &'static str {
        match self {
            Self::Serial => "dynamic",
            Self::Parallel => "dynamic-parallel",
        }
    }

    fn description(self) -> &'static str {
        match self {
            Self::Serial => "demand-driven workflow",
            Self::Parallel => "demand-driven parallel workflow",
        }
    }
}

#[derive(Debug, Clone)]
struct TemporaryWorkflowStep {
    id: String,
    title: String,
    system_prompt: String,
    user_prompt: String,
    column: i64,
    lane: i64,
}

#[derive(Debug, Clone)]
struct TemporaryWorkflowEdge {
    source: String,
    target: String,
}

#[derive(Debug, Clone)]
struct TemporaryWorkflowPlan {
    steps: Vec<TemporaryWorkflowStep>,
    edges: Vec<TemporaryWorkflowEdge>,
    final_node_id: String,
    topology: TemporaryWorkflowTopology,
}

fn contains_any(value: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| value.contains(needle))
}

fn strip_temporary_workflow_order_prefix(value: &str) -> &str {
    let value = value.trim();
    let value = value
        .strip_prefix("- ")
        .or_else(|| value.strip_prefix("* "))
        .or_else(|| value.strip_prefix("• "))
        .or_else(|| value.strip_prefix("· "))
        .unwrap_or(value)
        .trim_start();

    let digit_end = value
        .char_indices()
        .take_while(|(_, ch)| ch.is_ascii_digit())
        .map(|(idx, ch)| idx + ch.len_utf8())
        .last()
        .unwrap_or(0);
    if digit_end > 0 {
        let rest = value[digit_end..].trim_start();
        if let Some(ch) =
            rest.chars().next().filter(|ch| matches!(ch, '.' | '、' | ')' | '）' | ':' | '：'))
        {
            return rest[ch.len_utf8()..].trim_start();
        }
    }

    for prefix in ["一、", "二、", "三、", "四、", "五、", "六、", "七、", "八、"] {
        if let Some(rest) = value.strip_prefix(prefix) {
            return rest.trim_start();
        }
    }

    value
}

fn strip_temporary_workflow_request_prefix(mut value: &str) -> &str {
    loop {
        let trimmed = value.trim_start();
        let mut next = trimmed;
        for prefix in [
            "请帮我",
            "请你",
            "帮我",
            "麻烦",
            "请",
            "需要",
            "我要",
            "我想",
            "先",
            "然后",
            "接着",
            "随后",
            "再",
            "最后",
            "同时",
            "分别",
            "并且",
            "以及",
        ] {
            if let Some(rest) = trimmed.strip_prefix(prefix) {
                next = rest.trim_start();
                break;
            }
        }
        if next == value || next == trimmed {
            return trimmed;
        }
        value = next;
    }
}

fn normalize_temporary_workflow_task_title(value: &str) -> Option<String> {
    let value = strip_temporary_workflow_order_prefix(value);
    let value = strip_temporary_workflow_request_prefix(value);
    let value = value
        .trim_matches(|ch: char| {
            ch.is_whitespace()
                || matches!(
                    ch,
                    ',' | '，' | '.' | '。' | ';' | '；' | ':' | '：' | '!' | '！' | '?' | '？'
                )
        })
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    Some(compact_temporary_workflow_task_title(value))
}

fn compact_temporary_workflow_task_title(value: &str) -> String {
    let mut chars = value.chars();
    let head = chars.by_ref().take(TEMPORARY_WORKFLOW_TITLE_MAX_CHARS).collect::<String>();
    if chars.next().is_some() { format!("{head}...") } else { head }
}

fn split_temporary_workflow_segment(value: &str) -> Vec<String> {
    let mut normalized = strip_temporary_workflow_order_prefix(value).trim().to_string();
    for connector in [
        "，然后",
        "，接着",
        "，随后",
        "，再",
        "，最后",
        ", then",
        ", next",
        ", finally",
        "然后",
        "接着",
        "随后",
        "最后",
        "并且",
        "以及",
        "同时",
    ] {
        normalized = normalized.replace(connector, "\n");
    }
    normalized.split('\n').filter_map(normalize_temporary_workflow_task_title).collect()
}

fn temporary_workflow_task_titles(query: &str) -> Vec<String> {
    let normalized = query.replace("\r\n", "\n").replace('\r', "\n");
    let mut titles = Vec::<String>::new();
    for chunk in normalized.split(['\n', '；', ';', '。']) {
        for segment in split_temporary_workflow_segment(chunk) {
            let Some(title) = normalize_temporary_workflow_task_title(&segment) else {
                continue;
            };
            if !titles.iter().any(|existing| existing == &title) {
                titles.push(title);
            }
        }
    }

    if titles.is_empty() {
        if let Some(title) = normalize_temporary_workflow_task_title(query) {
            titles.push(title);
        }
    }
    if titles.is_empty() {
        titles.push("完成用户需求".to_string());
    }

    if titles.len() > TEMPORARY_WORKFLOW_MAX_TASKS {
        let mut limited = titles
            .iter()
            .take(TEMPORARY_WORKFLOW_MAX_TASKS.saturating_sub(1))
            .cloned()
            .collect::<Vec<_>>();
        let rest = titles
            .iter()
            .skip(TEMPORARY_WORKFLOW_MAX_TASKS.saturating_sub(1))
            .cloned()
            .collect::<Vec<_>>()
            .join("；");
        limited.push(compact_temporary_workflow_task_title(&format!("完成剩余任务：{rest}")));
        return limited;
    }

    titles
}

fn temporary_workflow_topology(query: &str, task_count: usize) -> TemporaryWorkflowTopology {
    if task_count < 2 {
        return TemporaryWorkflowTopology::Serial;
    }

    let normalized = query.to_lowercase();
    let serial_requested = contains_any(
        &normalized,
        &["先", "然后", "接着", "随后", "再", "最后", "依次", "按顺序", "串行", "then", "next"],
    );
    let parallel_requested = contains_any(
        &normalized,
        &["并行", "同时", "分别", "各自", "独立", "parallel", "in parallel"],
    );

    if parallel_requested && !serial_requested {
        TemporaryWorkflowTopology::Parallel
    } else {
        TemporaryWorkflowTopology::Serial
    }
}

fn temporary_workflow_task_system_prompt(title: &str, is_final: bool) -> String {
    let mut prompt = format!(
        "You are a task node in a demand-driven Dify workflow. Your node title is \"{title}\". Complete exactly the task described by this title, using the user request and available workflow context."
    );
    if is_final {
        prompt.push_str(
            " This is the final task node, so return the answer the user should receive.",
        );
    } else {
        prompt.push_str(" Return concise output that the next task node can use.");
    }
    prompt
}

fn first_workflow_user_prompt(task_title: &str, is_final: bool) -> String {
    let mut prompt = format!(
        "Conversation context:\n{{{{#start.conversation#}}}}\n\nUser request:\n{{{{#start.query#}}}}\n\nCurrent task title:\n{task_title}\n\nComplete this task according to the user request."
    );
    if is_final {
        prompt.push_str("\nReturn the final user-facing answer.");
    } else {
        prompt.push_str("\nReturn only the useful result for downstream task nodes.");
    }
    prompt
}

fn next_workflow_user_prompt(
    previous_node_id: &str,
    previous_title: &str,
    task_title: &str,
    is_final: bool,
) -> String {
    let mut prompt = format!(
        "Conversation context:\n{{{{#start.conversation#}}}}\n\nUser request:\n{{{{#start.query#}}}}\n\nPrevious task title:\n{previous_title}\n\nPrevious task output:\n{{{{#{previous_node_id}.answer#}}}}\n\nCurrent task title:\n{task_title}\n\nComplete this task using the previous task output when relevant."
    );
    if is_final {
        prompt.push_str("\nReturn the final user-facing answer.");
    } else {
        prompt.push_str("\nReturn only the useful result for downstream task nodes.");
    }
    prompt
}

fn final_parallel_workflow_user_prompt(
    steps: &[TemporaryWorkflowStep],
    task_title: &str,
) -> String {
    let mut prompt =
        "Conversation context:\n{{#start.conversation#}}\n\nUser request:\n{{#start.query#}}\n\nCompleted task outputs:\n".to_string();
    for step in steps {
        prompt.push_str(&format!(
            "\nTask title: {}\nTask output:\n{{{{#{}.answer#}}}}\n",
            step.title, step.id
        ));
    }
    prompt.push_str(&format!(
        "\nCurrent task title:\n{task_title}\n\nSynthesize the task outputs and return the final user-facing answer."
    ));
    prompt
}

fn workflow_step(
    id: String,
    title: String,
    user_prompt: String,
    column: i64,
    lane: i64,
    is_final: bool,
) -> TemporaryWorkflowStep {
    let system_prompt = temporary_workflow_task_system_prompt(&title, is_final);
    TemporaryWorkflowStep { id, title, system_prompt, user_prompt, column, lane }
}

fn workflow_edge(source: impl Into<String>, target: impl Into<String>) -> TemporaryWorkflowEdge {
    TemporaryWorkflowEdge { source: source.into(), target: target.into() }
}

fn temporary_workflow_task_id(index: usize) -> String {
    format!("task_{}", index + 1)
}

fn temporary_workflow_parallel_summary_title() -> String {
    "汇总并交付最终结果".to_string()
}

fn temporary_workflow_plan(query: &str) -> TemporaryWorkflowPlan {
    let task_titles = temporary_workflow_task_titles(query);
    let task_count = task_titles.len();
    let topology = temporary_workflow_topology(query, task_count);
    let mut steps = Vec::<TemporaryWorkflowStep>::new();
    let mut edges = Vec::<TemporaryWorkflowEdge>::new();

    match topology {
        TemporaryWorkflowTopology::Serial => {
            for (index, title) in task_titles.into_iter().enumerate() {
                let id = temporary_workflow_task_id(index);
                let is_final = index + 1 == task_count;
                let user_prompt = if let Some(previous) = steps.last() {
                    next_workflow_user_prompt(&previous.id, &previous.title, &title, is_final)
                } else {
                    first_workflow_user_prompt(&title, is_final)
                };
                if let Some(previous) = steps.last() {
                    edges.push(workflow_edge(previous.id.clone(), id.clone()));
                } else {
                    edges.push(workflow_edge("start", id.clone()));
                }
                steps.push(workflow_step(id, title, user_prompt, index as i64 + 1, 0, is_final));
            }
        }
        TemporaryWorkflowTopology::Parallel => {
            let task_count = task_titles.len();
            let lane_offset = (task_count.saturating_sub(1) as i64) / 2;
            for (index, title) in task_titles.into_iter().enumerate() {
                let id = temporary_workflow_task_id(index);
                let lane = index as i64 - lane_offset;
                let user_prompt = first_workflow_user_prompt(&title, false);
                edges.push(workflow_edge("start", id.clone()));
                steps.push(workflow_step(id, title, user_prompt, 1, lane, false));
            }

            let final_id = temporary_workflow_task_id(steps.len());
            let final_title = temporary_workflow_parallel_summary_title();
            let final_prompt = final_parallel_workflow_user_prompt(&steps, &final_title);
            for step in &steps {
                edges.push(workflow_edge(step.id.clone(), final_id.clone()));
            }
            steps.push(workflow_step(final_id, final_title, final_prompt, 2, 0, true));
        }
    }

    let final_node_id =
        steps.last().map(|step| step.id.clone()).unwrap_or_else(|| "task_1".to_string());

    TemporaryWorkflowPlan { steps, edges, final_node_id, topology }
}

fn temporary_workflow_query_fingerprint(query: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    query.trim().hash(&mut hasher);
    format!("{:08x}", hasher.finish() as u32)
}

fn temporary_workflow_node_position(column: i64, lane: i64) -> (i64, i64) {
    let x = -280 + (column * 380);
    let y = 150 + (lane * 180);
    (x, y)
}

fn temporary_workflow_node(
    id: &str,
    data: serde_json::Value,
    column: i64,
    lane: i64,
) -> serde_json::Value {
    use serde_json::json;

    let node_type = data.get("type").and_then(serde_json::Value::as_str).unwrap_or("custom");
    let height = match node_type {
        "start" => 116,
        "answer" => 98,
        "llm" => 126,
        "template" | "template-transform" => 118,
        _ => 100,
    };
    let (x, y) = temporary_workflow_node_position(column, lane);

    json!({
        "id": id,
        "data": data,
        "height": height,
        "position": {
            "x": x,
            "y": y
        },
        "positionAbsolute": {
            "x": x,
            "y": y
        },
        "selected": false,
        "sourcePosition": "right",
        "targetPosition": "left",
        "type": "custom",
        "width": 244,
        "zIndex": 0
    })
}

fn temporary_workflow_edge(
    source: &str,
    source_type: &str,
    target: &str,
    target_type: &str,
) -> serde_json::Value {
    use serde_json::json;

    json!({
        "id": format!("{source}-source-{target}-target"),
        "source": source,
        "sourceHandle": "source",
        "target": target,
        "targetHandle": "target",
        "type": "custom",
        "selected": false,
        "zIndex": 0,
        "data": {
            "isInLoop": false,
            "isInIteration": false,
            "sourceType": source_type,
            "targetType": target_type
        }
    })
}

fn temporary_workflow_start_data() -> serde_json::Value {
    use serde_json::json;

    json!({
        "title": "接收用户需求",
        "type": "start",
        "variables": [
            {
                "label": "User query",
                "type": "paragraph",
                "variable": "query"
            },
            {
                "label": "Conversation context",
                "type": "paragraph",
                "variable": "conversation"
            }
        ]
    })
}

fn temporary_workflow_llm_data(
    step: &TemporaryWorkflowStep,
    model: Option<&str>,
) -> serde_json::Value {
    use serde_json::json;

    let mut data = json!({
        "title": step.title.as_str(),
        "type": "llm",
        "prompt_template": [
            {
                "role": "system",
                "text": step.system_prompt.as_str()
            },
            {
                "role": "user",
                "text": step.user_prompt.as_str()
            }
        ]
    });
    if let Some(model) = model.map(str::trim).filter(|value| !value.is_empty()) {
        data["model"] = json!({
            "name": model,
            "mode": "chat",
            "provider": "vibewindow"
        });
    }
    data
}

fn temporary_workflow_answer_data(final_node_id: &str) -> serde_json::Value {
    use serde_json::json;

    json!({
        "title": "交付最终结果",
        "type": "answer",
        "answer": format!("{{{{#{final_node_id}.answer#}}}}")
    })
}

fn temporary_workflow_yaml(query: &str, model: Option<&str>) -> Result<String, String> {
    use serde_json::json;
    use std::collections::BTreeMap;

    let plan = temporary_workflow_plan(query);
    let mut node_types = BTreeMap::<String, &'static str>::new();
    node_types.insert("start".to_string(), "start");
    node_types.insert("answer".to_string(), "answer");
    for step in &plan.steps {
        node_types.insert(step.id.clone(), "llm");
    }

    let mut nodes = Vec::with_capacity(plan.steps.len() + 2);
    let mut edges = Vec::with_capacity(plan.edges.len() + 1);

    nodes.push(temporary_workflow_node("start", temporary_workflow_start_data(), 0, 0));

    for step in &plan.steps {
        nodes.push(temporary_workflow_node(
            &step.id,
            temporary_workflow_llm_data(step, model),
            step.column,
            step.lane,
        ));
    }

    nodes.push(temporary_workflow_node(
        "answer",
        temporary_workflow_answer_data(&plan.final_node_id),
        plan.steps.iter().map(|step| step.column).max().unwrap_or(0) + 1,
        0,
    ));

    for edge in &plan.edges {
        edges.push(temporary_workflow_edge(
            &edge.source,
            node_types.get(&edge.source).copied().unwrap_or("custom"),
            &edge.target,
            node_types.get(&edge.target).copied().unwrap_or("custom"),
        ));
    }
    edges.push(temporary_workflow_edge(
        &plan.final_node_id,
        node_types.get(&plan.final_node_id).copied().unwrap_or("llm"),
        "answer",
        "answer",
    ));

    let workflow = json!({
        "app": {
            "description": format!(
                "Temporary {} workflow generated by VibeWindow chat.",
                plan.topology.description()
            ),
            "mode": "advanced-chat",
            "name": format!(
                "vibewindow-temp-{}-{}",
                plan.topology.key(),
                temporary_workflow_query_fingerprint(query)
            ),
            "use_icon_as_answer_icon": false
        },
        "kind": "app",
        "version": "0.5.0",
        "workflow": {
            "conversation_variables": [],
            "environment_variables": [],
            "graph": {
                "nodes": nodes,
                "edges": edges,
                "viewport": {
                    "x": 0,
                    "y": 0,
                    "zoom": 0.8
                }
            }
        }
    });

    serde_yaml::to_string(&workflow).map_err(|error| format!("生成临时工作流失败: {error}"))
}

#[allow(dead_code)]
fn workflow_mode_yaml(query: &str, model: Option<&str>) -> Result<String, String> {
    temporary_workflow_yaml(query, model)
}

const TASK_PLAN_RUN_STREAM_PATH: &str = "/v1/task-plan-run/stream";
#[allow(dead_code)]
const WORKFLOW_CHAT_MESSAGES_PATH: &str = "/v1/workflow/applications/chat-messages";
#[allow(dead_code)]
const WORKFLOW_STREAM_CONNECT_TIMEOUT_SECS: u64 = 30;
#[allow(dead_code)]
const WORKFLOW_STREAM_REQUEST_TIMEOUT_SECS: u64 = 60 * 60 * 3;
const WORKFLOW_NODE_PREVIEW_MAX_CHARS: usize = 12_000;
#[allow(dead_code)]
const WORKFLOW_NODE_STREAM_FLUSH_CHARS: usize = 360;
#[allow(dead_code)]
const WORKFLOW_NODE_STREAM_FLUSH_MS: u128 = 220;
#[allow(dead_code)]
const WORKFLOW_NODE_INLINE_YAML_MAX_CHARS: usize = 16_000;

#[allow(dead_code)]
enum WorkflowChatMessagesEvent {
    Delta(String),
    Done { usage: Option<serde_json::Value>, message_id: Option<String> },
    Error(String),
    NodeStarted(WorkflowNodeStreamEvent),
    NodeDelta(WorkflowNodeDeltaStreamEvent),
    NodeFinished(WorkflowNodeStreamEvent),
    Other,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct WorkflowNodeStreamEvent {
    node_id: String,
    node_type: String,
    title: String,
    index: u32,
    status: Option<String>,
    elapsed_time: Option<f64>,
    error: Option<String>,
    outputs: Option<serde_json::Value>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct WorkflowNodeDeltaStreamEvent {
    node_id: String,
    node_type: String,
    title: String,
    index: u32,
    text: String,
    replace: bool,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct WorkflowNodeOutputBuffer {
    text: String,
    pending_chars: usize,
    last_flush: std::time::Instant,
    truncated: bool,
}

impl WorkflowNodeOutputBuffer {
    #[allow(dead_code)]
    fn new() -> Self {
        Self {
            text: String::new(),
            pending_chars: 0,
            last_flush: std::time::Instant::now(),
            truncated: false,
        }
    }

    #[allow(dead_code)]
    fn push(&mut self, delta: &str, replace: bool) {
        if replace {
            self.text.clear();
            self.truncated = false;
        }
        self.text.push_str(delta);
        self.pending_chars = self.pending_chars.saturating_add(delta.chars().count());
        self.truncated |= compact_workflow_text_in_place(&mut self.text);
    }

    #[allow(dead_code)]
    fn should_flush(&mut self, force: bool) -> bool {
        if force
            || self.pending_chars >= WORKFLOW_NODE_STREAM_FLUSH_CHARS
            || self.last_flush.elapsed().as_millis() >= WORKFLOW_NODE_STREAM_FLUSH_MS
        {
            self.pending_chars = 0;
            self.last_flush = std::time::Instant::now();
            return true;
        }
        false
    }
}

#[allow(dead_code)]
fn parse_workflow_node_event(payload: &serde_json::Value) -> WorkflowNodeStreamEvent {
    let data = payload.get("data").unwrap_or(payload);
    WorkflowNodeStreamEvent {
        node_id: data
            .get("node_id")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_string(),
        node_type: data
            .get("node_type")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_string(),
        title: data
            .get("title")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_string(),
        index: data.get("index").and_then(serde_json::Value::as_u64).unwrap_or_default() as u32,
        status: data.get("status").and_then(serde_json::Value::as_str).map(ToOwned::to_owned),
        elapsed_time: data.get("elapsed_time").and_then(serde_json::Value::as_f64),
        error: data.get("error").and_then(serde_json::Value::as_str).map(ToOwned::to_owned),
        outputs: data.get("outputs").cloned(),
    }
}

#[allow(dead_code)]
fn parse_workflow_node_delta_event(payload: &serde_json::Value) -> WorkflowNodeDeltaStreamEvent {
    let data = payload.get("data").unwrap_or(payload);
    WorkflowNodeDeltaStreamEvent {
        node_id: data
            .get("node_id")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_string(),
        node_type: data
            .get("node_type")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_string(),
        title: data
            .get("title")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_string(),
        index: data.get("index").and_then(serde_json::Value::as_u64).unwrap_or_default() as u32,
        text: data
            .get("text")
            .or_else(|| data.get("answer"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_string(),
        replace: data.get("replace").and_then(serde_json::Value::as_bool).unwrap_or(false),
    }
}

#[allow(dead_code)]
fn workflow_node_key(index: u32, node_id: &str) -> String {
    format!("{index}-{node_id}")
}

#[allow(dead_code)]
fn workflow_node_stream_event_key(event: &WorkflowNodeStreamEvent) -> String {
    workflow_node_key(event.index, &event.node_id)
}

#[allow(dead_code)]
fn workflow_node_delta_event_key(event: &WorkflowNodeDeltaStreamEvent) -> String {
    workflow_node_key(event.index, &event.node_id)
}

#[allow(dead_code)]
fn workflow_node_stream_event_from_delta(
    event: &WorkflowNodeDeltaStreamEvent,
    output: &WorkflowNodeOutputBuffer,
) -> WorkflowNodeStreamEvent {
    WorkflowNodeStreamEvent {
        node_id: event.node_id.clone(),
        node_type: event.node_type.clone(),
        title: event.title.clone(),
        index: event.index,
        status: Some("running".to_string()),
        elapsed_time: None,
        error: None,
        outputs: Some(workflow_preview_outputs(&output.text, None, output.truncated)),
    }
}

#[allow(dead_code)]
fn workflow_node_tool_call_id(event: &WorkflowNodeStreamEvent) -> String {
    format!("workflow-node-{}-{}", event.index, event.node_id)
}

#[allow(dead_code)]
fn workflow_node_usage(outputs: Option<&serde_json::Value>) -> Option<serde_json::Value> {
    outputs.and_then(|value| value.get("usage")).cloned()
}

#[allow(dead_code)]
fn workflow_node_inline_yaml(workflow_yaml: &str) -> Option<&str> {
    (workflow_yaml.chars().count() <= WORKFLOW_NODE_INLINE_YAML_MAX_CHARS).then_some(workflow_yaml)
}

fn compact_workflow_text(text: &str) -> (String, bool) {
    let char_count = text.chars().count();
    if char_count <= WORKFLOW_NODE_PREVIEW_MAX_CHARS {
        return (text.to_string(), false);
    }

    let head_len = WORKFLOW_NODE_PREVIEW_MAX_CHARS * 2 / 3;
    let tail_len = WORKFLOW_NODE_PREVIEW_MAX_CHARS.saturating_sub(head_len);
    let head = text.chars().take(head_len).collect::<String>();
    let tail =
        text.chars().rev().take(tail_len).collect::<String>().chars().rev().collect::<String>();
    let omitted = char_count.saturating_sub(head_len + tail_len);
    (format!("{head}\n\n... 已截断 {omitted} 字符，仅保留预览 ...\n\n{tail}"), true)
}

#[allow(dead_code)]
fn compact_workflow_text_in_place(text: &mut String) -> bool {
    let (compacted, truncated) = compact_workflow_text(text);
    if truncated {
        *text = compacted;
    }
    truncated
}

#[allow(dead_code)]
fn workflow_output_preview_text(outputs: &serde_json::Value) -> String {
    for key in ["answer", "text", "result", "output", "data"] {
        if let Some(text) = outputs
            .get(key)
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            return text.to_string();
        }
    }
    serde_json::to_string_pretty(outputs)
        .or_else(|_| serde_json::to_string(outputs))
        .unwrap_or_else(|_| "节点执行完成".to_string())
}

fn workflow_preview_outputs(
    text: &str,
    usage: Option<serde_json::Value>,
    already_truncated: bool,
) -> serde_json::Value {
    let (preview, truncated) = compact_workflow_text(text);
    let mut outputs = serde_json::json!({
        "text": preview.clone(),
        "answer": preview.clone(),
        "result": preview,
        "truncated": already_truncated || truncated,
    });
    if let Some(usage) = usage {
        outputs["usage"] = usage;
    }
    outputs
}

#[allow(dead_code)]
fn workflow_node_preview_outputs(event: &WorkflowNodeStreamEvent) -> Option<serde_json::Value> {
    let outputs = event.outputs.as_ref().filter(|value| !value.is_null())?;
    Some(workflow_preview_outputs(
        &workflow_output_preview_text(outputs),
        workflow_node_usage(Some(outputs)),
        false,
    ))
}

#[allow(dead_code)]
fn workflow_node_tool_block(
    event: &WorkflowNodeStreamEvent,
    running: bool,
    workflow_yaml: &str,
) -> String {
    use serde_json::json;

    let status = if running {
        "running"
    } else if event.error.as_deref().is_some_and(|error| !error.trim().is_empty())
        || event.status.as_deref() == Some("failed")
    {
        "error"
    } else {
        "completed"
    };
    let title = event.title.trim();
    let node_type = event.node_type.trim();
    let summary = if node_type.is_empty() {
        title.to_string()
    } else if title.is_empty() {
        node_type.to_string()
    } else {
        format!("{title} · {node_type}")
    };
    let output_value = if let Some(outputs) = workflow_node_preview_outputs(event) {
        outputs
    } else if running {
        workflow_preview_outputs("正在执行工作流节点", None, false)
    } else {
        workflow_preview_outputs("节点执行完成", None, false)
    };
    let output =
        serde_json::to_string(&output_value).unwrap_or_else(|_| "节点执行完成".to_string());
    let payload = json!({
        "tool_call_id": workflow_node_tool_call_id(event),
        "status": status,
        "input": summary,
        "summary": summary,
        "output": output,
        "error": event.error,
        "metadata": {
            "canonical_tool_id": "workflow_node",
            "node_id": event.node_id,
            "node_type": event.node_type,
            "title": event.title,
            "index": event.index,
            "elapsed_time": event.elapsed_time,
            "usage": workflow_node_usage(event.outputs.as_ref()),
            "workflow_yaml": workflow_node_inline_yaml(workflow_yaml),
        }
    });
    let payload = serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string());
    format!("tool workflow_node\n{payload}\n")
}

fn gateway_stream_stop_after_forward_err(
    err: iced::futures::channel::mpsc::TrySendError<Message>,
    stop_reason: &mut Option<GatewayStreamStopReason>,
) -> bool {
    *stop_reason = Some(if err.is_disconnected() {
        GatewayStreamStopReason::Cancelled
    } else {
        GatewayStreamStopReason::Backpressure
    });
    false
}

fn plan_run_event_type(event: &vw_api_types::task::TaskPlanRunStreamEventDto) -> &'static str {
    match event {
        vw_api_types::task::TaskPlanRunStreamEventDto::PlanningStarted { .. } => "planning_started",
        vw_api_types::task::TaskPlanRunStreamEventDto::SessionDelta { .. } => "session_delta",
        vw_api_types::task::TaskPlanRunStreamEventDto::SessionWaiting { .. } => "session_waiting",
        vw_api_types::task::TaskPlanRunStreamEventDto::PlanCreated { .. } => "plan_created",
        vw_api_types::task::TaskPlanRunStreamEventDto::BatchStarted { .. } => "batch_started",
        vw_api_types::task::TaskPlanRunStreamEventDto::BatchFinished { .. } => "batch_finished",
        vw_api_types::task::TaskPlanRunStreamEventDto::GoalTurnStarted { .. } => {
            "goal_turn_started"
        }
        vw_api_types::task::TaskPlanRunStreamEventDto::GoalEvaluated { .. } => "goal_evaluated",
        vw_api_types::task::TaskPlanRunStreamEventDto::SubtaskStarted { .. } => "subtask_started",
        vw_api_types::task::TaskPlanRunStreamEventDto::SubtaskFinished { .. } => "subtask_finished",
        vw_api_types::task::TaskPlanRunStreamEventDto::Blocked { .. } => "blocked",
        vw_api_types::task::TaskPlanRunStreamEventDto::Finished { .. } => "finished",
        vw_api_types::task::TaskPlanRunStreamEventDto::Error { .. } => "error",
        vw_api_types::task::TaskPlanRunStreamEventDto::Cancelled { .. } => "cancelled",
    }
}

fn plan_run_event_task_id(event: &vw_api_types::task::TaskPlanRunStreamEventDto) -> &str {
    match event {
        vw_api_types::task::TaskPlanRunStreamEventDto::PlanningStarted { task_id, .. }
        | vw_api_types::task::TaskPlanRunStreamEventDto::SessionDelta { task_id, .. }
        | vw_api_types::task::TaskPlanRunStreamEventDto::SessionWaiting { task_id, .. }
        | vw_api_types::task::TaskPlanRunStreamEventDto::PlanCreated { task_id, .. }
        | vw_api_types::task::TaskPlanRunStreamEventDto::BatchStarted { task_id, .. }
        | vw_api_types::task::TaskPlanRunStreamEventDto::BatchFinished { task_id, .. }
        | vw_api_types::task::TaskPlanRunStreamEventDto::GoalTurnStarted { task_id, .. }
        | vw_api_types::task::TaskPlanRunStreamEventDto::GoalEvaluated { task_id, .. }
        | vw_api_types::task::TaskPlanRunStreamEventDto::SubtaskStarted { task_id, .. }
        | vw_api_types::task::TaskPlanRunStreamEventDto::SubtaskFinished { task_id, .. }
        | vw_api_types::task::TaskPlanRunStreamEventDto::Blocked { task_id, .. }
        | vw_api_types::task::TaskPlanRunStreamEventDto::Finished { task_id, .. }
        | vw_api_types::task::TaskPlanRunStreamEventDto::Cancelled { task_id, .. } => task_id,
        vw_api_types::task::TaskPlanRunStreamEventDto::Error { task_id, .. } => {
            task_id.as_deref().unwrap_or("unknown")
        }
    }
}

fn plan_run_event_session_id(
    event: &vw_api_types::task::TaskPlanRunStreamEventDto,
) -> Option<&str> {
    match event {
        vw_api_types::task::TaskPlanRunStreamEventDto::PlanningStarted { session_id, .. }
        | vw_api_types::task::TaskPlanRunStreamEventDto::SessionDelta { session_id, .. }
        | vw_api_types::task::TaskPlanRunStreamEventDto::SessionWaiting { session_id, .. }
        | vw_api_types::task::TaskPlanRunStreamEventDto::PlanCreated { session_id, .. }
        | vw_api_types::task::TaskPlanRunStreamEventDto::GoalTurnStarted { session_id, .. }
        | vw_api_types::task::TaskPlanRunStreamEventDto::GoalEvaluated { session_id, .. }
        | vw_api_types::task::TaskPlanRunStreamEventDto::SubtaskStarted { session_id, .. }
        | vw_api_types::task::TaskPlanRunStreamEventDto::SubtaskFinished { session_id, .. }
        | vw_api_types::task::TaskPlanRunStreamEventDto::Blocked { session_id, .. } => {
            Some(session_id.as_str())
        }
        vw_api_types::task::TaskPlanRunStreamEventDto::Finished { session_id, .. }
        | vw_api_types::task::TaskPlanRunStreamEventDto::Cancelled { session_id, .. } => {
            session_id.as_deref()
        }
        vw_api_types::task::TaskPlanRunStreamEventDto::BatchStarted { .. }
        | vw_api_types::task::TaskPlanRunStreamEventDto::BatchFinished { .. }
        | vw_api_types::task::TaskPlanRunStreamEventDto::Error { .. } => None,
    }
}

fn plan_run_event_session_title(
    event: &vw_api_types::task::TaskPlanRunStreamEventDto,
) -> Option<String> {
    match event {
        vw_api_types::task::TaskPlanRunStreamEventDto::PlanningStarted { task_id, .. }
        | vw_api_types::task::TaskPlanRunStreamEventDto::PlanCreated { task_id, .. } => {
            Some(format!("规划 · {task_id}"))
        }
        vw_api_types::task::TaskPlanRunStreamEventDto::SubtaskStarted {
            subtask_id, title, ..
        }
        | vw_api_types::task::TaskPlanRunStreamEventDto::SubtaskFinished {
            subtask_id,
            title,
            ..
        } => {
            let title = title.trim();
            Some(if title.is_empty() {
                format!("子任务 · {subtask_id}")
            } else {
                title.to_string()
            })
        }
        vw_api_types::task::TaskPlanRunStreamEventDto::GoalTurnStarted { turn, .. } => {
            Some(format!("目标续轮 · 第 {turn} 轮"))
        }
        vw_api_types::task::TaskPlanRunStreamEventDto::GoalEvaluated { turn, .. } => {
            Some(format!("目标验收 · 第 {turn} 轮"))
        }
        _ => None,
    }
}

fn plan_run_session_started_message(
    event: &vw_api_types::task::TaskPlanRunStreamEventDto,
    parent_request_id: u64,
    parent_session_id: &str,
    directory: &str,
    query: &str,
) -> Option<Message> {
    let session_title = plan_run_event_session_title(event)?;
    let (session_id, initial_query) = match event {
        vw_api_types::task::TaskPlanRunStreamEventDto::PlanningStarted { session_id, .. } => {
            (session_id, query.to_string())
        }
        vw_api_types::task::TaskPlanRunStreamEventDto::SubtaskStarted { session_id, .. } => {
            (session_id, session_title.clone())
        }
        vw_api_types::task::TaskPlanRunStreamEventDto::GoalTurnStarted {
            session_id,
            reason,
            ..
        }
        | vw_api_types::task::TaskPlanRunStreamEventDto::GoalEvaluated {
            session_id, reason, ..
        } => (session_id, reason.clone()),
        _ => return None,
    };
    Some(Message::Chat(message::ChatMessage::PlanRunSessionStarted {
        parent_request_id,
        parent_session_id: parent_session_id.to_string(),
        session_id: session_id.clone(),
        title: session_title,
        directory: directory.to_string(),
        query: initial_query,
    }))
}

fn plan_run_session_finished_id(
    event: &vw_api_types::task::TaskPlanRunStreamEventDto,
) -> Option<&str> {
    match event {
        vw_api_types::task::TaskPlanRunStreamEventDto::PlanCreated { session_id, .. }
        | vw_api_types::task::TaskPlanRunStreamEventDto::GoalEvaluated { session_id, .. }
        | vw_api_types::task::TaskPlanRunStreamEventDto::SubtaskFinished { session_id, .. }
        | vw_api_types::task::TaskPlanRunStreamEventDto::Blocked { session_id, .. } => {
            Some(session_id)
        }
        vw_api_types::task::TaskPlanRunStreamEventDto::Finished { session_id, .. } => {
            session_id.as_deref()
        }
        _ => None,
    }
}

fn plan_run_session_cancelled_id(
    event: &vw_api_types::task::TaskPlanRunStreamEventDto,
) -> Option<&str> {
    match event {
        vw_api_types::task::TaskPlanRunStreamEventDto::Cancelled { session_id, .. } => {
            session_id.as_deref()
        }
        _ => None,
    }
}

fn plan_run_tool_call_id(event: &vw_api_types::task::TaskPlanRunStreamEventDto) -> String {
    match event {
        vw_api_types::task::TaskPlanRunStreamEventDto::PlanningStarted { task_id, .. }
        | vw_api_types::task::TaskPlanRunStreamEventDto::PlanCreated { task_id, .. } => {
            format!("plan-run-{task_id}-planning")
        }
        vw_api_types::task::TaskPlanRunStreamEventDto::SessionDelta {
            task_id, session_id, ..
        }
        | vw_api_types::task::TaskPlanRunStreamEventDto::SessionWaiting {
            task_id,
            session_id,
            ..
        } => format!("plan-run-{task_id}-session-{session_id}"),
        vw_api_types::task::TaskPlanRunStreamEventDto::BatchStarted { task_id, batch, .. }
        | vw_api_types::task::TaskPlanRunStreamEventDto::BatchFinished { task_id, batch, .. } => {
            format!("plan-run-{task_id}-batch-{batch}")
        }
        vw_api_types::task::TaskPlanRunStreamEventDto::GoalTurnStarted {
            task_id, turn, ..
        } => format!("plan-run-{task_id}-goal-turn-{turn}"),
        vw_api_types::task::TaskPlanRunStreamEventDto::GoalEvaluated { task_id, turn, .. } => {
            format!("plan-run-{task_id}-goal-evaluation-{turn}")
        }
        vw_api_types::task::TaskPlanRunStreamEventDto::SubtaskStarted {
            task_id,
            subtask_id,
            ..
        }
        | vw_api_types::task::TaskPlanRunStreamEventDto::SubtaskFinished {
            task_id,
            subtask_id,
            ..
        } => format!("plan-run-{task_id}-subtask-{subtask_id}"),
        vw_api_types::task::TaskPlanRunStreamEventDto::Blocked { task_id, subtask_id, .. } => {
            format!("plan-run-{}-blocked-{}", task_id, subtask_id.as_deref().unwrap_or("task"))
        }
        vw_api_types::task::TaskPlanRunStreamEventDto::Finished { task_id, .. } => {
            format!("plan-run-{task_id}-finished")
        }
        vw_api_types::task::TaskPlanRunStreamEventDto::Error { task_id, .. } => {
            format!("plan-run-{}-error", task_id.as_deref().unwrap_or("unknown"))
        }
        vw_api_types::task::TaskPlanRunStreamEventDto::Cancelled { task_id, .. } => {
            format!("plan-run-{task_id}-cancelled")
        }
    }
}

fn plan_run_event_status(event: &vw_api_types::task::TaskPlanRunStreamEventDto) -> &'static str {
    match event {
        vw_api_types::task::TaskPlanRunStreamEventDto::PlanningStarted { .. }
        | vw_api_types::task::TaskPlanRunStreamEventDto::SessionDelta { .. }
        | vw_api_types::task::TaskPlanRunStreamEventDto::BatchStarted { .. }
        | vw_api_types::task::TaskPlanRunStreamEventDto::GoalTurnStarted { .. }
        | vw_api_types::task::TaskPlanRunStreamEventDto::GoalEvaluated {
            achieved: false, ..
        }
        | vw_api_types::task::TaskPlanRunStreamEventDto::SubtaskStarted { .. } => "running",
        vw_api_types::task::TaskPlanRunStreamEventDto::SessionWaiting { .. } => "waiting",
        vw_api_types::task::TaskPlanRunStreamEventDto::SubtaskFinished {
            success: false, ..
        }
        | vw_api_types::task::TaskPlanRunStreamEventDto::BatchFinished { success: false, .. }
        | vw_api_types::task::TaskPlanRunStreamEventDto::Blocked { .. }
        | vw_api_types::task::TaskPlanRunStreamEventDto::Error { .. } => "error",
        vw_api_types::task::TaskPlanRunStreamEventDto::Cancelled { .. } => "cancelled",
        vw_api_types::task::TaskPlanRunStreamEventDto::PlanCreated { .. }
        | vw_api_types::task::TaskPlanRunStreamEventDto::BatchFinished { .. }
        | vw_api_types::task::TaskPlanRunStreamEventDto::SubtaskFinished { .. }
        | vw_api_types::task::TaskPlanRunStreamEventDto::GoalEvaluated { achieved: true, .. }
        | vw_api_types::task::TaskPlanRunStreamEventDto::Finished { .. } => "completed",
    }
}

fn plan_run_event_summary(event: &vw_api_types::task::TaskPlanRunStreamEventDto) -> String {
    match event {
        vw_api_types::task::TaskPlanRunStreamEventDto::PlanningStarted { .. } => {
            "规划任务".to_string()
        }
        vw_api_types::task::TaskPlanRunStreamEventDto::SessionDelta { .. } => {
            "规划会话".to_string()
        }
        vw_api_types::task::TaskPlanRunStreamEventDto::SessionWaiting { .. } => {
            "等待后代智能体".to_string()
        }
        vw_api_types::task::TaskPlanRunStreamEventDto::PlanCreated { subtask_count, .. } => {
            format!("计划已创建 · {subtask_count} 个子任务")
        }
        vw_api_types::task::TaskPlanRunStreamEventDto::BatchStarted { batch, .. } => {
            format!("批次 {batch}")
        }
        vw_api_types::task::TaskPlanRunStreamEventDto::BatchFinished { batch, .. } => {
            format!("批次 {batch}")
        }
        vw_api_types::task::TaskPlanRunStreamEventDto::GoalTurnStarted { turn, .. } => {
            format!("目标续轮 · 第 {turn} 轮")
        }
        vw_api_types::task::TaskPlanRunStreamEventDto::GoalEvaluated { turn, achieved, .. } => {
            let status = if *achieved { "通过" } else { "未通过" };
            format!("目标验收 · 第 {turn} 轮 · {status}")
        }
        vw_api_types::task::TaskPlanRunStreamEventDto::SubtaskStarted { batch, title, .. } => {
            format!("批次 {batch} · {title}")
        }
        vw_api_types::task::TaskPlanRunStreamEventDto::SubtaskFinished {
            batch,
            title,
            success,
            ..
        } => {
            let suffix = if *success { "完成" } else { "失败" };
            format!("批次 {batch} · {title} · {suffix}")
        }
        vw_api_types::task::TaskPlanRunStreamEventDto::Blocked { .. } => "执行受阻".to_string(),
        vw_api_types::task::TaskPlanRunStreamEventDto::Finished { .. } => {
            "计划执行完成".to_string()
        }
        vw_api_types::task::TaskPlanRunStreamEventDto::Error { .. } => "计划执行失败".to_string(),
        vw_api_types::task::TaskPlanRunStreamEventDto::Cancelled { .. } => {
            "计划执行已取消".to_string()
        }
    }
}

fn plan_run_event_output_text(event: &vw_api_types::task::TaskPlanRunStreamEventDto) -> String {
    match event {
        vw_api_types::task::TaskPlanRunStreamEventDto::PlanningStarted { plan_dir, .. } => {
            format!("正在规划任务，计划目录: {plan_dir}")
        }
        vw_api_types::task::TaskPlanRunStreamEventDto::SessionDelta { delta, .. } => delta.clone(),
        vw_api_types::task::TaskPlanRunStreamEventDto::SessionWaiting {
            active_descendant_session_ids,
            ..
        } => format!(
            "等待 {} 个后代智能体完成: {}",
            active_descendant_session_ids.len(),
            active_descendant_session_ids.join(", ")
        ),
        vw_api_types::task::TaskPlanRunStreamEventDto::PlanCreated {
            plan_dir,
            plan_md_path,
            tasks_json_path,
            subtask_count,
            ..
        } => format!(
            "计划已创建，共 {subtask_count} 个子任务。\nplan.md: {plan_md_path}\ntasks.json: {tasks_json_path}\n目录: {plan_dir}"
        ),
        vw_api_types::task::TaskPlanRunStreamEventDto::BatchStarted {
            batch, subtask_ids, ..
        } => format!("批次 {batch} 开始执行: {}", subtask_ids.join(", ")),
        vw_api_types::task::TaskPlanRunStreamEventDto::BatchFinished {
            batch,
            subtask_ids,
            success,
            ..
        } => {
            let status = if *success { "执行完毕" } else { "执行失败" };
            format!("批次 {batch} {status}: {}", subtask_ids.join(", "))
        }
        vw_api_types::task::TaskPlanRunStreamEventDto::GoalTurnStarted { turn, reason, .. } => {
            format!("目标第 {turn} 轮续行：{reason}")
        }
        vw_api_types::task::TaskPlanRunStreamEventDto::GoalEvaluated {
            turn,
            achieved,
            reason,
            ..
        } => {
            let status = if *achieved { "通过" } else { "未通过，继续执行" };
            format!("目标第 {turn} 轮验收{status}：{reason}")
        }
        vw_api_types::task::TaskPlanRunStreamEventDto::SubtaskStarted {
            task_file, title, ..
        } => format!("子任务开始: {title}\n任务文件: {task_file}"),
        vw_api_types::task::TaskPlanRunStreamEventDto::SubtaskFinished {
            success,
            output,
            error,
            task_file,
            ..
        } => {
            if *success {
                format!(
                    "子任务完成，任务文件已处理: {task_file}\n{}",
                    output.as_deref().unwrap_or_default()
                )
            } else {
                format!(
                    "子任务失败，任务文件保留: {task_file}\n{}",
                    error.as_deref().unwrap_or("执行失败")
                )
            }
        }
        vw_api_types::task::TaskPlanRunStreamEventDto::Blocked { reason, .. } => {
            format!("执行受阻: {reason}")
        }
        vw_api_types::task::TaskPlanRunStreamEventDto::Finished {
            status,
            output,
            plan_dir,
            ..
        } => format!("状态: {status}\n计划目录: {plan_dir}\n{output}"),
        vw_api_types::task::TaskPlanRunStreamEventDto::Error { message, .. } => {
            format!("计划执行失败: {message}")
        }
        vw_api_types::task::TaskPlanRunStreamEventDto::Cancelled { .. } => {
            "计划执行已取消".to_string()
        }
    }
}

fn plan_run_event_is_terminal(event: &vw_api_types::task::TaskPlanRunStreamEventDto) -> bool {
    matches!(
        event,
        vw_api_types::task::TaskPlanRunStreamEventDto::Finished { .. }
            | vw_api_types::task::TaskPlanRunStreamEventDto::Blocked { .. }
            | vw_api_types::task::TaskPlanRunStreamEventDto::Error { .. }
            | vw_api_types::task::TaskPlanRunStreamEventDto::Cancelled { .. }
    )
}

fn plan_run_tool_block(event: &vw_api_types::task::TaskPlanRunStreamEventDto) -> String {
    use serde_json::json;

    let summary = plan_run_event_summary(event);
    let output_value = workflow_preview_outputs(&plan_run_event_output_text(event), None, false);
    let output =
        serde_json::to_string(&output_value).unwrap_or_else(|_| "计划执行事件".to_string());
    let error = matches!(
        event,
        vw_api_types::task::TaskPlanRunStreamEventDto::SubtaskFinished { success: false, .. }
            | vw_api_types::task::TaskPlanRunStreamEventDto::BatchFinished { success: false, .. }
            | vw_api_types::task::TaskPlanRunStreamEventDto::Blocked { .. }
            | vw_api_types::task::TaskPlanRunStreamEventDto::Error { .. }
    )
    .then(|| plan_run_event_output_text(event));
    let mut metadata = json!({
        "canonical_tool_id": "workflow_node",
        "plan_run": true,
        "event_type": plan_run_event_type(event),
        "task_id": plan_run_event_task_id(event),
    });
    if let Some(session_id) = plan_run_event_session_id(event) {
        metadata["session_id"] = json!(session_id);
    }
    if let Some(title) = plan_run_event_session_title(event) {
        metadata["title"] = json!(title);
    }
    let payload = json!({
        "tool_call_id": plan_run_tool_call_id(event),
        "status": plan_run_event_status(event),
        "input": summary,
        "summary": summary,
        "output": output,
        "error": error,
        "metadata": metadata,
    });
    let payload = serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string());
    format!("tool workflow_node\n{payload}\n")
}

#[allow(dead_code)]
fn apply_workflow_auth(
    builder: reqwest::RequestBuilder,
    endpoint: &vw_gateway_client::GatewayEndpoint,
) -> reqwest::RequestBuilder {
    let Some(auth) = endpoint.auth.as_ref() else {
        return builder;
    };

    if let Some(skey) = auth.skey.as_deref().filter(|value| !value.trim().is_empty()) {
        builder.bearer_auth(skey)
    } else {
        builder
    }
}

#[allow(dead_code)]
fn take_next_workflow_stream_payload(buffer: &mut String) -> Option<String> {
    let separator = if let Some(idx) = buffer.find("\r\n\r\n") {
        Some((idx, 4))
    } else {
        buffer.find("\n\n").map(|idx| (idx, 2))
    }?;

    let (idx, sep_len) = separator;
    let frame = buffer[..idx].to_string();
    buffer.drain(..idx + sep_len);

    let payload = frame
        .lines()
        .filter_map(|line| line.trim_end_matches('\r').strip_prefix("data:"))
        .map(str::trim_start)
        .collect::<Vec<_>>()
        .join("\n");
    (!payload.is_empty()).then_some(payload)
}

#[allow(dead_code)]
fn parse_workflow_chat_messages_event(payload: serde_json::Value) -> WorkflowChatMessagesEvent {
    match payload.get("event").and_then(serde_json::Value::as_str).unwrap_or_default() {
        "message" => WorkflowChatMessagesEvent::Delta(
            payload
                .get("answer")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_string(),
        ),
        "message_end" => WorkflowChatMessagesEvent::Done {
            usage: payload.get("metadata").and_then(|metadata| metadata.get("usage")).cloned(),
            message_id: payload
                .get("message_id")
                .and_then(serde_json::Value::as_str)
                .map(ToOwned::to_owned),
        },
        "node_started" => {
            WorkflowChatMessagesEvent::NodeStarted(parse_workflow_node_event(&payload))
        }
        "text_chunk" => WorkflowChatMessagesEvent::Delta(
            payload
                .get("data")
                .and_then(|data| data.get("text"))
                .or_else(|| payload.get("answer"))
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_string(),
        ),
        "node_delta" => {
            WorkflowChatMessagesEvent::NodeDelta(parse_workflow_node_delta_event(&payload))
        }
        "node_finished" => {
            WorkflowChatMessagesEvent::NodeFinished(parse_workflow_node_event(&payload))
        }
        "workflow_finished" => match payload
            .get("data")
            .and_then(|data| data.get("status"))
            .and_then(serde_json::Value::as_str)
        {
            Some("failed") => WorkflowChatMessagesEvent::Error(
                payload
                    .get("data")
                    .and_then(|data| data.get("error"))
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("workflow failed")
                    .to_string(),
            ),
            _ => WorkflowChatMessagesEvent::Other,
        },
        "error" => WorkflowChatMessagesEvent::Error(
            payload
                .get("message")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("workflow stream failed")
                .to_string(),
        ),
        _ => WorkflowChatMessagesEvent::Other,
    }
}

#[allow(dead_code)]
async fn stream_workflow_chat_messages(
    endpoint: &vw_gateway_client::GatewayEndpoint,
    directory: Option<&str>,
    body: &serde_json::Value,
    mut on_event: impl FnMut(WorkflowChatMessagesEvent) -> bool,
) -> Result<(), String> {
    use iced::futures::StreamExt;
    use tracing::{debug, info};

    #[cfg(not(target_arch = "wasm32"))]
    let builder = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(WORKFLOW_STREAM_CONNECT_TIMEOUT_SECS))
        .timeout(std::time::Duration::from_secs(WORKFLOW_STREAM_REQUEST_TIMEOUT_SECS));
    #[cfg(target_arch = "wasm32")]
    let builder = reqwest::Client::builder();

    let client = builder.build().map_err(|error| error.to_string())?;
    let mut request =
        client.post(format!("{}{}", endpoint.base_url(), WORKFLOW_CHAT_MESSAGES_PATH)).json(body);
    if let Some(directory) = directory.filter(|value| !value.trim().is_empty()) {
        request = request.query(&[("directory", directory)]);
    }
    request = apply_workflow_auth(request, endpoint);

    let response = request.send().await.map_err(|error| error.to_string())?;
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("workflow chat messages failed: {} {}", status, body.trim()));
    }

    info!(
        target: "vw_desktop",
        endpoint = %endpoint.describe(),
        path = WORKFLOW_CHAT_MESSAGES_PATH,
        "desktop connected workflow chat messages stream"
    );

    let mut bytes_stream = response.bytes_stream();
    let mut buffer = String::new();
    while let Some(chunk) = bytes_stream.next().await {
        let chunk = chunk.map_err(|error| error.to_string())?;
        buffer.push_str(&String::from_utf8_lossy(&chunk));
        while let Some(payload) = take_next_workflow_stream_payload(&mut buffer) {
            let payload: serde_json::Value =
                serde_json::from_str(&payload).map_err(|error| error.to_string())?;
            let event = parse_workflow_chat_messages_event(payload);
            debug!(
                target: "vw_desktop",
                path = WORKFLOW_CHAT_MESSAGES_PATH,
                "desktop received workflow chat messages stream event"
            );
            if !on_event(event) {
                return Ok(());
            }
        }
    }

    Ok(())
}

/// 决定本轮是否进入 `task_plan_run`。
///
/// 门控只认工作流开关：单独开启目标不会进入严格 plan-run。
/// 网关 plan-run 固定目标模式，不再区分 Goal/Workflow 参数。
fn task_plan_run_active(workflow_mode_enabled: bool, resume_history_only: bool) -> bool {
    workflow_mode_enabled && !resume_history_only
}

/// 无工作流时，目标模式开启便由独立评估器决定是否续轮。
fn lightweight_goal_active(workflow_mode_enabled: bool, resume_history_only: bool) -> bool {
    !workflow_mode_enabled && !resume_history_only
}

fn lightweight_goal_enabled_for_request(req: &AgentRequest) -> bool {
    req.goal_mode_enabled
        && lightweight_goal_active(req.workflow_mode_enabled, req.resume_history_only)
}

/// 按工作流门控构造 plan-run 请求；未进入 plan-run 时返回 `Ok(None)`。
/// 缺项目根目录错误仅在工作流已开启（即会进入 plan-run）时触发。
fn build_task_plan_run_request(
    req: &AgentRequest,
) -> Result<Option<vw_api_types::task::TaskPlanRunRequest>, String> {
    if !task_plan_run_active(req.workflow_mode_enabled, req.resume_history_only) {
        return Ok(None);
    }
    let Some(root) = req.root.as_deref().filter(|value| !value.trim().is_empty()) else {
        return Err("启用工作流执行需要先选择项目根目录".to_string());
    };
    Ok(Some(vw_api_types::task::TaskPlanRunRequest {
        query: req.query.clone(),
        root: root.to_string(),
        model: req.model.clone(),
        agent: req.agent.clone(),
        acp_agent: req.acp_agent.clone(),
        parent_session_id: {
            let trimmed = req.session.trim();
            (!trimmed.is_empty()).then(|| trimmed.to_string())
        },
    }))
}

#[cfg(test)]
async fn stream_task_plan_run(
    endpoint: &vw_gateway_client::GatewayEndpoint,
    body: &vw_api_types::task::TaskPlanRunRequest,
    on_event: impl FnMut(vw_api_types::task::TaskPlanRunStreamEventDto) -> bool,
) -> Result<(), String> {
    tracing::info!(
        target: "vw_desktop",
        endpoint = %endpoint.describe(),
        path = TASK_PLAN_RUN_STREAM_PATH,
        "desktop connected task plan-run stream"
    );
    vw_gateway_client::GatewayClient::stream_task_plan_run(endpoint, body, on_event).await
}

async fn stream_task_plan_run_resumable(
    endpoint: &vw_gateway_client::GatewayEndpoint,
    body: &vw_api_types::task::TaskPlanRunRequest,
    resume: &vw_gateway_client::GatewayStreamResume,
    on_event: impl FnMut(
        vw_gateway_client::GatewayStreamEvent<vw_api_types::task::TaskPlanRunStreamEventDto>,
    ) -> bool,
) -> Result<vw_gateway_client::GatewayStreamAttemptResult, String> {
    tracing::info!(
        target: "vw_desktop",
        endpoint = %endpoint.describe(),
        path = TASK_PLAN_RUN_STREAM_PATH,
        "desktop connected resumable task plan-run stream"
    );
    vw_gateway_client::GatewayClient::stream_task_plan_run_resumable(
        endpoint, body, resume, on_event,
    )
    .await
}

fn gateway_request_options(
    req: &AgentRequest,
    subagent_system_prompt: Option<&str>,
) -> Option<serde_json::Value> {
    use serde_json::{Value, json};

    let mut options = serde_json::Map::new();
    if req.full_access_enabled || req.acp_test {
        options.insert("full_access".to_string(), json!(true));
    }
    if let Some(agent) = &req.agent {
        options.insert("agent".to_string(), json!(agent));
    }
    if let Some(allowed_tools) = &req.allowed_tools {
        options.insert("allowed_tools".to_string(), json!(allowed_tools));
    }
    if req.require_agent_tool {
        options.insert("require_agent_tool".to_string(), json!(true));
    }
    if req.acp_test {
        options.insert("acp_test".to_string(), json!(true));
        options.insert("acp_agent".to_string(), json!(req.acp_agent));
        if let Some(acp_allowed_tools) = &req.acp_allowed_tools {
            options.insert("acp_allowed_tools".to_string(), json!(acp_allowed_tools));
        }
        options.insert("acp_force_new_session".to_string(), json!(req.acp_force_new_session));
        options.insert("acp_history_strategy".to_string(), json!(req.acp_history_mode.as_str()));
        options.insert("acp_history_recent_count".to_string(), json!(req.acp_recent_count));
        options.insert("cwd".to_string(), json!(req.root));
        options.insert("acp_permission_mode".to_string(), json!("approve-all"));
    }
    if req.resume_history_only {
        options.insert("desktop_resume_history_only".to_string(), json!(true));
    }
    if let Some(system_prompt) = subagent_system_prompt {
        options.insert("chat_system_prompt".to_string(), json!(system_prompt));
    }
    if lightweight_goal_enabled_for_request(req) {
        // 目标模式由会话处理器依据 Todo 状态决定是否续轮。
        options.insert("goal_mode".to_string(), json!(true));
    }
    (!options.is_empty()).then_some(Value::Object(options))
}

fn gateway_agent_stream(req: AgentRequest) -> AgentBoxStream<Message> {
    use iced::futures::SinkExt;
    use iced::futures::StreamExt;
    use serde_json::{Value, json};
    use tracing::{debug, info};

    use crate::app::models;

    let s = iced::stream::channel(
        100,
        move |mut output: iced::futures::channel::mpsc::Sender<Message>| async move {
            let endpoint = crate::app::config::gateway_client_endpoint();

            let subagent_system_prompt = req
                .history
                .iter()
                .find(|message| {
                    matches!(message.role, models::ChatRole::System)
                        && message.content
                            == crate::app::message::chat::session::SUBAGENT_MODE_SYSTEM_PROMPT
                })
                .map(|message| message.content.clone());
            let mut messages = Vec::new();
            for message in &req.history {
                if subagent_system_prompt.as_deref().is_some_and(|prompt| {
                    matches!(message.role, models::ChatRole::System) && message.content == prompt
                }) {
                    continue;
                }
                messages.push(json!({
                    "role": match message.role {
                        models::ChatRole::User => "user",
                        models::ChatRole::Assistant => "assistant",
                        models::ChatRole::System => "system",
                        models::ChatRole::Tool => "tool",
                    },
                    "content": message.content,
                }));
            }
            if !req.resume_history_only && !req.query.trim().is_empty() {
                messages.push(json!({"role": "user", "content": req.query}));
            }

            let body = json!({
                "messages": messages,
                "model": req.model,
            });
            let options = gateway_request_options(&req, subagent_system_prompt.as_deref());
            // 仅工作流开启才进入 task_plan_run；目标模式由网关固定开启。
            let workflow_mode_active =
                task_plan_run_active(req.workflow_mode_enabled, req.resume_history_only);
            let plan_run_body = match build_task_plan_run_request(&req) {
                Ok(body) => body,
                Err(error) => {
                    let _ = output
                        .send(Message::Chat(message::ChatMessage::AgentStreamError(req.id, error)))
                        .await;
                    return;
                }
            };
            let chat_body = vw_gateway_client::GatewayChatStreamRequest {
                session_id: Some(req.session.clone().into()),
                messages: body
                    .get("messages")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default(),
                system: None,
                model: body
                    .get("model")
                    .cloned()
                    .and_then(|value| serde_json::from_value(value).ok()),
                reasoning_effort: req.acp_test.then_some(req.reasoning_effort).flatten(),
                agent: req.agent.clone(),
                allowed_tools: req.allowed_tools.clone(),
                acp_agent: req.acp_test.then(|| req.acp_agent.clone()).flatten(),
                acp_allowed_tools: if req.acp_test { req.acp_allowed_tools.clone() } else { None },
                options: options.clone(),
            };
            let mut stream_done;
            let mut last_plan_run_event: Option<&'static str> = None;
            let mut plan_run_session_ids = std::collections::HashSet::<String>::new();
            let mut plan_run_terminal_error: Option<String> = None;
            let mut plan_run_cancelled = false;
            let mut attempt = 0_u64;

            info!(
                target: "vw_desktop",
                request_id = req.id,
                session_id = %req.session,
                acp_test = req.acp_test,
                agent = ?req.agent,
                allowed_tools = ?req.allowed_tools,
                require_agent_tool = req.require_agent_tool,
                acp_agent = ?req.acp_agent,
                acp_allowed_tools = ?req.acp_allowed_tools,
                acp_force_new_session = req.acp_force_new_session,
                acp_history_mode = %req.acp_history_mode.as_str(),
                acp_recent_count = req.acp_recent_count,
                full_access_enabled = req.full_access_enabled,
                workflow_mode_enabled = req.workflow_mode_enabled,
                goal_mode_enabled = req.goal_mode_enabled,
                workflow_mode_active,
                model = ?req.model,
                has_root = req.root.is_some(),
                history_len = req.history.len(),
                query_len = req.query.len(),
                options = ?options,
                "starting gateway agent stream"
            );

            let mut reconnect = GatewayReconnectState::new(desktop_stream_request_id(&req));
            stream_done = false;

            loop {
                attempt = attempt.saturating_add(1);
                let resume = reconnect.resume();
                let mut saw_new_event = false;
                let mut last_forwarded_event_id = None;
                let mut stop_reason = None;

                let result = if let Some(plan_run_body) = plan_run_body.as_ref() {
                    let root = req.root.as_deref().expect("validated workflow root");
                    stream_task_plan_run_resumable(
                        &endpoint,
                        plan_run_body,
                        &resume,
                        |stream_event| {
                            let event_id = stream_event.id;
                            let event = stream_event.event;
                            if resume.last_event_id.is_none_or(|last| event_id > last) {
                                saw_new_event = true;
                            }
                            let done_event = matches!(
                                event,
                                vw_api_types::task::TaskPlanRunStreamEventDto::Finished { .. }
                            );
                            let cancelled_event = matches!(
                                event,
                                vw_api_types::task::TaskPlanRunStreamEventDto::Cancelled { .. }
                            );
                            let blocked_reason = match &event {
                                vw_api_types::task::TaskPlanRunStreamEventDto::Blocked {
                                    reason,
                                    ..
                                } => Some(reason.clone()),
                                _ => None,
                            };
                            let error_event = match &event {
                                vw_api_types::task::TaskPlanRunStreamEventDto::Error {
                                    message,
                                    ..
                                } => Some(message.clone()),
                                _ => None,
                            };
                            let event_type = plan_run_event_type(&event);
                            last_plan_run_event = Some(event_type);
                            info!(
                                target: "vw_desktop",
                                request_id = req.id,
                                event_id,
                                event_type,
                                "desktop received task plan-run event"
                            );

                            if let Some(started_message) = plan_run_session_started_message(
                                &event,
                                req.id,
                                &req.session,
                                root,
                                &req.query,
                            ) {
                                if let Err(err) = output.try_send(started_message) {
                                    return gateway_stream_stop_after_forward_err(
                                        err,
                                        &mut stop_reason,
                                    );
                                }
                                if let Some(session_id) = plan_run_event_session_id(&event) {
                                    plan_run_session_ids.insert(session_id.to_string());
                                }
                            }

                            if let vw_api_types::task::TaskPlanRunStreamEventDto::SessionDelta {
                                session_id,
                                delta,
                                ..
                            } = &event
                            {
                                match output.try_send(Message::Chat(
                                    message::ChatMessage::PlanRunSessionDelta {
                                        session_id: session_id.clone(),
                                        delta: delta.clone(),
                                    },
                                )) {
                                    Ok(()) => {
                                        last_forwarded_event_id = Some(event_id);
                                        return true;
                                    }
                                    Err(err) => {
                                        return gateway_stream_stop_after_forward_err(
                                            err,
                                            &mut stop_reason,
                                        );
                                    }
                                }
                            }

                            if let vw_api_types::task::TaskPlanRunStreamEventDto::SessionWaiting {
                                session_id,
                                ..
                            } = &event
                                && let Err(err) = output.try_send(Message::Chat(
                                    message::ChatMessage::PlanRunSessionWaiting(session_id.clone()),
                                ))
                            {
                                return gateway_stream_stop_after_forward_err(
                                    err,
                                    &mut stop_reason,
                                );
                            }

                            if let Some(session_id) =
                                plan_run_session_cancelled_id(&event).map(str::to_string)
                            {
                                match output.try_send(Message::Chat(
                                    message::ChatMessage::PlanRunSessionCancelled(
                                        session_id.clone(),
                                    ),
                                )) {
                                    Ok(()) => {
                                        plan_run_session_ids.remove(&session_id);
                                    }
                                    Err(err) => {
                                        return gateway_stream_stop_after_forward_err(
                                            err,
                                            &mut stop_reason,
                                        );
                                    }
                                }
                            }

                            if let Some(session_id) =
                                plan_run_session_finished_id(&event).map(str::to_string)
                            {
                                match output.try_send(Message::Chat(
                                    message::ChatMessage::PlanRunSessionFinished(
                                        session_id.clone(),
                                    ),
                                )) {
                                    Ok(()) => {
                                        plan_run_session_ids.remove(&session_id);
                                    }
                                    Err(err) => {
                                        return gateway_stream_stop_after_forward_err(
                                            err,
                                            &mut stop_reason,
                                        );
                                    }
                                }
                            }

                            if let Err(err) = output.try_send(Message::Chat(
                                message::ChatMessage::AgentWorkflowNodeUpdate(
                                    req.id,
                                    plan_run_tool_block(&event),
                                ),
                            )) {
                                return gateway_stream_stop_after_forward_err(
                                    err,
                                    &mut stop_reason,
                                );
                            }

                            last_forwarded_event_id = Some(event_id);
                            if done_event {
                                stream_done = true;
                            } else if cancelled_event {
                                stream_done = true;
                                plan_run_cancelled = true;
                            } else if let Some(error) = error_event {
                                stream_done = true;
                                plan_run_terminal_error = Some(error);
                            } else if let Some(reason) = blocked_reason {
                                stream_done = true;
                                plan_run_terminal_error = Some(if reason.trim().is_empty() {
                                    "工作流执行受阻".to_string()
                                } else {
                                    format!("工作流执行受阻：{reason}")
                                });
                            } else {
                                debug_assert!(!plan_run_event_is_terminal(&event));
                            }
                            true
                        },
                    )
                    .await
                } else {
                    vw_gateway_client::GatewayClient::stream_chat_resumable(
                        &endpoint,
                        req.root.as_deref(),
                        &chat_body,
                        &resume,
                        |stream_event| {
                            let event_id = stream_event.id;
                            let event = stream_event.event;
                            if resume.last_event_id.is_none_or(|last| event_id > last) {
                                saw_new_event = true;
                            }
                            let event_kind = match &event {
                                vw_gateway_client::GatewayChatStreamEvent::Delta(_) => "delta",
                                vw_gateway_client::GatewayChatStreamEvent::Done { .. } => "done",
                                vw_gateway_client::GatewayChatStreamEvent::Error(_) => "error",
                                vw_gateway_client::GatewayChatStreamEvent::Other(_) => "other",
                            };
                            let terminal_event = matches!(
                                event,
                                vw_gateway_client::GatewayChatStreamEvent::Done { .. }
                                    | vw_gateway_client::GatewayChatStreamEvent::Error(_)
                            );
                            let mut handoff = false;

                            let next_message = match event {
                                vw_gateway_client::GatewayChatStreamEvent::Delta(delta) => {
                                    tracing::debug!(
                                        target: "vw_desktop",
                                        request_id = req.id,
                                        event_id,
                                        delta_len = delta.len(),
                                        "desktop received gateway delta"
                                    );
                                    Some(Message::Chat(message::ChatMessage::AgentStreamDelta(
                                        req.id, delta,
                                    )))
                                }
                                vw_gateway_client::GatewayChatStreamEvent::Done {
                                    usage,
                                    message_id,
                                    parent_message_id,
                                    ..
                                } => {
                                    info!(
                                        target: "vw_desktop",
                                        request_id = req.id,
                                        event_id,
                                        has_usage = usage.is_some(),
                                        message_id = ?message_id,
                                        parent_message_id = ?parent_message_id,
                                        "desktop received gateway done"
                                    );
                                    Some(Message::Chat(message::ChatMessage::AgentStreamDone(
                                        req.id,
                                        parse_usage(usage.as_ref()),
                                        message_id,
                                        parent_message_id,
                                    )))
                                }
                                vw_gateway_client::GatewayChatStreamEvent::Error(error) => {
                                    info!(
                                        target: "vw_desktop",
                                        request_id = req.id,
                                        event_id,
                                        "desktop received gateway error"
                                    );
                                    Some(Message::Chat(message::ChatMessage::AgentStreamError(
                                        req.id, error,
                                    )))
                                }
                                vw_gateway_client::GatewayChatStreamEvent::Other(payload) => {
                                    match payload
                                        .get("type")
                                        .and_then(serde_json::Value::as_str)
                                        .unwrap_or_default()
                                    {
                                        "chat.context_loaded" => {
                                            let paths = payload
                                                .get("paths")
                                                .and_then(serde_json::Value::as_array)
                                                .into_iter()
                                                .flatten()
                                                .filter_map(serde_json::Value::as_str)
                                                .map(ToOwned::to_owned)
                                                .collect::<Vec<_>>();
                                            (!paths.is_empty()).then(|| {
                                                Message::Chat(
                                                    message::ChatMessage::AgentContextLoaded(
                                                        req.id, paths,
                                                    ),
                                                )
                                            })
                                        }
                                        "chat.step_start" => {
                                            let step_index = payload
                                                .get("step_index")
                                                .and_then(serde_json::Value::as_u64)
                                                .unwrap_or_default()
                                                as u32;
                                            let created_ms = payload
                                                .get("created_ms")
                                                .and_then(serde_json::Value::as_u64)
                                                .unwrap_or_default();
                                            let model = payload
                                                .get("model")
                                                .and_then(serde_json::Value::as_str)
                                                .map(ToOwned::to_owned);
                                            Some(Message::Chat(
                                                message::ChatMessage::AgentStepStart(
                                                    req.id,
                                                    req.session.clone(),
                                                    step_index,
                                                    created_ms,
                                                    model,
                                                ),
                                            ))
                                        }
                                        "chat.step_finish" => {
                                            let step_index = payload
                                                .get("step_index")
                                                .and_then(serde_json::Value::as_u64)
                                                .unwrap_or_default()
                                                as u32;
                                            let finished_ms = payload
                                                .get("finished_ms")
                                                .and_then(serde_json::Value::as_u64)
                                                .unwrap_or_default();
                                            let usage = parse_usage(payload.get("usage"));
                                            let finish_reason = payload
                                                .get("finish_reason")
                                                .and_then(serde_json::Value::as_str)
                                                .map(ToOwned::to_owned);
                                            let model = payload
                                                .get("model")
                                                .and_then(serde_json::Value::as_str)
                                                .map(ToOwned::to_owned);
                                            Some(Message::Chat(
                                                message::ChatMessage::AgentStepFinish(
                                                    req.id,
                                                    req.session.clone(),
                                                    step_index,
                                                    finished_ms,
                                                    usage,
                                                    finish_reason,
                                                    model,
                                                ),
                                            ))
                                        }
                                        "chat.post_tool_round" => {
                                            if !crate::app::state::take_pending_guide_handoff(
                                                req.id,
                                            ) {
                                                None
                                            } else {
                                                handoff = true;
                                                let step_index = payload
                                                    .get("step_index")
                                                    .and_then(serde_json::Value::as_u64)
                                                    .unwrap_or_default()
                                                    as u32;
                                                Some(Message::Chat(
                                                    message::ChatMessage::AgentPostToolRound(
                                                        req.id,
                                                        req.session.clone(),
                                                        step_index,
                                                    ),
                                                ))
                                            }
                                        }
                                        _ => None,
                                    }
                                }
                            };

                            let Some(next_message) = next_message else {
                                last_forwarded_event_id = Some(event_id);
                                return true;
                            };

                            match output.try_send(next_message) {
                                Ok(()) => {
                                    last_forwarded_event_id = Some(event_id);
                                    if terminal_event {
                                        stream_done = true;
                                    }
                                    if handoff {
                                        stop_reason =
                                            Some(GatewayStreamStopReason::PostToolRoundHandoff);
                                        false
                                    } else {
                                        true
                                    }
                                }
                                Err(err) => {
                                    debug!(
                                        target: "vw_desktop",
                                        request_id = req.id,
                                        event_id,
                                        event_kind,
                                        "desktop agent stream forward failed"
                                    );
                                    gateway_stream_stop_after_forward_err(err, &mut stop_reason)
                                }
                            }
                        },
                    )
                    .await
                };

                match reconnect.finish_attempt(
                    result,
                    saw_new_event,
                    last_forwarded_event_id,
                    stop_reason,
                ) {
                    GatewayStreamNext::Terminal => break,
                    GatewayStreamNext::Cancelled => {
                        debug!(
                            target: "vw_desktop",
                            request_id = req.id,
                            attempt,
                            last_event_id = reconnect.last_event_id,
                            last_plan_run_event,
                            "gateway agent stream cancelled by UI subscription drop"
                        );
                        return;
                    }
                    GatewayStreamNext::PostToolRoundHandoff => {
                        debug!(
                            target: "vw_desktop",
                            request_id = req.id,
                            attempt,
                            "gateway agent stream stopped after post-tool-round handoff"
                        );
                        return;
                    }
                    GatewayStreamNext::Failed(error) => {
                        for session_id in plan_run_session_ids.drain() {
                            let _ = output
                                .send(Message::Chat(message::ChatMessage::PlanRunSessionFinished(
                                    session_id,
                                )))
                                .await;
                        }
                        debug!(
                            target: "vw_desktop",
                            request_id = req.id,
                            attempt,
                            error = %error,
                            "gateway agent stream failed without retry"
                        );
                        let _ = output
                            .send(Message::Chat(message::ChatMessage::AgentStreamError(
                                req.id, error,
                            )))
                            .await;
                        return;
                    }
                    GatewayStreamNext::Retry(delay) => {
                        debug!(
                            target: "vw_desktop",
                            request_id = req.id,
                            stream_kind = if workflow_mode_active { "plan_run" } else { "chat" },
                            attempt,
                            delay_ms = delay.as_millis(),
                            last_event_id = reconnect.last_event_id,
                            last_plan_run_event,
                            "gateway agent stream retry scheduled"
                        );
                        gateway_stream_retry_sleep(delay).await;
                    }
                }
            }

            if workflow_mode_active && stream_done {
                for session_id in plan_run_session_ids.drain() {
                    let _ = output
                        .send(Message::Chat(if plan_run_cancelled {
                            message::ChatMessage::PlanRunSessionCancelled(session_id)
                        } else {
                            message::ChatMessage::PlanRunSessionFinished(session_id)
                        }))
                        .await;
                }
                let terminal_message = if plan_run_cancelled {
                    Message::Chat(message::ChatMessage::AgentStreamCancelled(req.id))
                } else {
                    match plan_run_terminal_error {
                        Some(error) => {
                            Message::Chat(message::ChatMessage::AgentStreamError(req.id, error))
                        }
                        None => Message::Chat(message::ChatMessage::AgentStreamDone(
                            req.id,
                            crate::app::models::TokenUsage::default(),
                            None,
                            None,
                        )),
                    }
                };
                let _ = output.send(terminal_message).await;
            }
        },
    );

    #[cfg(not(target_arch = "wasm32"))]
    return s.boxed();
    #[cfg(target_arch = "wasm32")]
    return s.boxed_local();
}

fn parse_usage(value: Option<&serde_json::Value>) -> crate::app::models::TokenUsage {
    let Some(value) = value else {
        return crate::app::models::TokenUsage::default();
    };
    if value.get("prompt_tokens").is_some()
        || value.get("completion_tokens").is_some()
        || value.get("total_tokens").is_some()
    {
        return crate::app::models::TokenUsage {
            input_tokens: value
                .get("prompt_tokens")
                .and_then(serde_json::Value::as_i64)
                .unwrap_or(0),
            output_tokens: value
                .get("completion_tokens")
                .and_then(serde_json::Value::as_i64)
                .unwrap_or(0),
            cached_tokens: 0,
            reasoning_tokens: 0,
        };
    }
    serde_json::from_value(value.clone()).unwrap_or_default()
}

#[cfg(test)]
#[path = "agent_stream_tests.rs"]
mod agent_stream_tests;
