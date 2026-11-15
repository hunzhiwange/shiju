//! 任务详情推理强度选择器的测试，覆盖默认值和各档位交互。
//! 用例核对禁用及选中样式，并保证消息携带预期的强度值。
use super::*;
use crate::slice_view_fixture::Fixture;
use crate::message::TaskBoardMessage;
use vw_api_chat_identity::ReasoningEffortDto;

#[test]
fn reasoning_option_formats_automatic_and_explicit_efforts() {
    assert_eq!(
        TaskBoardReasoningEffortOption(None, config_ui::DesktopLocale::SimplifiedChinese)
            .to_string(),
        "默认"
    );
    assert_eq!(
        TaskBoardReasoningEffortOption(
            Some(ReasoningEffortDto::High),
            config_ui::DesktopLocale::SimplifiedChinese
        )
        .to_string(),
        "High"
    );
}

#[test]
fn selector_kind_reads_draft_and_bulk_agent_and_effort_sources() {
    let mut app = Fixture::new().0;
    app.task_board_draft.acp_agent = Some("draft-agent".to_string());
    app.task_board_draft.reasoning_effort = Some(ReasoningEffortDto::Medium);
    app.task_board_bulk_acp_agent = Some("bulk-agent".to_string());
    app.task_board_bulk_reasoning_effort = Some(ReasoningEffortDto::Low);

    assert_eq!(TaskBoardReasoningSelectorKind::Draft.agent(app.reasoning()), Some("draft-agent"));
    assert_eq!(TaskBoardReasoningSelectorKind::Edit.agent(app.reasoning()), Some("draft-agent"));
    assert_eq!(TaskBoardReasoningSelectorKind::Bulk.agent(app.reasoning()), Some("bulk-agent"));
    assert_eq!(
        TaskBoardReasoningSelectorKind::Edit.current(app.reasoning()),
        Some(ReasoningEffortDto::Medium)
    );
    assert_eq!(TaskBoardReasoningSelectorKind::Bulk.current(app.reasoning()), Some(ReasoningEffortDto::Low));
}

#[test]
fn selector_builders_cover_draft_edit_bulk_and_missing_agent_defaults() {
    let mut app = Fixture::new().0;

    assert!(build_reasoning_effort_selector(app.reasoning(), false).is_none());
    assert!(build_reasoning_effort_selector(app.reasoning(), true).is_none());
    assert!(build_bulk_reasoning_effort_selector(app.reasoning()).is_none());

    app.task_board_draft.acp_agent = Some("codex".to_string());
    app.task_board_bulk_acp_agent = Some("codex".to_string());
    assert!(build_reasoning_effort_selector(app.reasoning(), false).is_some());
    assert!(build_bulk_reasoning_effort_selector(app.reasoning()).is_some());
}


#[test]
fn selector_kind_reads_updated_effort_and_maps_default_selection() {
 let mut app = Fixture::new().0;
 app.task_board_draft.reasoning_effort = Some(ReasoningEffortDto::High);
 assert_eq!(TaskBoardReasoningSelectorKind::Draft.current(app.reasoning()), Some(ReasoningEffortDto::High));
 assert!(matches!(TaskBoardReasoningSelectorKind::Draft.select_message(None), TaskBoardMessage::UpdateDraftReasoningEffort(None)));
}
