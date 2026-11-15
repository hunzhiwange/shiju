use super::*;
use tui_contracts::action::{V3UserInputOption, V3UserInputQuestion};

fn request() -> V3UserInputRequest {
    V3UserInputRequest::new(
        "input-1",
        vec![V3UserInputQuestion::new(
            "mode",
            "Mode",
            "Choose a mode",
            vec![
                V3UserInputOption::new("Safe", "Read-only work"),
                V3UserInputOption::new("Fast", "Allow broader work"),
            ],
        )],
    )
}

// 验证 `request_user_input_emits_data_without_reply_side_effect` 场景的触发条件与预期结果。
#[test]
fn request_user_input_emits_data_without_reply_side_effect() {
    let mut overlay = RequestUserInputOverlay::new(request());
    assert_eq!(
        overlay.handle_key_event(KeyEvent::from(KeyCode::Enter)),
        V3Action::AnswerUserInput {
            request_id: "input-1".into(),
            answers: vec![V3UserInputAnswer {
                question_id: "mode".into(),
                values: vec!["Safe".into()],
            }],
        }
    );
}

// 验证 `unanswered_confirmation_keeps_go_back_path` 场景的触发条件与预期结果。
#[test]
fn unanswered_confirmation_keeps_go_back_path() {
    let mut question = request();
    question.questions[0].options.clear();
    let mut overlay = RequestUserInputOverlay::new(question);
    assert_eq!(overlay.handle_key_event(KeyEvent::from(KeyCode::Enter)), V3Action::None);
    assert_eq!(overlay.handle_key_event(KeyEvent::from(KeyCode::Esc)), V3Action::None);
    assert!(!overlay.is_complete());
}
