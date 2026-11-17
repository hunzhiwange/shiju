
pub(super) const WORKFLOW_HISTORY_MAX_CHARS: usize = 12_000;

#[allow(dead_code)]
pub(super) fn workflow_role_label(role: session_ui_types::ChatRole) -> &'static str {
    match role {
        session_ui_types::ChatRole::User => "User",
        session_ui_types::ChatRole::Assistant => "Assistant",
        session_ui_types::ChatRole::System => "System",
        session_ui_types::ChatRole::Developer => "Developer",
        session_ui_types::ChatRole::Tool => "Tool",
    }
}

#[allow(dead_code)]
pub(super) fn workflow_history_context(history: &[session_ui_types::ChatMessage]) -> String {
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

pub(super) const TEMPORARY_WORKFLOW_MAX_TASKS: usize = 8;
pub(super) const TEMPORARY_WORKFLOW_TITLE_MAX_CHARS: usize = 48;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TemporaryWorkflowTopology {
    Serial,
    Parallel,
}

impl TemporaryWorkflowTopology {
    pub(super) fn key(self) -> &'static str {
        match self {
            Self::Serial => "dynamic",
            Self::Parallel => "dynamic-parallel",
        }
    }

    pub(super) fn description(self) -> &'static str {
        match self {
            Self::Serial => "demand-driven workflow",
            Self::Parallel => "demand-driven parallel workflow",
        }
    }
}

#[derive(Debug, Clone)]
pub(super) struct TemporaryWorkflowStep {
    pub(super) id: String,
    pub(super) title: String,
    pub(super) system_prompt: String,
    pub(super) user_prompt: String,
    pub(super) column: i64,
    pub(super) lane: i64,
}

#[derive(Debug, Clone)]
pub(super) struct TemporaryWorkflowEdge {
    pub(super) source: String,
    pub(super) target: String,
}

#[derive(Debug, Clone)]
pub(super) struct TemporaryWorkflowPlan {
    pub(super) steps: Vec<TemporaryWorkflowStep>,
    pub(super) edges: Vec<TemporaryWorkflowEdge>,
    pub(super) final_node_id: String,
    pub(super) topology: TemporaryWorkflowTopology,
}

pub(super) fn contains_any(value: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| value.contains(needle))
}

pub(super) fn strip_temporary_workflow_order_prefix(value: &str) -> &str {
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

pub(super) fn strip_temporary_workflow_request_prefix(mut value: &str) -> &str {
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

pub(super) fn normalize_temporary_workflow_task_title(value: &str) -> Option<String> {
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

pub(super) fn compact_temporary_workflow_task_title(value: &str) -> String {
    let mut chars = value.chars();
    let head = chars.by_ref().take(TEMPORARY_WORKFLOW_TITLE_MAX_CHARS).collect::<String>();
    if chars.next().is_some() { format!("{head}...") } else { head }
}

pub(super) fn split_temporary_workflow_segment(value: &str) -> Vec<String> {
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

pub(super) fn temporary_workflow_task_titles(query: &str) -> Vec<String> {
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

pub(super) fn temporary_workflow_topology(query: &str, task_count: usize) -> TemporaryWorkflowTopology {
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

pub(super) fn temporary_workflow_task_system_prompt(title: &str, is_final: bool) -> String {
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

pub(super) fn first_workflow_user_prompt(task_title: &str, is_final: bool) -> String {
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

pub(super) fn next_workflow_user_prompt(
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

pub(super) fn final_parallel_workflow_user_prompt(
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

pub(super) fn workflow_step(
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

pub(super) fn workflow_edge(source: impl Into<String>, target: impl Into<String>) -> TemporaryWorkflowEdge {
    TemporaryWorkflowEdge { source: source.into(), target: target.into() }
}

pub(super) fn temporary_workflow_task_id(index: usize) -> String {
    format!("task_{}", index + 1)
}

pub(super) fn temporary_workflow_parallel_summary_title() -> String {
    "汇总并交付最终结果".to_string()
}

pub(super) fn temporary_workflow_plan(query: &str) -> TemporaryWorkflowPlan {
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

pub(super) fn temporary_workflow_query_fingerprint(query: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    query.trim().hash(&mut hasher);
    format!("{:08x}", hasher.finish() as u32)
}

pub(super) fn temporary_workflow_node_position(column: i64, lane: i64) -> (i64, i64) {
    let x = -280 + (column * 380);
    let y = 150 + (lane * 180);
    (x, y)
}

pub(super) fn temporary_workflow_node(
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

pub(super) fn temporary_workflow_edge(
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

pub(super) fn temporary_workflow_start_data() -> serde_json::Value {
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

pub(super) fn temporary_workflow_llm_data(
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

pub(super) fn temporary_workflow_answer_data(final_node_id: &str) -> serde_json::Value {
    use serde_json::json;

    json!({
        "title": "交付最终结果",
        "type": "answer",
        "answer": format!("{{{{#{final_node_id}.answer#}}}}")
    })
}

pub(super) fn temporary_workflow_yaml(query: &str, model: Option<&str>) -> Result<String, String> {
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
pub(super) fn workflow_mode_yaml(query: &str, model: Option<&str>) -> Result<String, String> {
    temporary_workflow_yaml(query, model)
}

pub(super) const TASK_PLAN_RUN_STREAM_PATH: &str = "/v1/task-plan-run/stream";
#[allow(dead_code)]
pub(super) const WORKFLOW_CHAT_MESSAGES_PATH: &str = "/v1/workflow/applications/chat-messages";
#[allow(dead_code)]
pub(super) const WORKFLOW_STREAM_CONNECT_TIMEOUT_SECS: u64 = 30;
#[allow(dead_code)]
pub(super) const WORKFLOW_STREAM_REQUEST_TIMEOUT_SECS: u64 = 60 * 60 * 3;
pub(super) const WORKFLOW_NODE_PREVIEW_MAX_CHARS: usize = 12_000;
#[allow(dead_code)]
pub(super) const WORKFLOW_NODE_STREAM_FLUSH_CHARS: usize = 360;
#[allow(dead_code)]
pub(super) const WORKFLOW_NODE_STREAM_FLUSH_MS: u128 = 220;
#[allow(dead_code)]
pub(super) const WORKFLOW_NODE_INLINE_YAML_MAX_CHARS: usize = 16_000;

#[allow(dead_code)]
pub(super) enum WorkflowChatMessagesEvent {
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
pub(super) struct WorkflowNodeStreamEvent {
    pub(super) node_id: String,
    pub(super) node_type: String,
    pub(super) title: String,
    pub(super) index: u32,
    pub(super) status: Option<String>,
    pub(super) elapsed_time: Option<f64>,
    pub(super) error: Option<String>,
    pub(super) outputs: Option<serde_json::Value>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub(super) struct WorkflowNodeDeltaStreamEvent {
    pub(super) node_id: String,
    pub(super) node_type: String,
    pub(super) title: String,
    pub(super) index: u32,
    pub(super) text: String,
    pub(super) replace: bool,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub(super) struct WorkflowNodeOutputBuffer {
    pub(super) text: String,
    pub(super) pending_chars: usize,
    pub(super) last_flush: web_time::Instant,
    pub(super) truncated: bool,
}

impl WorkflowNodeOutputBuffer {
    #[allow(dead_code)]
    pub(super) fn new() -> Self {
        Self {
            text: String::new(),
            pending_chars: 0,
            last_flush: web_time::Instant::now(),
            truncated: false,
        }
    }

    #[allow(dead_code)]
    pub(super) fn push(&mut self, delta: &str, replace: bool) {
        if replace {
            self.text.clear();
            self.truncated = false;
        }
        self.text.push_str(delta);
        self.pending_chars = self.pending_chars.saturating_add(delta.chars().count());
        self.truncated |= compact_workflow_text_in_place(&mut self.text);
    }

    #[allow(dead_code)]
    pub(super) fn should_flush(&mut self, force: bool) -> bool {
        if force
            || self.pending_chars >= WORKFLOW_NODE_STREAM_FLUSH_CHARS
            || self.last_flush.elapsed().as_millis() >= WORKFLOW_NODE_STREAM_FLUSH_MS
        {
            self.pending_chars = 0;
            self.last_flush = web_time::Instant::now();
            return true;
        }
        false
    }
}

#[allow(dead_code)]
pub(super) fn parse_workflow_node_event(payload: &serde_json::Value) -> WorkflowNodeStreamEvent {
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
pub(super) fn parse_workflow_node_delta_event(payload: &serde_json::Value) -> WorkflowNodeDeltaStreamEvent {
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
pub(super) fn workflow_node_key(index: u32, node_id: &str) -> String {
    format!("{index}-{node_id}")
}

#[allow(dead_code)]
pub(super) fn workflow_node_stream_event_key(event: &WorkflowNodeStreamEvent) -> String {
    workflow_node_key(event.index, &event.node_id)
}

#[allow(dead_code)]
pub(super) fn workflow_node_delta_event_key(event: &WorkflowNodeDeltaStreamEvent) -> String {
    workflow_node_key(event.index, &event.node_id)
}

#[allow(dead_code)]
pub(super) fn workflow_node_stream_event_from_delta(
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
pub(super) fn workflow_node_tool_call_id(event: &WorkflowNodeStreamEvent) -> String {
    format!("workflow-node-{}-{}", event.index, event.node_id)
}

#[allow(dead_code)]
pub(super) fn workflow_node_usage(outputs: Option<&serde_json::Value>) -> Option<serde_json::Value> {
    outputs.and_then(|value| value.get("usage")).cloned()
}

#[allow(dead_code)]
pub(super) fn workflow_node_inline_yaml(workflow_yaml: &str) -> Option<&str> {
    (workflow_yaml.chars().count() <= WORKFLOW_NODE_INLINE_YAML_MAX_CHARS).then_some(workflow_yaml)
}

pub(super) fn compact_workflow_text(text: &str) -> (String, bool) {
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
pub(super) fn compact_workflow_text_in_place(text: &mut String) -> bool {
    let (compacted, truncated) = compact_workflow_text(text);
    if truncated {
        *text = compacted;
    }
    truncated
}

#[allow(dead_code)]
pub(super) fn workflow_output_preview_text(outputs: &serde_json::Value) -> String {
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

pub(super) fn workflow_preview_outputs(
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
pub(super) fn workflow_node_preview_outputs(event: &WorkflowNodeStreamEvent) -> Option<serde_json::Value> {
    let outputs = event.outputs.as_ref().filter(|value| !value.is_null())?;
    Some(workflow_preview_outputs(
        &workflow_output_preview_text(outputs),
        workflow_node_usage(Some(outputs)),
        false,
    ))
}

#[allow(dead_code)]
pub(super) fn workflow_node_tool_block(
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
