
// 宿主保留：语音草稿状态机与普通输入区宿主装配契约。
#[test]
fn voice_moon_ui_normal_composer_has_no_voice_panel() {
    let (app, _) = crate::app::App::new();
    assert!(crate::app::components::voice_chat::local_inline(&app).is_none());
}
