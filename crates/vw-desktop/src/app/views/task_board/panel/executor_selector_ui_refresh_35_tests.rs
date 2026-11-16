//! Cross-domain selector message routing.
use crate::app::Message;
use crate::app::message::TaskBoardMessage;
/// Domain model events still reach the three different host message routes.
#[test]
fn ui_refresh_35_model_search_groups_and_bulk_selection_host_messages() {
    use crate::app::message::{SettingsMessage, ViewMessage};
    use desktop_task_board::model_selector::SelectorMessage;
    let map = super::super::model_selector::map;
    assert!(matches!(map(SelectorMessage::ModelQueryChanged("fixturex".into())),
        Message::Settings(SettingsMessage::ModelQueryChanged(value)) if value == "fixturex"));
    assert!(matches!(map(SelectorMessage::UpdateBulkModelInput("fixture/saved-modelx".into())),
        Message::TaskBoard(TaskBoardMessage::UpdateBulkModelInput(value)) if value == "fixture/saved-modelx"));
    assert!(
        matches!(map(SelectorMessage::OpenModelDetail("fixture".into(), "saved-model".into())),
        Message::View(ViewMessage::OpenSystemSettingsModelDetail(provider, model)) if provider == "fixture" && model == "saved-model")
    );
    assert!(matches!(map(SelectorMessage::BulkModelSelected("fixture/saved-model".into())),
        Message::TaskBoard(TaskBoardMessage::BulkModelSelected(value)) if value == "fixture/saved-model"));
    assert!(matches!(
        map(SelectorMessage::CloseBulkModelPopover),
        Message::TaskBoard(TaskBoardMessage::CloseBulkModelPopover)
    ));
}
