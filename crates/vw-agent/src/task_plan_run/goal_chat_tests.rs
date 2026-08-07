//! 无工作流轻量目标续轮测试。

use super::{
    LightweightGoalAction, SessionRunRequest, SessionRunner, build_lightweight_goal_continuation_prompt,
    build_lightweight_goal_evaluation_prompt, decide_lightweight_goal_action,
    detect_explicit_lightweight_blocker, parse_goal_evaluation, run_lightweight_goal_chat,
};
use async_trait::async_trait;
use std::collections::VecDeque;
use std::path::Path;
use std::sync::{Arc, Mutex};
use vw_api_types::task::TaskPlanRunStreamEventDto;

#[derive(Clone)]
struct ScriptedLightweightRunner {
    responses: Arc<Mutex<VecDeque<Result<String, String>>>>,
    calls: Arc<Mutex<Vec<SessionRunRequest>>>,
}

impl ScriptedLightweightRunner {
    fn new(responses: Vec<Result<String, String>>) -> Self {
        Self {
            responses: Arc::new(Mutex::new(VecDeque::from(responses))),
            calls: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

#[async_trait]
impl SessionRunner for ScriptedLightweightRunner {
    async fn run_session(
        &self,
        request: SessionRunRequest,
        _delta_tx: Option<tokio::sync::mpsc::Sender<String>>,
    ) -> Result<String, String> {
        self.calls.lock().expect("calls").push(request);
        self.responses
            .lock()
            .expect("responses")
            .pop_front()
            .unwrap_or_else(|| Err("scripted lightweight runner: no more responses".to_string()))
    }
}

#[test]
fn build_lightweight_goal_evaluation_prompt_omits_task_file_gate() {
    let prompt = build_lightweight_goal_evaluation_prompt(
        "修复登录超时并以验证证据证明完成",
        "已修改超时配置，但尚未运行测试",
    );

    assert!(prompt.contains("修复登录超时"));
    assert!(prompt.contains("已修改超时配置"));
    assert!(prompt.contains("achieved"));
    assert!(
        !prompt.contains("编号任务文件"),
        "lightweight evaluation must not gate on numbered task files"
    );
    assert!(
        !prompt.contains("plan.md"),
        "lightweight evaluation must not require plan.md"
    );
    assert!(
        !prompt.contains("所有编号任务文件均已删除"),
        "lightweight evaluation must omit task-file deletion gate"
    );
    assert!(prompt.contains("可验证证据"));

    let continuation = build_lightweight_goal_continuation_prompt(
        "修复登录超时并以验证证据证明完成",
        "缺少测试退出码证据",
    );
    assert!(continuation.contains("目标续轮") || continuation.contains("继续"));
    assert!(continuation.contains("缺少测试退出码证据"));
    assert!(!continuation.contains("编号任务文件"));
    assert!(!continuation.contains("plan.md"));
}

#[tokio::test]
async fn lightweight_goal_continues_until_evaluation_passes() {
    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path().to_string_lossy().to_string();
    let runner = ScriptedLightweightRunner::new(vec![
        Ok("已实现修改，但还没跑验证".to_string()),
        Ok(r#"{"achieved":false,"reason":"缺少验证退出码证据"}"#.to_string()),
        Ok("已重新运行验证，exit 0".to_string()),
        Ok(r#"{"achieved":true,"reason":"验证退出码为 0，目标达成"}"#.to_string()),
    ]);
    let events = Arc::new(Mutex::new(Vec::new()));
    let captured = Arc::clone(&events);

    run_lightweight_goal_chat(
        "chat-goal-1",
        "修复登录超时并以验证证据证明完成",
        &root,
        None,
        None,
        None,
        &runner,
        |event| {
            captured.lock().expect("events").push(event);
            true
        },
    )
    .await
    .expect("lightweight goal run");

    let events = events.lock().expect("events");
    let evaluations = events
        .iter()
        .filter_map(|event| match event {
            TaskPlanRunStreamEventDto::GoalEvaluated {
                turn,
                achieved,
                reason,
                ..
            } => Some((*turn, *achieved, reason.as_str())),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        evaluations,
        vec![
            (1, false, "缺少验证退出码证据"),
            (2, true, "验证退出码为 0，目标达成"),
        ]
    );
    assert!(events.iter().any(|event| matches!(
        event,
        TaskPlanRunStreamEventDto::GoalTurnStarted { turn: 2, reason, .. }
            if reason.contains("缺少验证退出码")
    )));
    assert!(matches!(
        events.last(),
        Some(TaskPlanRunStreamEventDto::Finished { status, plan_dir, .. })
            if status == "completed" && plan_dir.is_empty()
    ));
    assert_eq!(runner.calls.lock().expect("calls").len(), 4);
    assert!(
        !Path::new(&root).join(".plans").exists(),
        "lightweight goal must not create a strict workflow plan directory"
    );
}

#[tokio::test]
async fn lightweight_goal_stops_on_explicit_blocker() {
    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path().to_string_lossy().to_string();
    let runner = ScriptedLightweightRunner::new(vec![Ok(
        "无法继续。\n\n状态：已阻塞\n\n阻塞原因：缺少部署凭据，需要用户输入\n".to_string(),
    )]);
    let events = Arc::new(Mutex::new(Vec::new()));
    let captured = Arc::clone(&events);

    run_lightweight_goal_chat(
        "chat-goal-blocked",
        "完成外部部署",
        &root,
        None,
        None,
        None,
        &runner,
        |event| {
            captured.lock().expect("events").push(event);
            true
        },
    )
    .await
    .expect("blocked is represented by event");

    let events = events.lock().expect("events");
    assert!(matches!(
        events.last(),
        Some(TaskPlanRunStreamEventDto::Blocked { reason, .. })
            if reason.contains("缺少部署凭据")
    ));
    assert!(!events.iter().any(|event| {
        matches!(event, TaskPlanRunStreamEventDto::Finished { .. })
    }));
    assert_eq!(
        runner.calls.lock().expect("calls").len(),
        1,
        "explicit blocker must stop before evaluation/continuation"
    );

    let blocker = detect_explicit_lightweight_blocker(
        "状态：已阻塞\n阻塞原因：缺少部署凭据，需要用户输入",
    );
    assert!(blocker.expect("blocker").contains("缺少部署凭据"));

    let evaluation = parse_goal_evaluation(r#"{"achieved":false,"reason":"还差一步"}"#)
        .expect("evaluation");
    assert!(matches!(
        decide_lightweight_goal_action(&evaluation, Some("需要用户提供 API key")),
        LightweightGoalAction::Blocked { reason } if reason.contains("API key")
    ));
    assert!(matches!(
        decide_lightweight_goal_action(&evaluation, None),
        LightweightGoalAction::Continue { .. }
    ));
    let achieved = parse_goal_evaluation(r#"{"achieved":true,"reason":"完成"}"#).expect("ok");
    assert!(matches!(
        decide_lightweight_goal_action(&achieved, None),
        LightweightGoalAction::Complete { .. }
    ));
}
