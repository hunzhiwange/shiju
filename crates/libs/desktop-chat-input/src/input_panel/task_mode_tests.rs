use super::task_mode::task_mode_reasoning_effort_options;
use super::task_mode::TaskModeReasoningEffortOption;
use vw_api_chat_identity::ReasoningEffortDto;
#[test]
fn task_mode_reasoning_effort_option_displays_every_shared_label() {
    let cases = [
        (None, "默认"),
        (Some(ReasoningEffortDto::None), "None"),
        (Some(ReasoningEffortDto::Low), "Low"),
        (Some(ReasoningEffortDto::Medium), "Medium"),
        (Some(ReasoningEffortDto::High), "High"),
        (Some(ReasoningEffortDto::Xhigh), "Xhigh"),
        (Some(ReasoningEffortDto::Max), "Max"),
    ];

    for (effort, expected) in cases {
        assert_eq!(
            TaskModeReasoningEffortOption(
                effort,
                super::reasoning_selector::reasoning_effort_label(effort)
            )
            .to_string(),
            expected
        );
    }
}
#[test]
fn task_mode_reasoning_effort_handles_missing_and_claude_agents() {
    assert_eq!(
        task_mode_reasoning_effort_options(&[], "auto", None),
        vec![None]
    );
    assert_eq!(
        task_mode_reasoning_effort_options(
            &[],
            "auto",
            Some(" CLAUDE-CODE ")
        ),
        vec![None, Some(ReasoningEffortDto::High), Some(ReasoningEffortDto::Max),]
    );
}
#[test]
fn task_mode_reasoning_effort_uses_shared_agent_fallback_options() {
    assert_eq!(
        task_mode_reasoning_effort_options(&[], "auto", Some("codex")),
        vec![
            None,
            Some(ReasoningEffortDto::None),
            Some(ReasoningEffortDto::Low),
            Some(ReasoningEffortDto::Medium),
            Some(ReasoningEffortDto::High),
            Some(ReasoningEffortDto::Xhigh),
        ]
    );
    assert_eq!(
        task_mode_reasoning_effort_options(&[], "auto", Some("unknown")),
        vec![None]
    );
}
#[test]
fn task_mode_reasoning_effort_accepts_codex_cli_alias() {
    assert_eq!(
        task_mode_reasoning_effort_options(&[], "auto", Some(" CoDeX CLI ")),
        vec![
            None,
            Some(ReasoningEffortDto::None),
            Some(ReasoningEffortDto::Low),
            Some(ReasoningEffortDto::Medium),
            Some(ReasoningEffortDto::High),
            Some(ReasoningEffortDto::Xhigh),
        ]
    );
}
