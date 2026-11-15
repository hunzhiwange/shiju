//! Slash palette 查询与分组排序测试。

use super::slash_palette::collect_slash_palette_candidates;

#[test]
fn slash_palette_candidates_hide_native_tools_in_acp_sessions() {
    let mut app = crate::app::App::new().0;
    app.agents_settings.entries.clear();
    app.agents_settings.available_tools = vec!["read".into()];
    app.agents_settings.registered_tools = None;
    app.current_session_runtime_mut().acp_agent = None;
    assert!(!collect_slash_palette_candidates(&app).tools.is_empty());

    app.current_session_runtime_mut().acp_agent = Some("codex".into());
    assert!(collect_slash_palette_candidates(&app).tools.is_empty());
}
