use desktop_session::message::chat::stream::questions::{self, QuestionState};
use serde_json::json;
#[derive(Default)]
struct FormState {
    id: Option<String>,
    request: Option<question_types::Request>,
    answers: Vec<Vec<String>>,
    custom: Vec<String>,
}
impl FormState {
    fn borrowed(&mut self) -> QuestionState<'_> {
        QuestionState {
            question_modal_request_id: &mut self.id,
            question_modal_request: &mut self.request,
            question_modal_answers: &mut self.answers,
            question_modal_custom: &mut self.custom,
        }
    }
}
fn form() -> question_types::McpForm {
    serde_json::from_value(json!({"server":"表单验收", "schema":{"type":"object","properties":{"age":{"type":"integer","minimum":18},"choice":{"type":"string","enum":["a","b"]}},"required":["age"]}})).unwrap()
}
fn request(id: &str) -> question_types::Request {
    serde_json::from_value(json!({"id":id,"sessionID":id,"mcp_form":form(),"questions":[{"header":"表单验收","question":"中文长内容".repeat(100),"options":[],"custom":true},{"header":"表单验收","question":"选择","options":[{"label":"a","description":"选项A"},{"label":"b","description":"选项B"}],"custom":true},{"header":"表单验收","question":"提交？","options":[{"label":"accept","description":"接受"},{"label":"decline","description":"拒绝"}],"custom":false}]})).unwrap()
}
#[test]
fn codex_plugin_compat_t25_renders_and_cancels_server_form() {
    let mut state = FormState::default();
    questions::apply_list_result(&mut state.borrowed(), Ok(vec![request("first")]));
    let view = crate::modals::question_modal::with_question_modal(
        state.request.as_ref(),
        &state.answers,
        &state.custom,
        config_ui::DesktopLocale::SimplifiedChinese,
        None,
        |event| event,
        iced::widget::text("会话").into(),
    );
    std::hint::black_box(view);
    assert!(questions::reject_request(&mut state.borrowed()).is_some());
    assert!(questions::reject_request(&mut state.borrowed()).is_none());
}
#[test]
fn codex_plugin_compat_t25_rejects_invalid_input() {
    let mut state = FormState::default();
    questions::apply_list_result(&mut state.borrowed(), Ok(vec![request("invalid")]));
    assert!(
        questions::prepare_reply(&mut state.borrowed(), |request, answers| {
            request
                .mcp_form
                .as_ref()
                .is_none_or(|form| desktop_web_runtime::mcp_form::valid(form, answers))
        })
        .is_err()
    );
    assert!(
        questions::reject_request(&mut state.borrowed()).is_some(),
        "invalid form must remain open for correction or cancellation"
    );
}
#[test]
fn codex_plugin_compat_t25_preserves_isolation() {
    let mut state = FormState::default();
    questions::apply_list_result(&mut state.borrowed(), Ok(vec![request("first")]));
    questions::handle_question_custom_changed(&mut state.borrowed(), 0, "secret".into());
    questions::apply_list_result(&mut state.borrowed(), Ok(vec![]));
    assert!(questions::prepare_reply(&mut state.borrowed(), |_, _| true).unwrap().is_none());
    let second = request("second");
    assert_eq!(second.session_id, "second");
    assert_eq!(second.mcp_form.unwrap().server, "表单验收");
}
