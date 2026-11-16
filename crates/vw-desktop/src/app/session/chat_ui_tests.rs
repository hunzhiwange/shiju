// 宿主契约：保留缓存高度查询转发，以及历史替换时活动会话问题索引的清理接线；其余局部行为迁至 desktop-chat-view::desktop_split_04_tests。
//! 聊天界面派生状态测试，覆盖虚拟列表分块、可见窗口和消息工件解析。
//! 索引换算与缓存重建必须在空记录、局部分块及过期准备结果下保持一致。

fn message(role: super::ChatRole, content: &str) -> super::ChatMessage {
    super::ChatMessage {
        role,
        content: content.to_string(),
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
    }
}

#[test]
fn invalidate_chat_ui_state_clears_user_question_index_for_history_replacement() {
    let (mut app, _task) = super::App::new();
    app.chat = vec![message(super::ChatRole::User, "old question")];
    app.rebuild_active_session_message_meta();

    app.invalidate_chat_ui_state();

    assert!(app.active_session_view_state.user_question_markers.is_empty());
    assert_eq!(app.active_session_view_state.user_question_indexed_chat_len, 0);
}

#[test]
fn resolve_render_cache_height_requires_matching_message_content_hash() {
    let (mut app, _task) = super::App::new();
    app.chat = vec![message(super::ChatRole::Assistant, "cached")];
    let content_hash =
        crate::app::components::chat_panel::message_view::hash_chat_content("cached");
    app.chat_render_cache.insert(
        0,
        super::ChatRenderCacheEntry {
            content_hash,
            estimated_expanded_height: 240.0,
            ..super::ChatRenderCacheEntry::default()
        },
    );

    assert_eq!(super::resolve_render_cache_height(&app, 0), Some(240.0));

    app.chat[0].content = "changed".to_string();

    assert_eq!(super::resolve_render_cache_height(&app, 0), None);
    assert_eq!(super::resolve_render_cache_height(&app, 1), None);
}
