use std::time::{SystemTime, UNIX_EPOCH};

use vw_gateway_client::vw_api_types::task::{
    TaskPlanRunMode, TaskPlanRunRequest, TaskPlanRunStatusResponse,
};
use vw_gateway_client::{
    GatewayClient, GatewayStreamAttemptResult, GatewayStreamResume,
};
use vw_shared::session::info::Info as SessionInfo;

use super::gateway::{GatewayUiRuntime, block_on_gateway};
use super::stream_adapter::{
    UiRuntimeEvent, UiRuntimeTerminalEvent, adapt_task_plan_run_event,
};
use crate::cli::tui_v2::model::{PlanRunMode, PromptSubmission};

const PLAN_RUN_RESUME_ATTEMPTS: usize = 3;

pub(crate) fn plan_run_request_for_submission(
    submission: &PromptSubmission,
) -> Result<Option<TaskPlanRunRequest>, String> {
    let mode = match submission.execution.plan_run_mode {
        PlanRunMode::Disabled => return Ok(None),
        PlanRunMode::Workflow => {
            if !workflow_input_requests_plan(submission.text.as_str()) {
                return Ok(None);
            }
            TaskPlanRunMode::Workflow
        }
        PlanRunMode::Goal => {
            if submission.text.trim().is_empty() {
                return Ok(None);
            }
            TaskPlanRunMode::Goal
        }
    };
    let root = submission
        .root
        .as_deref()
        .map(str::trim)
        .filter(|root| !root.is_empty())
        .ok_or_else(|| "计划运行需要有效的项目根目录".to_string())?;
    Ok(Some(TaskPlanRunRequest {
        query: submission.text.clone(),
        root: root.to_string(),
        model: submission.model.clone(),
        agent: submission.execution.delegate_agent.clone(),
        acp_agent: submission.execution.acp_backend.clone(),
        parent_session_id: submission.session_id.clone(),
        mode,
    }))
}

pub(crate) fn workflow_input_requests_plan(query: &str) -> bool {
    let query = query.trim();
    if query.is_empty() {
        return false;
    }

    let normalized = query.to_ascii_lowercase();
    const CHINESE_ACTION_PREFIXES: &[&str] = &[
        "实现", "修复", "修改", "新增", "添加", "删除", "移除", "重构", "创建", "生成",
        "编写", "开发", "规划", "替换", "迁移", "升级", "安装", "配置", "部署", "发布",
        "提交", "推送", "合并", "执行", "运行", "重试", "接入", "支持", "改成", "做一个",
        "加上",
    ];
    const CHINESE_REQUEST_PREFIXES: &[&str] = &["请", "帮我", "帮忙", "麻烦"];
    const ENGLISH_ACTION_PREFIXES: &[&str] = &[
        "implement ",
        "fix ",
        "change ",
        "update ",
        "add ",
        "remove ",
        "delete ",
        "refactor ",
        "create ",
        "generate ",
        "write ",
        "build ",
        "plan ",
        "replace ",
        "migrate ",
        "upgrade ",
        "install ",
        "configure ",
        "deploy ",
        "publish ",
        "commit ",
        "push ",
        "merge ",
        "run ",
        "execute ",
    ];

    let mut conversational_query = query;
    while let Some(remainder) = CHINESE_REQUEST_PREFIXES
        .iter()
        .find_map(|prefix| conversational_query.strip_prefix(prefix))
    {
        let trimmed = remainder.trim_start_matches(['：', ':', '，', ',', ' ']);
        if trimmed.len() == conversational_query.len() {
            break;
        }
        conversational_query = trimmed;
    }
    let english_action_query =
        normalized.strip_prefix("please ").map(str::trim_start).unwrap_or(&normalized);
    let starts_with_action =
        CHINESE_ACTION_PREFIXES.iter().any(|prefix| query.starts_with(prefix))
            || ENGLISH_ACTION_PREFIXES
                .iter()
                .any(|prefix| english_action_query.starts_with(prefix));
    let polite_action_request = conversational_query != query
        && CHINESE_ACTION_PREFIXES
            .iter()
            .any(|action| conversational_query.starts_with(action));
    let object_action_request = query.starts_with('把')
        && CHINESE_ACTION_PREFIXES.iter().any(|action| query.contains(action));
    if starts_with_action || polite_action_request || object_action_request {
        return true;
    }

    const CHINESE_QUESTION_PREFIXES: &[&str] = &[
        "什么", "为什么", "为何", "怎么", "如何", "是否", "能否", "能不能", "可否",
        "可以", "可不可以", "有没有", "哪里", "哪个", "谁", "多少", "几", "请问",
        "解释", "介绍", "分析", "总结", "对比", "看看",
    ];
    const ENGLISH_QUESTION_PREFIXES: &[&str] = &[
        "what ",
        "why ",
        "how ",
        "is ",
        "are ",
        "can ",
        "could ",
        "would ",
        "should ",
        "do ",
        "does ",
        "did ",
        "where ",
        "when ",
        "who ",
        "which ",
        "explain ",
        "describe ",
        "analyze ",
        "summarize ",
        "compare ",
    ];
    let looks_like_question = query.ends_with(['?', '？'])
        || CHINESE_QUESTION_PREFIXES.iter().any(|prefix| query.starts_with(prefix))
        || CHINESE_QUESTION_PREFIXES
            .iter()
            .any(|prefix| conversational_query.starts_with(prefix))
        || ENGLISH_QUESTION_PREFIXES.iter().any(|prefix| normalized.starts_with(prefix));
    if looks_like_question {
        return false;
    }

    ["需要", "必须"].iter().any(|marker| query.contains(marker))
        && CHINESE_ACTION_PREFIXES.iter().any(|action| query.contains(action))
}

impl GatewayUiRuntime {
    pub(crate) fn session_is_plan_run_child_blocking(
        &self,
        session_id: &str,
    ) -> Result<bool, String> {
        let directory = self.directory().to_str();
        block_on_gateway(
            self.client()
                .session_get::<SessionInfo>(session_id, directory),
        )
        .map(|info| info.parent_id.is_some() && info.is_plan_run())
    }

    pub(crate) fn stream_task_plan_run_blocking(
        &self,
        request: &TaskPlanRunRequest,
        mut on_event: impl FnMut(UiRuntimeEvent) -> bool,
    ) -> UiRuntimeTerminalEvent {
        let request_id = plan_run_request_id();
        let mut last_event_id = None;
        let mut terminal = None;

        for _ in 0..PLAN_RUN_RESUME_ATTEMPTS {
            let resume = GatewayStreamResume {
                request_id: request_id.clone(),
                last_event_id,
            };
            let mut next_event_id = last_event_id;
            let attempt = block_on_gateway(GatewayClient::stream_task_plan_run_resumable(
                self.endpoint(),
                request,
                &resume,
                |event| {
                    next_event_id = Some(event.id);
                    for runtime_event in adapt_task_plan_run_event(event.event) {
                        if let UiRuntimeEvent::Terminal(runtime_terminal) = &runtime_event {
                            terminal = Some(runtime_terminal.clone());
                        }
                        if !on_event(runtime_event) {
                            return false;
                        }
                    }
                    true
                },
            ));
            last_event_id = next_event_id;
            match attempt {
                Ok(GatewayStreamAttemptResult::Terminated) => {
                    return terminal.unwrap_or_else(|| {
                        UiRuntimeTerminalEvent::Error(
                            "plan-run stream terminated without terminal event".to_string(),
                        )
                    });
                }
                Ok(GatewayStreamAttemptResult::CallerStopped) => {
                    return terminal.unwrap_or(UiRuntimeTerminalEvent::Cancelled {
                        reason: Some("plan-run stream cancelled by TUI".to_string()),
                        usage: None,
                        message_id: None,
                        parent_message_id: None,
                    });
                }
                Ok(GatewayStreamAttemptResult::Incomplete) => {}
                Err(error) => return UiRuntimeTerminalEvent::Error(error),
            }
        }

        UiRuntimeTerminalEvent::Error(
            "plan-run stream ended before a terminal event after resume attempts".to_string(),
        )
    }

    pub(crate) fn task_plan_run_stop_blocking(
        &self,
        task_id: &str,
    ) -> Result<TaskPlanRunStatusResponse, String> {
        block_on_gateway(self.client().task_plan_run_stop(task_id))
    }
}

fn plan_run_request_id() -> String {
    let nanos =
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
    format!("tui-plan-run-{nanos}")
}
