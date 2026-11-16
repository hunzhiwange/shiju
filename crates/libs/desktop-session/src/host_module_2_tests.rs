use crate::message::chat::send_input::apply_inline_mode_command;
use crate::message::chat::session::InlineModeCommand;

#[test]
fn apply_inline_mode_command_toggles_task_mode_and_clears_runtime_editor() {
    let mut runtime = crate::session::agent::SessionRuntimeState::<()>::default();
    runtime.input_editor = iced::widget::text_editor::Content::with_text("draft");

    apply_inline_mode_command(&mut runtime, InlineModeCommand::TaskMode);

    assert!(runtime.task_mode_enabled);
    assert_eq!(runtime.input_editor.text(), "");

    apply_inline_mode_command(&mut runtime, InlineModeCommand::ChatMode);

    assert!(!runtime.task_mode_enabled);
}
