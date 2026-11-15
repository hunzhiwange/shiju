use serde_json::json;
use vw_desktop::app::{
    App,
    message::{ChatMessage, chat},
};
fn form() -> question_types::McpForm {
    serde_json::from_value(json!({"server":"表单验收", "schema":{"type":"object","properties":{"age":{"type":"integer","minimum":18},"choice":{"type":"string","enum":["a","b"]}},"required":["age"]}})).unwrap()
}
fn request(id: &str) -> question_types::Request {
    serde_json::from_value(json!({"id":id,"sessionID":id,"mcp_form":form(),"questions":[{"header":"表单验收","question":"中文长内容".repeat(100),"options":[],"custom":true},{"header":"表单验收","question":"选择","options":[{"label":"a","description":"选项A"},{"label":"b","description":"选项B"}],"custom":true},{"header":"表单验收","question":"提交？","options":[{"label":"accept","description":"接受"},{"label":"decline","description":"拒绝"}],"custom":false}]})).unwrap()
}
#[test]
fn codex_plugin_compat_t25_renders_and_cancels_server_form() {
    let (mut app, _) = App::new();
    let _ = chat::update(&mut app, ChatMessage::QuestionListLoaded(Ok(vec![request("first")])));
    std::hint::black_box(app.view());
    let cancel = chat::update(&mut app, ChatMessage::QuestionReject);
    assert!(cancel.units() > 0);
    assert_eq!(chat::update(&mut app, ChatMessage::QuestionReject).units(), 0);
}
#[test]
fn codex_plugin_compat_t25_rejects_invalid_input() {
    let (mut app, _) = App::new();
    let _ = chat::update(&mut app, ChatMessage::QuestionListLoaded(Ok(vec![request("invalid")])));
    let _ = chat::update(&mut app, ChatMessage::QuestionSubmit);
    assert!(
        chat::update(&mut app, ChatMessage::QuestionReject).units() > 0,
        "invalid form must remain open for correction or cancellation"
    );
}
#[test]
fn codex_plugin_compat_t25_preserves_isolation() {
    let (mut app, _) = App::new();
    let _ = chat::update(&mut app, ChatMessage::QuestionListLoaded(Ok(vec![request("first")])));
    let _ = chat::update(&mut app, ChatMessage::QuestionCustomChanged(0, "secret".into()));
    let _ = chat::update(&mut app, ChatMessage::QuestionListLoaded(Ok(vec![])));
    assert_eq!(chat::update(&mut app, ChatMessage::QuestionSubmit).units(), 0);
    let second = request("second");
    assert_eq!(second.session_id, "second");
    assert_eq!(second.mcp_form.unwrap().server, "表单验收");
}
